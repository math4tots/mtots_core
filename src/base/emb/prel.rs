pub(super) const SOURCE: &str = r###"
"""
The prelude script
Values defined here will be available to every other mtots script
by default
"""

def* range(start, end=nil) {
    if end is nil {
        end = start
        start = 0
    }

    while start < end {
        yield start
        start = start + 1
    }
}
"###;