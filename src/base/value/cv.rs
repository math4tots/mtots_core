use super::*;

/// Describes a parse time constant value
#[derive(Debug, Clone)]
pub enum ConstVal {
    Nil,
    Bool(bool),
    Number(f64),
    String(RcStr),
}
