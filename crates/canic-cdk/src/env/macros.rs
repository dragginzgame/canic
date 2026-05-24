//! Internal macro to expose static `Principal` handles for known canisters.
//! Data lives in `.inc.rs` files and is shared with build.rs via include!().

macro_rules! static_canisters {
    ($($name:ident = $id:expr;)+) => {
        $(
            pub static $name: ::std::sync::LazyLock<::candid::Principal> =
                ::std::sync::LazyLock::new(|| {
                    ::candid::Principal::from_text($id)
                        .expect("principal literal validated by build.rs")
                });
        )+
    }
}
