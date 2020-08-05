//! Hash map implementation
//! with some features to make it more suitable for use
//! in a scripting language.
//!
//! Some features missing from std::collections::HashMap:
//!     * maintains insertion order
//!     * mutable state is made available to hash and eq
//!     * custom hash and equality functions are failable
//!
//! Based on:
//! https://hg.python.org/cpython/file/52f68c95e025/Objects/dictobject.c
//! https://stackoverflow.com/questions/327311
//!
//! GMap is the most generic version, that will require `State`, `FailableEq`,
//! `FailableHash` and `Error` generic parameters.
//!
//! HMap is a specialization of GMap that only require Key and Value generic
//! parameters
//!
use std::fmt;
use std::iter;
use std::mem;

pub trait FailableEq<S, T, E> {
    fn eq(state: &mut S, a: &T, b: &T) -> Result<bool, E>;
}

pub trait FailableHash<S, T, E> {
    fn hash(state: &mut S, x: &T) -> Result<u64, E>;
}

pub struct GMap<S, K, V, EqF, HashF, E>
where
    EqF: FailableEq<S, K, E>,
    HashF: FailableHash<S, K, E>,
{
    count: usize,
    fill: usize, // count + #<lingering-deleted-entries>
    index_map: Vec<usize>,
    entries: Vec<Option<Entry<K, V>>>,
    change_count: usize,
    eqf_type: std::marker::PhantomData<*const EqF>,
    hashf_type: std::marker::PhantomData<*const HashF>,
    state_type: std::marker::PhantomData<*const S>,
    error_type: std::marker::PhantomData<*const E>,
}

#[derive(Clone)]
struct Entry<K, V> {
    hash: u64,
    pair: (K, V),
}

impl<K, V> Entry<K, V> {
    fn key(&self) -> &K {
        &self.pair.0
    }
    fn value(&self) -> &V {
        &self.pair.1
    }
    fn into_value(self) -> V {
        self.pair.1
    }
}

const NULL_INDEX: usize = std::usize::MAX;
const INIT_CAP: usize = 8;
const PERTURB_SHIFT: usize = 5;
const MIN_CAP: usize = INIT_CAP;

pub struct DefaultEqF<K>(std::marker::PhantomData<K>);

impl<K: PartialEq> FailableEq<(), K, ()> for DefaultEqF<K> {
    fn eq(_: &mut (), a: &K, b: &K) -> Result<bool, ()> {
        Ok(a.eq(b))
    }
}

pub struct DefaultHashF<K>(std::marker::PhantomData<K>);

impl<K: std::hash::Hash> FailableHash<(), K, ()> for DefaultHashF<K> {
    fn hash(_: &mut (), x: &K) -> Result<u64, ()> {
        use std::hash::Hasher;
        let mut s = std::collections::hash_map::DefaultHasher::new();
        x.hash(&mut s);
        Ok(s.finish())
    }
}

/// AlwaysFalseEqF is useful when we're inserting into the map
/// and we know that there are no duplicates. This allows us to
/// avoid actually calling the user-specified failable functions.
struct AlwaysFalseEqF<K>(std::marker::PhantomData<K>);

impl<S, K> FailableEq<S, K, ()> for AlwaysFalseEqF<K> {
    fn eq(_: &mut S, _: &K, _: &K) -> Result<bool, ()> {
        Ok(false)
    }
}

/// Version of HMap using Rust's builtin Eq and Hash
pub type HMap<K, V> = GMap<(), K, V, DefaultEqF<K>, DefaultHashF<K>, ()>;

impl<K, V, S, EqF, HashF, E> GMap<S, K, V, EqF, HashF, E>
where
    EqF: FailableEq<S, K, E>,
    HashF: FailableHash<S, K, E>,
{
    pub fn new() -> GMap<S, K, V, EqF, HashF, E> {
        GMap {
            count: 0,
            fill: 0,
            index_map: vec![NULL_INDEX; INIT_CAP],
            entries: Vec::new(),
            change_count: 0,
            eqf_type: std::marker::PhantomData,
            hashf_type: std::marker::PhantomData,
            state_type: std::marker::PhantomData,
            error_type: std::marker::PhantomData,
        }
    }

    /// Gets the index of a key for the map.
    /// Allows providing custom 'S', 'E' and 'EqF'
    ///     This is used when reallocating the capacity of the map;
    ///     all the hashes are precomputed and we know that there are
    ///     no duplicates, so there's no need to actually call any of
    ///     the user provided functions if we do it this way.
    fn index<IS, IE, IEqF>(&self, state: &mut IS, key: &K, hash: u64) -> Result<usize, IE>
    where
        IEqF: FailableEq<IS, K, IE>,
    {
        let mask = self.index_map.len() - 1;
        let mut index = (hash as usize) & mask;
        let mut freeslot: Option<usize> = None;
        let entry_index = self.index_map[index];

        if entry_index == NULL_INDEX {
            // The entry for first guessed index is completely empty
            // Simplest possible scenario
            return Ok(index);
        } else {
            if let Some(entry) = &self.entries[entry_index] {
                if entry.hash == hash && IEqF::eq(state, entry.key(), key)? {
                    // The entry at the first index contains
                    // matching key
                    return Ok(index);
                }
            } else {
                // Found deleted entry
                // If we can't find this key elsewhere, this is
                // what we want to return.
                // If we do find the key, we should return that instead.
                freeslot = Some(index);
            }
        }

        let mut perturb = hash as usize;
        loop {
            index = ((index << 2) + index + perturb + 1) & mask;
            let entry_index = self.index_map[index];

            if entry_index == NULL_INDEX {
                // Key not found -- check if we found a freeslot before
                // (via a deleted entry), otherwise, return the empty
                // slot we just found
                return Ok(freeslot.unwrap_or(index));
            }

            if let Some(entry) = &self.entries[entry_index] {
                if entry.hash == hash && IEqF::eq(state, entry.key(), key)? {
                    // Found matching key
                    return Ok(index);
                }
            } else {
                // Found deleted key
                if freeslot.is_none() {
                    freeslot = Some(index);
                }
            }

            perturb >>= PERTURB_SHIFT;
        }
    }

    fn insert_interal<IS, IE, IEqF>(
        &mut self,
        state: &mut IS,
        key: K,
        value: V,
        hash: u64,
    ) -> Result<Option<V>, IE>
    where
        IEqF: FailableEq<IS, K, IE>,
    {
        let index = self.index::<IS, IE, IEqF>(state, &key, hash)?;
        let mut entry = Some(Entry {
            hash,
            pair: (key, value),
        });
        let entry_index = self.index_map[index];
        let old_value = if entry_index == NULL_INDEX {
            let entry_index = self.entries.len();
            self.entries.push(entry);
            self.index_map[index] = entry_index;
            None
        } else {
            mem::swap(&mut self.entries[entry_index], &mut entry);
            entry.map(|entry| entry.into_value())
        };
        if old_value.is_none() {
            self.count += 1;
            self.fill += 1;
        }
        Ok(old_value)
    }

    fn resize_cap(&mut self, new_min_cap: usize) {
        let new_cap = {
            // we need to ensure new_cap is always a power of 2
            let mut new_cap = MIN_CAP;
            while new_cap <= new_min_cap {
                new_cap <<= 1;
            }
            new_cap
        };

        self.count = 0;
        self.fill = 0;
        self.index_map = vec![NULL_INDEX; new_cap];
        let entries = mem::replace(&mut self.entries, Vec::new());

        for entry in entries {
            if let Some(Entry {
                hash,
                pair: (key, value),
            }) = entry
            {
                self.insert_interal::<(), (), AlwaysFalseEqF<K>>(&mut (), key, value, hash)
                    .unwrap();
            }
        }
    }

    /// Returns the number of changes made to the map since it was
    /// created
    /// This can be used to detect whether a given map has been
    /// mutated over time
    pub fn change_count(&self) -> usize {
        self.change_count
    }

    pub fn s_insert(&mut self, state: &mut S, key: K, value: V) -> Result<Option<V>, E> {
        self.change_count += 1;
        let hash = HashF::hash(state, &key)?;
        let optv = self.insert_interal::<S, E, EqF>(state, key, value, hash)?;

        // If we added a key, consider resizing
        // If fill >= 2/3 capacity, adjust capacity
        if optv.is_none() && self.fill * 3 >= self.capacity() * 2 {
            self.resize_cap(if self.count > 5000 {
                self.count * 2
            } else {
                self.count * 4
            });
        }

        Ok(optv)
    }

    pub fn s_remove(&mut self, state: &mut S, key: &K) -> Result<Option<V>, E> {
        self.change_count += 1;
        let hash = HashF::hash(state, &key)?;
        let index = self.index::<S, E, EqF>(state, &key, hash)?;
        let entry_index = self.index_map[index];
        if entry_index == NULL_INDEX {
            Ok(None)
        } else {
            let entry = mem::replace(&mut self.entries[entry_index], None);
            match entry {
                Some(entry) => {
                    self.count -= 1;
                    Ok(Some(entry.into_value()))
                }
                None => Ok(None),
            }
        }
    }

    pub fn s_get(&self, state: &mut S, key: &K) -> Result<Option<&V>, E> {
        let hash = HashF::hash(state, &key)?;
        let index = self.index::<S, E, EqF>(state, &key, hash)?;
        let entry_index = self.index_map[index];
        if entry_index == NULL_INDEX {
            Ok(None)
        } else {
            let entry = &self.entries[entry_index];
            match entry {
                Some(entry) => Ok(Some(entry.value())),
                None => Ok(None),
            }
        }
    }

    fn capacity(&self) -> usize {
        self.index_map.len()
    }

    pub fn len(&self) -> usize {
        self.count
    }

    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Should almost never need to be used
    /// See doc on method `get_pair_at_index`
    pub fn reserved_entries_count(&self) -> usize {
        self.entries.len()
    }

    /// This will retrieve the (key, value) pair at the given index
    /// This is a semi-private method in the sense that, you normally
    /// should never really need it
    /// However, this method, together with `reserved_entries_count`
    /// is needed for implementing Map iterators in mtots
    pub fn get_pair_at_index(&self, i: usize) -> Option<&(K, V)> {
        match &self.entries[i] {
            Some(entry) => Some(&entry.pair),
            None => None,
        }
    }

    pub fn s_eq<EqVF>(&self, state: &mut S, other: &Self) -> Result<bool, E>
    where
        EqVF: FailableEq<S, V, E>,
    {
        if self.len() != other.len() {
            return Ok(false);
        }
        for entry in &self.entries {
            if let Some(Entry {
                pair: (key, value), ..
            }) = entry
            {
                if let Some(oval) = other.s_get(state, key)? {
                    if !EqVF::eq(state, value, oval)? {
                        return Ok(false);
                    }
                } else {
                    return Ok(false);
                }
            }
        }
        Ok(true)
    }

    pub fn iter(&self) -> GMapRefIter<'_, K, V> {
        self.into_iter()
    }

    pub fn keys(&self) -> GMapRefKeysIter<'_, K, V> {
        GMapRefKeysIter {
            done: false,
            iter: self.entries.iter(),
        }
    }
}

/// Some convenience methods for the default case
impl<K, V> HMap<K, V>
where
    K: Eq + std::hash::Hash,
{
    pub fn get(&self, key: &K) -> Option<&V> {
        self.s_get(&mut (), key).unwrap()
    }

    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        self.s_insert(&mut (), key, value).unwrap()
    }

    pub fn remove(&mut self, key: &K) -> Option<V> {
        self.s_remove(&mut (), key).unwrap()
    }
}

impl<K, V> PartialEq for HMap<K, V>
where
    K: Eq + std::hash::Hash,
    V: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.s_eq::<DefaultEqF<V>>(&mut (), other).unwrap()
    }
}

impl<K, V> Eq for HMap<K, V>
where
    K: Eq + std::hash::Hash,
    V: Eq,
{
}

impl<K, V> iter::FromIterator<(K, V)> for HMap<K, V>
where
    K: Eq + std::hash::Hash,
{
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> HMap<K, V> {
        let iter = iter.into_iter();
        let mut map = HMap::new();
        for (k, v) in iter {
            map.insert(k, v);
        }
        map
    }
}

impl<K, V, S, EqF, HashF, E> Clone for GMap<S, K, V, EqF, HashF, E>
where
    EqF: FailableEq<S, K, E>,
    HashF: FailableHash<S, K, E>,
    K: Clone,
    V: Clone,
{
    fn clone(&self) -> Self {
        GMap {
            count: self.count,
            fill: self.fill,
            change_count: self.change_count,
            index_map: self.index_map.clone(),
            entries: self.entries.clone(),
            eqf_type: self.eqf_type.clone(),
            hashf_type: self.hashf_type.clone(),
            state_type: self.state_type.clone(),
            error_type: self.error_type.clone(),
        }
    }
}

impl<K, V, S, EqF, HashF, E> fmt::Debug for GMap<S, K, V, EqF, HashF, E>
where
    EqF: FailableEq<S, K, E>,
    HashF: FailableHash<S, K, E>,
    K: fmt::Debug,
    V: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "GMap{{")?;
        let mut first = true;
        for (k, v) in self {
            if !first {
                write!(f, ", ")?;
            }
            write!(f, "{:?}: {:?}", k, v)?;
            first = false;
        }
        write!(f, "}}")
    }
}

pub struct GMapIntoIter<K, V> {
    done: bool,
    iter: std::vec::IntoIter<Option<Entry<K, V>>>,
}

impl<K, V> Iterator for GMapIntoIter<K, V> {
    type Item = (K, V);
    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        loop {
            match self.iter.next() {
                Some(Some(entry)) => return Some(entry.pair),
                Some(None) => (),
                None => {
                    self.done = true;
                    return None;
                }
            }
        }
    }
}

impl<S, K, V, EqF, HashF, E> IntoIterator for GMap<S, K, V, EqF, HashF, E>
where
    EqF: FailableEq<S, K, E>,
    HashF: FailableHash<S, K, E>,
{
    type Item = (K, V);
    type IntoIter = GMapIntoIter<K, V>;
    fn into_iter(self) -> GMapIntoIter<K, V> {
        GMapIntoIter {
            done: false,
            iter: self.entries.into_iter(),
        }
    }
}

pub struct GMapRefIter<'a, K, V> {
    done: bool,
    iter: std::slice::Iter<'a, Option<Entry<K, V>>>,
}

impl<'a, K, V> Iterator for GMapRefIter<'a, K, V> {
    type Item = &'a (K, V);
    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        loop {
            match self.iter.next() {
                Some(Some(entry)) => return Some(&entry.pair),
                Some(None) => (),
                None => {
                    self.done = true;
                    return None;
                }
            }
        }
    }
}

impl<'a, S, K, V, EqF, HashF, E> IntoIterator for &'a GMap<S, K, V, EqF, HashF, E>
where
    EqF: FailableEq<S, K, E>,
    HashF: FailableHash<S, K, E>,
{
    type Item = &'a (K, V);
    type IntoIter = GMapRefIter<'a, K, V>;
    fn into_iter(self) -> GMapRefIter<'a, K, V> {
        GMapRefIter {
            done: false,
            iter: self.entries.iter(),
        }
    }
}

pub struct GMapRefKeysIter<'a, K, V> {
    done: bool,
    iter: std::slice::Iter<'a, Option<Entry<K, V>>>,
}

impl<'a, K, V> Iterator for GMapRefKeysIter<'a, K, V> {
    type Item = &'a K;
    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        loop {
            match self.iter.next() {
                Some(Some(entry)) => return Some(entry.key()),
                Some(None) => (),
                None => {
                    self.done = true;
                    return None;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_replace() {
        let mut map = HMap::<&'static str, &'static str>::new();
        assert_eq!(map.len(), 0);
        map.s_insert(&mut (), "Hello", "world").unwrap();
        assert_eq!(map.len(), 1);
        assert_eq!(map.s_get(&mut (), &"Hello"), Ok(Some(&"world")));
        assert_eq!(map.s_get(&mut (), &"hello"), Ok(None));
        map.s_insert(&mut (), "Hello", "foo").unwrap();
        assert_eq!(map.len(), 1);
        assert_eq!(map.s_get(&mut (), &"Hello"), Ok(Some(&"foo")));
    }

    #[test]
    fn test_many_insert() {
        let mut map = HMap::<i64, i64>::new();
        for i in 0..1000 {
            map.s_insert(&mut (), i, 2 * i).unwrap();
        }
        assert_eq!(map.len(), 1000);
        for i in 0..1000 {
            assert_eq!(map.s_get(&mut (), &i), Ok(Some(&(i * 2))),);
        }
    }

    #[test]
    fn test_remove() {
        let mut map = HMap::<i64, i64>::new();
        for i in 0..1000 {
            map.s_insert(&mut (), i, 2 * i).unwrap();
        }
        assert_eq!(map.len(), 1000);
        for i in 500..600 {
            assert!(map.s_remove(&mut (), &i).unwrap().is_some());
        }
        assert_eq!(map.len(), 900);
        for i in 0..500 {
            assert_eq!(map.s_get(&mut (), &i).unwrap(), Some(&(i * 2)));
        }
        for i in 500..600 {
            assert_eq!(map.s_get(&mut (), &i).unwrap(), None);
        }
        for i in 600..1000 {
            assert_eq!(map.s_get(&mut (), &i).unwrap(), Some(&(i * 2)));
        }
    }

    #[test]
    fn test_eq() {
        let mut map1 = HMap::<i64, i64>::new();
        for i in 0..1000 {
            map1.s_insert(&mut (), i, 2 * i).unwrap();
        }
        assert_eq!(map1.len(), 1000);
        let mut map2 = HMap::<i64, i64>::new();
        for i in (0..1000).rev() {
            map2.s_insert(&mut (), i, 2 * i).unwrap();
        }
        assert_eq!(map2.len(), 1000);
        assert!(map1.s_eq::<DefaultEqF<i64>>(&mut (), &map2).unwrap());
    }

    #[test]
    fn test_fill() {
        let mut map = HMap::<i64, i64>::new();
        for i in 0..8 {
            map.insert(i, 2 * i + 1);
            map.remove(&i);
        }
        assert_eq!(map.len(), 0);

        // If fill is not handled properly, the
        // some of the 'insert' calls in the loop below
        // may hang
        for i in 0..16 {
            map.insert(i, 2 * i);
        }
        assert_eq!(map.len(), 16);
    }

    #[test]
    fn test_iter() {
        let mut map = HMap::<i64, i64>::new();
        for i in (4..7).rev() {
            map.s_insert(&mut (), i, 2 * i).unwrap();
        }
        let lines: Vec<String> = map
            .iter()
            .map(|(k, v)| format!("(k, v) = ({}, {})", k, v))
            .collect();
        assert_eq!(
            lines,
            vec!["(k, v) = (6, 12)", "(k, v) = (5, 10)", "(k, v) = (4, 8)",],
        );

        let pairs: Vec<(i64, i64)> = map.into_iter().collect();
        assert_eq!(pairs, vec![(6, 12), (5, 10), (4, 8)],);

        let mut map = HMap::<i64, i64>::new();
        for i in 4..7 {
            map.s_insert(&mut (), i, 2 * i).unwrap();
        }
        let lines: Vec<String> = map
            .iter()
            .map(|(k, v)| format!("(k, v) = ({}, {})", k, v))
            .collect();
        assert_eq!(
            lines,
            vec!["(k, v) = (4, 8)", "(k, v) = (5, 10)", "(k, v) = (6, 12)",],
        );
    }
}
