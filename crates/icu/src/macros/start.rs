/// icu_start
#[macro_export]
macro_rules! icu_start {
    // No arg version
    (
        $canister_path:path
        $(,)?
    ) => {
        #[::icu::ic::init]
        fn init(root_pid: ::candid::Principal, parent_pid: ::candid::Principal) {
            use ::icu::interface::memory::canister::state;

            ::icu::memory::init();
            ::icu::log!(::icu::Log::Info, "init: {}", $canister_path);

            state::set_root_pid(root_pid).unwrap();
            state::set_parent_pid(parent_pid).unwrap();
            state::set_path($canister_path).unwrap();

            _init();
        }

        #[::icu::ic::update]
        async fn init_async() {
            _init_async().await;
        }

        ::icu::icu_endpoints!();
    };

    // Single argument version
    (
        $canister_path:path,
        extra = $extra_ty:ty
        $(,)?
    ) => {
        #[::icu::ic::init]
        fn init(root_pid: ::candid::Principal, parent_pid: ::candid::Principal, extra: $extra_ty) {
            use ::icu::interface::memory::canister::state;

            ::icu::memory::init();
            ::icu::log!(::icu::Log::Info, "init: {}", $canister_path);
            state::set_root_pid(root_pid).unwrap();
            state::set_parent_pid(parent_pid).unwrap();
            state::set_path($canister_path).unwrap();

            _init(extra);
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
