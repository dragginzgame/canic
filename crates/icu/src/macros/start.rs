/// icu_start
#[macro_export]
macro_rules! icu_start {
    // private implementation arm: accepts optional extra‐argument tokens
    (@impl $canister_path:path, ( $($extra_arg:ident: $extra_ty:ty)? ), ( $($extra:ident)? )) => {
        #[::icu::ic::init]
        fn init(
            root_pid: ::candid::Principal,
            parent_pid: ::candid::Principal
            $(, $extra_arg: $extra_ty)?
        ) {
            use ::icu::interface::memory::canister::state;

            ::icu::memory::init();
            ::icu::log!(::icu::Log::Info, "init: {}", $canister_path);

            state::set_root_pid(root_pid).unwrap();
            state::set_parent_pid(parent_pid).unwrap();
            state::set_path($canister_path).unwrap();

            // call your user‐defined initializer
            _init( $($extra)? );
        }

        #[::icu::ic::update]
        async fn init_async() {
            _init_async().await;
        }

        ::icu::icu_endpoints!();
    };

    // public arm: no extra
    (
        $canister_path:path
        $(,)?
    ) => {
        icu_start!(@impl $canister_path, (), () );
    };

    // public arm: with extra
    (
        $canister_path:path,
        extra = $extra_ty:ty
        $(,)?
    ) => {
        icu_start!(@impl $canister_path, (extra: $extra_ty), (extra) );
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
