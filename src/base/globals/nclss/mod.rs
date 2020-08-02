/// Builtin native classes
use crate::Class;
use crate::Globals;

use std::rc::Rc;

mod bewl;
mod bf;
mod bytes;
mod cell;
mod cls;
mod code;
mod ek;
mod exc;
mod flt;
mod func;
mod gobj;
mod hnd;
mod int;
mod iterable;
mod iterator;
mod list;
mod m;
mod map;
mod mbytes;
mod mlist;
mod mmap;
mod mset;
mod mstr;
mod nc;
mod ni;
mod nil;
mod obj;
mod opq;
mod path;
mod set;
mod strcls;
mod sym;
mod table;

// You might wonder, why not use macros here to keep from having
// to repeat yourself?
// In fact, all you'd need to supply are the sub-module names.
// The problem is that `cargo fmt` and `rls` don't work so well
// inside macros -- in particular, if the mod declarations are
// inside macros, cargo fmt doesn't even seem to realize that
// those modules get pulled in at all

#[allow(non_snake_case)]
pub struct BuiltinClasses {
    pub Object: Rc<Class>,
    pub Iterable: Rc<Class>,
    pub Iterator: Rc<Class>,
    pub Nil: Rc<Class>,
    pub Bool: Rc<Class>,
    pub Int: Rc<Class>,
    pub Float: Rc<Class>,
    pub Symbol: Rc<Class>,
    pub String: Rc<Class>,
    pub Bytes: Rc<Class>,
    pub Path: Rc<Class>,
    pub List: Rc<Class>,
    pub Table: Rc<Class>,
    pub Set: Rc<Class>,
    pub Map: Rc<Class>,
    pub Exception: Rc<Class>,
    pub NativeFunction: Rc<Class>,
    pub NativeClosure: Rc<Class>,
    pub Code: Rc<Class>,
    pub Function: Rc<Class>,
    pub Class: Rc<Class>,
    pub ExceptionKind: Rc<Class>,
    pub NativeIterator: Rc<Class>,
    pub GeneratorObject: Rc<Class>,
    pub Module: Rc<Class>,
    pub Opaque: Rc<Class>,
    pub Handle: Rc<Class>,
    pub MutableString: Rc<Class>,
    pub MutableBytes: Rc<Class>,
    pub MutableList: Rc<Class>,
    pub MutableSet: Rc<Class>,
    pub MutableMap: Rc<Class>,
    pub Cell: Rc<Class>,
}

impl BuiltinClasses {
    pub fn list(&self) -> Vec<&Rc<Class>> {
        vec![
            &self.Object,
            &self.Iterator,
            &self.Iterable,
            &self.Bool,
            &self.Int,
            &self.Float,
            &self.Symbol,
            &self.String,
            &self.Bytes,
            &self.Path,
            &self.List,
            &self.Table,
            &self.Set,
            &self.Map,
            &self.Exception,
            &self.Class,
            &self.Module,
            &self.MutableString,
            &self.MutableBytes,
            &self.MutableList,
            &self.MutableSet,
            &self.MutableMap,
            &self.Cell,
        ]
    }
}

impl Globals {
    #[allow(non_snake_case)]
    pub(super) fn new_builtin_classes() -> BuiltinClasses {
        let Object = obj::mkcls();
        let Iterable = iterable::mkcls(Object.clone());
        let Iterator = iterator::mkcls(Iterable.clone());
        let Nil = nil::mkcls(Object.clone());
        let Bool = bewl::mkcls(Object.clone());
        let Int = int::mkcls(Object.clone());
        let Float = flt::mkcls(Object.clone());
        let Symbol = sym::mkcls(Object.clone());
        let String = strcls::mkcls(Object.clone());
        let Bytes = bytes::mkcls(Iterable.clone());
        let Path = path::mkcls(Object.clone());
        let List = list::mkcls(Iterable.clone());
        let Table = table::mkcls(Object.clone());
        let Set = set::mkcls(Iterable.clone());
        let Map = map::mkcls(Iterable.clone());
        let Exception = exc::mkcls(Object.clone());
        let NativeFunction = bf::mkcls(Object.clone());
        let NativeClosure = nc::mkcls(Object.clone());
        let Code = code::mkcls(Object.clone());
        let Function = func::mkcls(Object.clone());
        let Class = cls::mkcls(Object.clone());
        let ExceptionKind = ek::mkcls(Object.clone());
        let NativeIterator = ni::mkcls(Iterator.clone());
        let GeneratorObject = gobj::mkcls(Iterator.clone());
        let Module = m::mkcls(Object.clone());
        let Opaque = opq::mkcls(Object.clone());
        let Handle = hnd::mkcls(Object.clone());
        let MutableString = mstr::mkcls(Object.clone());
        let MutableBytes = mbytes::mkcls(Object.clone());
        let MutableList = mlist::mkcls(Object.clone());
        let MutableSet = mset::mkcls(Object.clone());
        let MutableMap = mmap::mkcls(Object.clone());
        let Cell = cell::mkcls(Object.clone());
        BuiltinClasses {
            Object,
            Iterable,
            Iterator,
            Nil,
            Bool,
            Int,
            Float,
            Symbol,
            String,
            Bytes,
            Path,
            List,
            Table,
            Set,
            Map,
            Exception,
            NativeFunction,
            NativeClosure,
            Code,
            Function,
            Class,
            ExceptionKind,
            NativeIterator,
            GeneratorObject,
            Module,
            Opaque,
            Handle,
            MutableString,
            MutableBytes,
            MutableList,
            MutableSet,
            MutableMap,
            Cell,
        }
    }
}
