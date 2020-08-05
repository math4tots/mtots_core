//! Some references:
//! * https://mail.mozilla.org/pipermail/rust-dev/2013-April/003680.html
//! * Boute 1992 (https://core.ac.uk/download/pdf/55698442.pdf)

#[inline(always)]
pub(crate) fn divmod(a: i64, b: i64) -> (i64, i64) {
    if a > 0 && b < 0 {
        ((a - 1) / b - 1, (a - 1) % b + b + 1)
    } else if a < 0 && b > 0 {
        ((a + 1) / b - 1, (a + 1) % b + b - 1)
    } else {
        (a / b, a % b)
    }
}

// TODO: divmod for f64
