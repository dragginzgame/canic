/// icu_start
#[macro_export]
macro_rules! icu_start {
    ($canister_path:path) => {
        #[::icu::ic::init]
        fn init(root_id: ::candid::Principal, parent_id: ::candid::Principal) {
            use ::icu::interface::state::core::canister_state;

            canister_state::set_root_id(root_id).unwrap();
            canister_state::set_parent_id(parent_id).unwrap();
            canister_state::set_path($canister_path).unwrap();

            log!(Log::Info, "init: {}", $canister_path);

            _init();
        }

        #[::icu::ic::update]
        async fn init_async() {
            _init_async().await;
        }

        ::icu::icu_endpoints!();
    };
}

/// icu_start_root
#[macro_export]
macro_rules! icu_start_root {
    ($canister_path:path) => {
        #[::icu::ic::init]
        fn init() {
            use ::icu::interface::state::core::canister_state;

            canister_state::set_root_id(::icu::ic::api::canister_self()).unwrap();
            canister_state::set_path($canister_path).unwrap();

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
