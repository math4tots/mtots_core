use super::*;

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Invalid => write!(f, "<invalid>"),
            Value::Nil => write!(f, "nil"),
            Value::Bool(x) => write!(f, "{}", if *x { "true" } else { "false" }),
            Value::Number(x) => write!(f, "{}", x),
            Value::String(x) => {
                write!(f, "\"")?;
                for c in x.chars() {
                    match c {
                        '\\' => write!(f, "\\\\")?,
                        '\"' => write!(f, "\\\"")?,
                        '\'' => write!(f, "\\\'")?,
                        '\n' => write!(f, "\\\n")?,
                        '\r' => write!(f, "\\\r")?,
                        '\t' => write!(f, "\\\t")?,
                        _ => write!(f, "{}", c)?,
                    }
                }
                write!(f, "\"")
            }
            Value::List(xs) => {
                write!(f, "[")?;
                for (i, x) in xs.borrow().iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{:?}", x)?;
                }
                write!(f, "]")
            }
            Value::Set(xs) => {
                write!(f, "Set([")?;
                for (i, x) in xs.sorted_keys().into_iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{:?}", Value::from(x.clone()))?;
                }
                write!(f, "])")
            }
            Value::Map(xs) => {
                write!(f, "[")?;
                if xs.borrow().is_empty() {
                    write!(f, ":")?;
                } else {
                    for (i, (k, v)) in xs.borrow().iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{:?}: {:?}", Value::from(k.clone()), v)?;
                    }
                }
                write!(f, "]")
            }
            Value::Function(func) => write!(f, "{:?}", func),
            Value::NativeFunction(func) => write!(f, "{:?}", func),
            Value::Generator(gen) => write!(f, "{:?}", gen.borrow()),
            Value::NativeGenerator(gen) => write!(f, "{:?}", gen.borrow()),
            Value::Module(module) => write!(f, "{:?}", module),
            Value::Class(cls) => write!(f, "{:?}", cls),
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::String(x) => write!(f, "{}", x),
            _ => write!(f, "{:?}", self),
        }
    }
}
