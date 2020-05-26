use crate::HMap;
use crate::RcStr;
use std::cell::Ref;
use std::cell::RefCell;
use std::cmp;
use std::collections::HashMap;
use std::fmt;
use std::hash;
use std::rc::Rc;

// Borrow<str> is not implemented for Symbol by design
// The contract of Borrow would requrie that Eq, Ord and Hash
// be equivalent as compared to str.
// This would mean that all of those operations would need to
// be against the &str value rather than against just the id
assert_not_impl!(Symbol, std::borrow::Borrow<str>);

#[derive(Debug)]
pub struct Symbol(&'static (usize, &'static str));

impl Symbol {
    pub fn id(&self) -> usize {
        (self.0).0
    }

    pub fn str(&self) -> &str {
        (self.0).1
    }
}

impl fmt::Display for Symbol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self.str(), f)
    }
}

impl cmp::PartialOrd for Symbol {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl cmp::Ord for Symbol {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        (self.0).1.cmp((other.0).1)
    }
}

impl cmp::PartialEq for Symbol {
    fn eq(&self, other: &Self) -> bool {
        self.id() == other.id()
    }
}

impl cmp::Eq for Symbol {}

impl hash::Hash for Symbol {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.id().hash(state)
    }
}

impl Clone for Symbol {
    fn clone(&self) -> Self {
        Symbol(self.0)
    }
}

impl Copy for Symbol {}

/// Due to the ubiquity of how symbol registries are used
/// (at least in this codebase), I'm just going to make it
/// easier on myself and always pass around the registry
/// by Rc<RefCell> (via SymbolRegistryHandle), rather
/// than allow the use of a SymbolRegistry directly outside
/// this module
#[derive(Clone)]
pub struct SymbolRegistryHandle(Rc<RefCell<SymbolRegistry>>);

impl SymbolRegistryHandle {
    pub fn new() -> SymbolRegistryHandle {
        SymbolRegistryHandle(Rc::new(RefCell::new(SymbolRegistry::new())))
    }

    pub fn intern_rcstr(&self, rcstr: &RcStr) -> Symbol {
        self.0.borrow_mut().intern_rcstr(rcstr)
    }

    pub fn intern_str(&self, s: &str) -> Symbol {
        self.0.borrow_mut().intern_str(s)
    }

    pub fn rcstr(&self, symbol: Symbol) -> Ref<RcStr> {
        Ref::map(self.0.borrow(), |sr| sr.rcstr(symbol))
    }

    pub fn translate_hmap<V, I: IntoIterator<Item = (RcStr, V)>>(&self, map: I) -> HMap<Symbol, V> {
        self.0.borrow_mut().translate_hmap(map)
    }

    pub fn translate_vec<T: Into<RcStr>>(&self, vec: Vec<T>) -> Vec<Symbol> {
        self.0.borrow_mut().translate_vec(vec)
    }
}

struct SymbolRegistry {
    map: HashMap<RcStr, Symbol>,
    id_to_rcstr: Vec<&'static RcStr>,
}

impl SymbolRegistry {
    fn new() -> SymbolRegistry {
        let mut registry = SymbolRegistry {
            map: HashMap::new(),
            id_to_rcstr: Vec::new(),
        };
        registry.preload_symbols();
        registry
    }

    /// allows interning strings without making copies of the string buffer
    fn intern_rcstr(&mut self, rcstr: &RcStr) -> Symbol {
        if !self.map.contains_key(rcstr) {
            assert_eq!(
                self.map.len(),
                self.id_to_rcstr.len(),
                "Inconsistent symbol registry"
            );
            let id = self.map.len();

            let leaked_rcstr: &'static RcStr = Box::leak(Box::new(rcstr.clone()));
            let leaked_str: &'static str = &leaked_rcstr;
            let leaked_symbol_data: &'static (usize, &'static str) =
                Box::leak(Box::new((id, leaked_str)));
            let symbol = Symbol(leaked_symbol_data);

            self.id_to_rcstr.push(leaked_rcstr);
            self.map.insert(leaked_rcstr.clone(), symbol);
        }

        *self.map.get(rcstr).unwrap()
    }

    fn intern_str(&mut self, s: &str) -> Symbol {
        self.intern_rcstr(&s.into())
    }

    fn rcstr(&self, symbol: Symbol) -> &RcStr {
        self.id_to_rcstr[symbol.id()]
    }

    fn translate_hmap<V, I: IntoIterator<Item = (RcStr, V)>>(&mut self, map: I) -> HMap<Symbol, V> {
        map.into_iter()
            .map(|(k, v)| (self.intern_rcstr(&k.into()), v))
            .collect()
    }

    fn translate_vec<T: Into<RcStr>>(&mut self, vec: Vec<T>) -> Vec<Symbol> {
        vec.into_iter()
            .map(|t| self.intern_rcstr(&t.into()))
            .collect()
    }
}

macro_rules! define_preloaded_symbols {
    (
        $( $name:ident $value:expr , )*
    ) => {

        #[allow(non_camel_case_types)]
        enum PreloadedSymbolsEnum {
            $( $name, )*
        }

        impl Symbol {
            $( pub const $name: Symbol =
                Symbol(&(PreloadedSymbolsEnum::$name as usize, $value)); )*

            pub const PRELOADED_SYMBOLS: &'static [Symbol] = &[
                $( Symbol::$name, )*
            ];
        }

        impl SymbolRegistry {
            fn preload_symbols(&mut self) {
                $(
                    assert_eq!(self.map.len(), self.id_to_rcstr.len());
                    assert_eq!(self.map.len(), PreloadedSymbolsEnum::$name as usize);
                    self.map.insert(Symbol::$name.str().into(), Symbol::$name);

                    let leaked_rcstr: &'static RcStr =
                        Box::leak(Box::new(Symbol::$name.str().into()));

                    self.id_to_rcstr.push(leaked_rcstr);
                )*
            }
        }
    };
}

define_preloaded_symbols! {
    SELF "self",
    ARGS "args",
    KWARGS "kwargs",
    DEF "def",
    LEN "len",
    FROM_ITERABLE "from_iterable",
    DUNDER_CALL "__call",

    // for os module
    INHERIT "inherit",
    PIPE "pipe",
    NULL "null",
    UTF8 "utf8",
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn symbol_size() {
        use std::mem::size_of;
        assert_eq!(size_of::<Symbol>(), size_of::<usize>());

        // if &str were ever to be the same size as &&str,
        // we probably want to use &str instead of &&str
        assert!(size_of::<Symbol>() < size_of::<&str>());
    }

    #[test]
    fn preloaded_symbols_identity() {
        // check that all preloaded symbols are the 'same' as
        // any symbol interned later with the same values

        fn hash<T: std::hash::Hash>(t: T) -> u64 {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::Hasher;
            let mut h = DefaultHasher::new();
            t.hash(&mut h);
            h.finish()
        }

        let mut reg = SymbolRegistry::new();

        for preloaded in Symbol::PRELOADED_SYMBOLS {
            let interned = reg.intern_str(preloaded.str());
            assert_eq!(interned, *preloaded);
            assert_eq!(hash(interned), hash(preloaded));
        }
    }

    #[test]
    fn eq() {
        let mut reg = SymbolRegistry::new();
        assert_eq!(reg.intern_str("len"), reg.intern_str("len"));
        assert_eq!(reg.intern_str("len"), Symbol::LEN);
        assert_eq!(reg.intern_str("hello"), reg.intern_str("hello"));
        assert!(reg.intern_str("hello") != reg.intern_str("hi"));
    }

    #[test]
    fn hash() {
        use std::collections::HashSet;
        let mut reg = SymbolRegistry::new();
        let mut set = HashSet::new();
        set.insert(reg.intern_str("foo"));
        set.insert(reg.intern_str("foo"));
        assert_eq!(set.len(), 1);
        set.insert(reg.intern_str("bar"));
        assert_eq!(set.len(), 2);
        set.insert(Symbol::LEN);
        assert_eq!(set.len(), 3);
        set.insert(reg.intern_str("len"));
        assert_eq!(set.len(), 3);
        assert!(set.contains(&Symbol::LEN));
        assert!(set.contains(&reg.intern_str("foo")));
    }
}
