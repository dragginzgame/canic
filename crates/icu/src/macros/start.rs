#[doc(hidden)]
#[macro_export]
macro_rules! __icu_load_config {
    () => {
        #[cfg(icu)]
        {
            let config_str = include_str!(env!("ICU_CONFIG_PATH"));
            $crate::expect_or_trap(
                $crate::config::Config::init_from_toml(config_str),
                "init config",
            )
        }
    };
}

#[macro_export]
macro_rules! icu_start {
    ($canister_type:expr) => {
        #[::icu::cdk::init]
        fn init(
            state: ::icu::memory::state::CanisterStateData,
            parents: Vec<::icu::memory::CanisterView>,
            args: Option<Vec<u8>>,
        ) {
            ::icu::memory::state::CanisterState::import(state);
            ::icu::log!(::icu::Log::Info, "🏁 init: {}", $canister_type);

            // setup
            ::icu::memory::subnet::SubnetParents::import(parents);
            ::icu::memory::canister::CanisterRoot::set(::icu::cdk::api::msg_caller());
            __icu_shared_setup();

            // timer - icu_install
            let _ = ::icu::cdk::timers::set_timer(::std::time::Duration::from_secs(0), move || {
                ::icu::cdk::futures::spawn(icu_install(args));
            });
        }

        #[::icu::cdk::post_upgrade]
        fn post_upgrade() {
            __icu_shared_setup();

            // timer - icu_upgrade
            let _ = ::icu::cdk::timers::set_timer(::std::time::Duration::from_secs(0), move || {
                ::icu::cdk::futures::spawn(icu_upgrade());
            });
        }

        #[allow(unexpected_cfgs)]
        fn __icu_shared_setup() {
            ::icu::__icu_load_config!();
            ::icu::memory::registry::force_init_all_tls();
            ::icu::memory::canister::CycleTracker::start();

            icu_setup();
        }

        ::icu::icu_endpoints!();
        ::icu::icu_endpoints_nonroot!();
    };
}

#[macro_export]
macro_rules! icu_start_root {
    () => {
        #[::icu::cdk::init]
        fn init() {
            ::icu::cdk::println!("");
            ::icu::log!(
                ::icu::Log::Info,
                "------------------------------------------------------------"
            );
            ::icu::log!(::icu::Log::Info, "🏁 init: root");

            // setup
            let entry =
                ::icu::memory::subnet::SubnetRegistry::init_root(::icu::cdk::api::canister_self());
            ::icu::memory::canister::CanisterRoot::set(::icu::cdk::api::canister_self());
            ::icu::memory::state::CanisterState::set_view(entry.into());
            __icu_root_shared_setup();

            // timer - icu_install
            let _ = ::icu::cdk::timers::set_timer(::std::time::Duration::from_secs(0), move || {
                ::icu::cdk::futures::spawn(icu_install());
            });
        }

        #[::icu::cdk::post_upgrade]
        fn post_upgrade() {
            __icu_root_shared_setup();

            // timer - icu_upgrade
            let _ = ::icu::cdk::timers::set_timer(::std::time::Duration::from_secs(0), move || {
                ::icu::cdk::futures::spawn(icu_upgrade());
            });
        }

        #[allow(unexpected_cfgs)]
        fn __icu_root_shared_setup() {
            ::icu::__icu_load_config!();
            ::icu::memory::registry::force_init_all_tls();
            ::icu::memory::root::CanisterPool::start();
            ::icu::memory::canister::CycleTracker::start();
            ::icu::state::wasm::WasmRegistry::import(WASMS);

            icu_setup();
        }

        ::icu::icu_endpoints!();
        ::icu::icu_endpoints_root!();
    };
}
