//! L1: no top state is `Clone` (01 §8.4), by a compile-time assertion over each of them.

/// Fails to compile where the type is `Clone`: the one blanket impl is ambiguous with the second.
macro_rules! assert_not_clone {
    ($t:ty) => {
        const _: fn() = || {
            trait AmbiguousIfClone<A> {
                fn some_item() {}
            }
            impl<T: ?Sized> AmbiguousIfClone<()> for T {}
            impl<T: ?Sized + Clone> AmbiguousIfClone<u8> for T {}
            let _ = <$t as AmbiguousIfClone<_>>::some_item;
        };
    };
}

assert_not_clone!(crate::StarterApp);
assert_not_clone!(faraday_online::OnlineApp);
assert_not_clone!(faraday_settings::State);
assert_not_clone!(faraday_session::State);
assert_not_clone!(faraday_vaults::State);
