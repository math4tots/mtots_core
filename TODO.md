

Longer term
* Eventually I'd like a separation between the AST and IR
    This way, I could resolve constexprs across modules (with just the
    AST, I'd have enough to resolve constexprs and which other modules to
    import).
    Furthermore, this could mean I can have parse-time constant enums
    and switch statements that can restrict each case to constant expressions
    Not the most pressing issue at the moment however
