#[doc(hidden)]
#[macro_export]
macro_rules! __icu_load_config {
    () => {
        #[cfg(icu_config)]
        {
            let config_str = include_str!(env!("ICU_CONFIG_PATH"));
            $crate::config::Config::init_from_toml(config_str).unwrap()
        }
    };
}

#[macro_export]
macro_rules! icu_start {
    ($canister_type:expr) => {
        #[::icu::ic::init]
        fn init(
            bundle: ::icu::interface::state::StateBundle,
            parents: Vec<::icu::memory::canister::CanisterEntry>,
            args: Option<Vec<u8>>,
        ) {
            ::icu::log!(::icu::Log::Info, "🚀 init: {}", $canister_type);

            // setup
            ::icu::interface::state::save_state(&bundle);
            ::icu::memory::CanisterState::set_parents(parents);
            ::icu::memory::CanisterState::set_type($canister_type).unwrap();
            __icu_shared_setup();

            let _ = ::icu::ic::timers::set_timer(::std::time::Duration::from_secs(0), move || {
                ::icu::ic::futures::spawn(icu_install(args));
            });
        }

        #[::icu::ic::post_upgrade]
        fn post_upgrade() {
            __icu_shared_setup();

            let _ = ::icu::ic::timers::set_timer(::std::time::Duration::from_secs(0), move || {
                ::icu::ic::futures::spawn(icu_upgrade());
            });
        }

        fn __icu_shared_setup() {
            ::icu::__icu_load_config!();
            ::icu::memory::CycleTracker::start();
            icu_setup();
        }

        ::icu::icu_endpoints!();
    };
}

#[macro_export]
macro_rules! icu_start_root {
    () => {
        #[::icu::ic::init]
        fn init() {
            ::icu::ic::println!("");
            ::icu::log!(
                ::icu::Log::Info,
                "------------------------------------------------------------"
            );
            ::icu::log!(::icu::Log::Info, "🏁 init: root");

            // setup
            ::icu::memory::CanisterState::set_type(&::icu::canister::CanisterType::Root).unwrap();
            __icu_shared_setup();

            // register in CanisterRegistry
            ::icu::memory::CanisterRegistry::init_root(::icu::ic::api::canister_self());

            let _ = ::icu::ic::timers::set_timer(::std::time::Duration::from_secs(0), move || {
                ::icu::ic::futures::spawn(icu_install());
            });
        }

        #[::icu::ic::post_upgrade]
        fn post_upgrade() {
            __icu_shared_setup();

            let _ = ::icu::ic::timers::set_timer(::std::time::Duration::from_secs(0), move || {
                ::icu::ic::futures::spawn(icu_upgrade());
            });
        }

        fn __icu_shared_setup() {
            ::icu::__icu_load_config!();
            ::icu::memory::CycleTracker::start();
            ::icu::state::canister::CanisterCatalog::import(CANISTERS);
            icu_setup();
        }

        ::icu::icu_endpoints!();
        ::icu::icu_endpoints_root!();
    };
}
