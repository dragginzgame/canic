/// icu_start
#[macro_export]
macro_rules! icu_start {
    // private implementation arm: accepts optional extra‐argument tokens
    ($canister_path:path) => {
        #[::icu::ic::init]
        fn init(root_pid: ::candid::Principal, parent_pid: ::candid::Principal) {
            use ::icu::interface::memory::canister::state;

            ::icu::log!(
                ::icu::Log::Info,
                "init: {} root: {root_pid} parent: {parent_pid}",
                $canister_path
            );

            ::icu::memory::init();

            state::set_root_pid(root_pid).unwrap();
            state::set_parent_pid(parent_pid).unwrap();
            state::set_path($canister_path).unwrap();

            _init()
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
            use ::icu::interface::memory::canister::state;

            ::icu::memory::init();

            state::set_root_pid(::icu::ic::api::canister_self()).unwrap();
            state::set_path($canister_path).unwrap();

            log!(Log::Info, "init: {} (root)", $canister_path);

            _init()
        }

        ::icu::icu_endpoints_root!();
        ::icu::icu_endpoints!();
    };
}
