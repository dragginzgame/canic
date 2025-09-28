//
// PUBLIC MACROS
//

#[macro_export]
macro_rules! icu_start {
    ($canister_type:expr) => {
        #[::icu::cdk::init]
        fn init(
            state: ::icu::memory::state::CanisterStateData,
            parents: Vec<::icu::memory::CanisterView>,
            args: Option<Vec<u8>>,
        ) {
            // log (generic, no state info yet)
            ::icu::log!(::icu::Log::Info, "🏁 init: {}", $canister_type);

            // config
            ::icu::__icu_load_config!();

            // tls
            ::icu::eager::init_eager_tls(); // ⚠️ MUST precede init_memory

            // memory
            ::icu::memory::registry::init_memory();
            ::icu::memory::state::CanisterState::import(state);
            ::icu::memory::subnet::SubnetParents::import(parents);
            ::icu::memory::canister::CanisterRoot::set(::icu::cdk::api::msg_caller());

            // services
            ::icu::memory::canister::CycleTracker::start();

            // timers
            let _ = ::icu::cdk::timers::set_timer(::std::time::Duration::from_secs(0), move || {
                ::icu::cdk::futures::spawn(async move {
                    icu_setup().await;
                    icu_install(args).await;
                });
            });
        }

        #[::icu::cdk::post_upgrade]
        fn post_upgrade() {
            // log
            ::icu::log!(::icu::Log::Info, "🏁 post_upgrade: {}", $canister_type);

            // config
            ::icu::__icu_load_config!();

            // tls
            ::icu::eager::init_eager_tls();

            // services
            ::icu::memory::canister::CycleTracker::start();

            // timers
            let _ = ::icu::cdk::timers::set_timer(::std::time::Duration::from_secs(0), move || {
                ::icu::cdk::futures::spawn(async move {
                    icu_setup().await;
                    icu_upgrade().await;
                });
            });
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
            // log
            ::icu::cdk::println!("");
            ::icu::log!(
                ::icu::Log::Info,
                "------------------------------------------------------------"
            );
            ::icu::log!(::icu::Log::Info, "🏁 init: root");

            // config
            ::icu::__icu_load_config!();

            // tls
            ::icu::eager::init_eager_tls();

            // memory
            ::icu::memory::registry::init_memory();
            let entry =
                ::icu::memory::subnet::SubnetRegistry::init_root(::icu::cdk::api::canister_self());
            ::icu::memory::canister::CanisterRoot::set(::icu::cdk::api::canister_self());
            ::icu::memory::state::CanisterState::set_view(entry.into());

            // state
            ::icu::state::wasm::WasmRegistry::import(WASMS);

            // services
            ::icu::memory::canister::CycleTracker::start();
            ::icu::memory::root::CanisterReserve::start();

            // timers
            let _ = ::icu::cdk::timers::set_timer(::std::time::Duration::from_secs(0), move || {
                ::icu::cdk::futures::spawn(async move {
                    icu_setup().await;
                    icu_install().await;
                });
            });
        }

        #[::icu::cdk::post_upgrade]
        fn post_upgrade() {
            // log
            ::icu::log!(::icu::Log::Info, "🏁 post_upgrade: root");

            // config
            ::icu::__icu_load_config!();

            // tls
            ::icu::eager::init_eager_tls();

            // state
            ::icu::state::wasm::WasmRegistry::import(WASMS);

            // services
            ::icu::memory::canister::CycleTracker::start();
            ::icu::memory::root::CanisterReserve::start();

            // timers
            let _ = ::icu::cdk::timers::set_timer(::std::time::Duration::from_secs(0), move || {
                ::icu::cdk::futures::spawn(async move {
                    icu_setup().await;
                    icu_upgrade().await;
                });
            });
        }

        ::icu::icu_endpoints!();
        ::icu::icu_endpoints_root!();
    };
}

///
/// PRIVATE MACROS
///

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
