#![macro_use]

// See:
// https://users.rust-lang.org/t/a-macro-to-assert-that-a-type-does-not-implement-trait-bounds/31179
macro_rules! assert_not_impl {
    ($x:ty, $($t:path),+ $(,)*) => {
        const _: fn() -> () = || {
            struct Check<T: ?Sized>(T);
            trait AmbiguousIfImpl<A> { fn some_item() { } }

            impl<T: ?Sized> AmbiguousIfImpl<()> for Check<T> { }
            impl<T: ?Sized $(+ $t)*> AmbiguousIfImpl<u8> for Check<T> { }

            // Due to the way this macro works, an error in the following line
            // probably indicates that an assert_not_impl assertion is being
            // broken somewhere
            <Check::<$x> as AmbiguousIfImpl<_>>::some_item()
        };
    };
}
