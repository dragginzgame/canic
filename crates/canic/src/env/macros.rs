macro_rules! static_canisters {
    ($($name:ident = $id:expr;)*) => {
        $(
            pub static $name: ::std::sync::LazyLock<::candid::Principal> =
                ::std::sync::LazyLock::new(|| ::candid::Principal::from_text($id).unwrap());
        )*

        // auto-generate a test module too
        #[cfg(test)]
        mod __canister_tests {
            use candid::Principal;

            #[test]
            fn all_canister_ids_are_valid() {
                $(
                    assert!(
                        Principal::from_text($id).is_ok(),
                        "Invalid canister id literal for {}: {}",
                        stringify!($name),
                        $id
                    );
                )*
            }
        }
    }
}
