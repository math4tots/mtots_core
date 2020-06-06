use crate::Class;
use crate::ClassKind;
use crate::Eval;
use crate::GeneratorResult;
use crate::NativeFunction;
use crate::NativeIterator;
use crate::SymbolRegistryHandle;
use crate::Value;

use std::collections::HashMap;
use std::rc::Rc;

// There are only two Iterator types, NativeIterator and GeneratorObject
// TODO: Enforce this in the runtime (e.g. something like Scala's sealed traits)

pub(super) fn mkcls(sr: &SymbolRegistryHandle, base: Rc<Class>) -> Rc<Class> {
    // While I'm hesitant to add many methods to Iterable because various
    // user classes may implement them, Iterator should have exactly two descendants
    // both of which are builtins. So I'm not really worried about method name conflicts
    // in this case

    let methods = vec![
        NativeFunction::simple0(sr, "list", &["self"], |globals, args, _kwargs| {
            Ok(Eval::list_from_iterable(globals, &args[0])?)
        }),
        // This really should be two different functions,
        //   - map() => for building a map from an iterable of pairs
        //   - map(f) => for getting a new iterator with f applied to each element
        // TODO: Consider whether this is evil
        NativeFunction::snew(
            sr,
            "map",
            (&["self"], &[("f", Value::Uninitialized)], None, None),
            |globals, args, _kwargs| {
                if let Value::Uninitialized = &args[1] {
                    Ok(Eval::map_from_iterable(globals, &args[0])?)
                } else {
                    let owner = args[0].clone();
                    let f = args[1].clone();
                    Ok(NativeIterator::new(move |globals, input_value| {
                        match Eval::resume(globals, &owner, input_value) {
                            GeneratorResult::Yield(iterator_yield_value) => {
                                match Eval::call(globals, &f, vec![iterator_yield_value]) {
                                    Ok(new_value) => GeneratorResult::Yield(new_value),
                                    Err(_) => return GeneratorResult::Error,
                                }
                            }
                            result => result,
                        }
                    })
                    .into())
                }
            },
        ),
    ]
    .into_iter()
    .map(|f| (sr.intern_rcstr(f.name()), Value::from(f)))
    .collect();
    let static_methods = HashMap::new();
    Class::new0(
        ClassKind::Trait,
        "Iterator".into(),
        vec![base],
        None,
        methods,
        static_methods,
    )
    .into()
}
