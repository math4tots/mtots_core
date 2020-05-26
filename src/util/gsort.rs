use std::cmp;
/// Failable sort
/// I can't use the builtin sorting functions, because they
/// don't support the case where the comparison operation itself
/// may fail
use std::mem;

/// Simple failable merge sort
/// TODO: implement a faster sort
pub fn gsort<S, V, F, E>(state: &mut S, arr: &mut [V], mut lt: F) -> Result<(), E>
where
    V: Default,
    F: FnMut(&mut S, &V, &V) -> Result<bool, E>,
{
    let mut buffer = mkvec(arr.len());
    let mut stack = vec![Sort(0, arr.len())];

    loop {
        match stack.pop() {
            Some(Sort(a, b)) => {
                if b - a > 1 {
                    let m = (a + b) / 2;
                    stack.push(Merge(a, m, b));
                    stack.push(Sort(m, b));
                    stack.push(Sort(a, m));
                }
            }
            Some(Merge(a, m, b)) => {
                {
                    let (x, y) = arr.split_at_mut(m);
                    let x = &mut x[a..];
                    let y = &mut y[..b - m];
                    let z = &mut buffer[a..b];
                    let mut i = 0;
                    let mut j = 0;
                    let mut k = 0;
                    while i < x.len() && j < y.len() {
                        // if x[i] <= y[j], pick x[i] to keep the sort stable
                        z[k] = if !lt(state, &y[j], &x[i])? {
                            i += 1;
                            mem::replace(&mut x[i - 1], Default::default())
                        } else {
                            j += 1;
                            mem::replace(&mut y[j - 1], Default::default())
                        };
                        k += 1;
                    }
                    while i < x.len() {
                        z[k] = mem::replace(&mut x[i], Default::default());
                        i += 1;
                        k += 1;
                    }
                    while j < y.len() {
                        z[k] = mem::replace(&mut y[j], Default::default());
                        j += 1;
                        k += 1;
                    }
                }
                for i in a..b {
                    arr[i] = mem::replace(&mut buffer[i], Default::default());
                }
            }
            None => break,
        }
    }

    Ok(())
}

/// Calls gsort, but with trivial type arguments for
/// the error handling parts so that at least the sorting
/// part is easier to test
pub fn msort<V: Default + cmp::Ord>(arr: &mut [V]) {
    gsort::<(), V, _, ()>(&mut (), arr, |_, lhs, rhs| Ok(lhs < rhs)).unwrap()
}

fn mkvec<T: Default>(n: usize) -> Vec<T> {
    let mut ret = Vec::new();
    for _ in 0..n {
        ret.push(Default::default());
    }
    ret
}

enum Command {
    Sort(usize, usize),
    Merge(usize, usize, usize),
}

use Command::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
        let mut arr = vec![5, 4, 3, 2, 1, 2, 3, 4, 5];
        msort(&mut arr);
        assert_eq!(arr, vec![1, 2, 2, 3, 3, 4, 4, 5, 5],);
        let mut arr = vec![1, 2, 3, 4, 3, 2, 1];
        msort(&mut arr);
        assert_eq!(arr, vec![1, 1, 2, 2, 3, 3, 4],);
    }
}
