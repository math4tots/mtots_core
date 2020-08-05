use crate::IndexSet;
use crate::RcStr;
use std::cmp;
use std::hash::Hash;
use std::hash::Hasher;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Key {
    Nil,
    Bool(bool),
    NumberBits(u64), // f64 stored as bits
    String(RcStr),
    List(Vec<Key>),
    Set(HSet),
}

/// Basically a IndexSet that has been made Ord and Hash
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HSet(pub IndexSet<Key>);

impl HSet {
    fn sorted_keys(&self) -> Vec<Key> {
        let mut keys: Vec<_> = self.0.clone().into_iter().collect();
        keys.sort();
        keys
    }
}

impl Hash for HSet {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.sorted_keys().hash(state);
    }
}

impl cmp::PartialOrd for HSet {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        self.sorted_keys().partial_cmp(&other.sorted_keys())
    }
}

impl cmp::Ord for HSet {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.sorted_keys().cmp(&other.sorted_keys())
    }
}
