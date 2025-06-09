/// icu_start
#[macro_export]
macro_rules! icu_start {
    (
        $canister_path:path,
        args = $arg_ty:ty
        $(,)?
    ) => {
        #[::icu::ic::init]
        fn init(args: $arg_ty) {
            use ::icu::interface::memory::canister::state;

            ::icu::memory::init();

            ::icu::log!(::icu::Log::Info, "init: {}", $canister_path);

            state::set_root_pid(args.root_pid).unwrap();
            state::set_parent_pid(args.parent_pid).unwrap();
            state::set_path($canister_path).unwrap();

            _init(args.extra);
        }

        #[::icu::ic::update]
        async fn init_async() {
            _init_async().await;
        }

        ::icu::icu_endpoints!();
    };

    // Default to no extra args (InitArgs<()>)
    (
        $canister_path:path
        $(,)?
    ) => {
        $crate::icu_start!($canister_path, args = ::icu::InitArgs<()>);
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
