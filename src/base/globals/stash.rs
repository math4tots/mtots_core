/// Trait that allows a value to be stored
/// in the Global stash.
pub trait Stashable
where
    Self: 'static + Default,
{
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Globals;
    use std::cell::RefCell;
    use std::rc::Rc;

    struct Sample {
        x: i32,
    }

    impl Stashable for Sample {}
    impl Default for Sample {
        fn default() -> Self {
            Sample { x: 123 }
        }
    }

    #[test]
    fn basic() {
        let mut globals = Globals::new();

        // check that value is properly initialized
        {
            let sample: Rc<RefCell<Sample>> = globals.get_from_stash();
            assert_eq!(sample.borrow().x, 123);
            sample.borrow_mut().x = 999;
            assert_eq!(sample.borrow().x, 999);
        }

        // check that changes persist
        {
            let sample: Rc<RefCell<Sample>> = globals.get_from_stash();
            assert_eq!(sample.borrow().x, 999);
        }
    }
}
