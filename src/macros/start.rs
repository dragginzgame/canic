/// icu_start
#[macro_export]
macro_rules! icu_start {
    // Shared macro body: takes arg list and init call
    (@body $canister_path:path, [$($param:tt)*], $init_call:expr) => {
        #[::icu::ic::init]
        fn init(root_pid: ::candid::Principal, parent_pid: ::candid::Principal $($param)*) {
            use ::icu::interface::memory::canister::state;

            ::icu::memory::init();

            state::set_root_pid(root_pid).unwrap();
            state::set_parent_pid(parent_pid).unwrap();
            state::set_path($canister_path).unwrap();

            log!(Log::Info, "init: {}", $canister_path);

            $init_call
        }

        #[::icu::ic::update]
        async fn init_async() {
            _init_async().await;
        }

        ::icu::icu_endpoints!();
    };

    // with arguments
    (
        $canister_path:path,
        args = ( $($aname:ident : $aty:ty),* $(,)? )
        $(,)?
    ) => {
        $crate::icu_start!(@body $canister_path, [, $($aname : $aty),*], _init($($aname),*););
    };

    // without arguments
    (
        $canister_path:path
        $(,)?
    ) => {
        $crate::icu_start!(@body $canister_path, [], _init(););
    };
}

/// icu_start_root
#[macro_export]
macro_rules! icu_start_root {
    ($canister_path:path) => {
        #[::icu::ic::init]
        fn init() {
            use ::icu::interface::memory::canister::state;

            ::icu::memory::init();

            state::set_root_pid(::icu::ic::api::canister_self()).unwrap();
            state::set_path($canister_path).unwrap();

            log!(Log::Info, "init: {} (root)", $canister_path);

            _init();
        }

        #[::icu::ic::update]
        async fn init_async() {
            _init_async().await;
        }

        ::icu::icu_endpoints_root!();
        ::icu::icu_endpoints!();
    };
}
