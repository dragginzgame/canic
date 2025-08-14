#[macro_export]
macro_rules! icu_config {
    () => {{
        #[cfg(icu_config)]
        {
            let config_str = include_str!(env!("ICU_CONFIG_PATH"));
            $crate::config::Config::init_from_toml(config_str).unwrap()
        }
    }};
}

#[macro_export]
macro_rules! icu_start {
    ($kind:expr) => {
        #[::icu::ic::init]
        fn init(
            bundle: ::icu::interface::state::StateBundle,
            parents: Vec<::icu::memory::canister::CanisterParent>,
            args: Option<Vec<u8>>,
        ) {
            ::icu::log!(::icu::Log::Info, "🚀 init: {}", $kind);

            ::icu::interface::state::save_state(&bundle);
            ::icu::memory::CanisterState::set_parents(parents);
            ::icu::memory::CanisterState::set_kind($kind).unwrap();

            // config
            icu_config!();

            // automatically calls icu_init
            let _ = ::icu::ic::timers::set_timer(::std::time::Duration::from_secs(0), move || {
                ::icu::ic::futures::spawn(icu_setup());
                ::icu::ic::futures::spawn(icu_install(args));
            });
        }

        #[::icu::ic::post_upgrade]
        fn post_upgrade() {
            icu_config!();

            let _ = ::icu::ic::timers::set_timer(::std::time::Duration::from_secs(0), move || {
                ::icu::ic::futures::spawn(icu_setup());
                ::icu::ic::futures::spawn(icu_upgrade());
            });
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
                "-------------------------------------------------------"
            );
            ::icu::log!(::icu::Log::Info, "🏁 init: root");

            ::icu::memory::CanisterState::set_kind_root();

            // config
            icu_config!();

            // import canisters
            ::icu::canister::CanisterRegistry::import(CANISTERS);

            // automatically calls init_async
            let _ = ::icu::ic::timers::set_timer(::std::time::Duration::from_secs(0), move || {
                ::icu::ic::futures::spawn(icu_setup());
                ::icu::ic::futures::spawn(icu_install());
            });
        }

        #[::icu::ic::post_upgrade]
        fn post_upgrade() {
            // config
            icu_config!();

            // import canisters
            ::icu::canister::CanisterRegistry::import(CANISTERS);

            let _ = ::icu::ic::timers::set_timer(::std::time::Duration::from_secs(0), move || {
                ::icu::ic::futures::spawn(icu_setup());
                ::icu::ic::futures::spawn(icu_upgrade());
            });
        }

        ::icu::icu_endpoints!();
        ::icu::icu_endpoints_root!();

        // app
        // modify app-level state
        // @todo eventually this will cascade down from an orchestrator canister
        #[::icu::ic::update]
        async fn icu_app(cmd: ::icu::memory::app::AppCommand) -> Result<(), ::icu::Error> {
            ::icu::memory::AppState::command(cmd)?;

            let bundle = ::icu::interface::state::StateBundle::app_state();
            ::icu::interface::state::cascade(&bundle).await?;

            Ok(())
        }

        // response
        #[::icu::ic::update]
        async fn icu_response(
            request: ::icu::interface::request::Request,
        ) -> Result<::icu::interface::root::response::Response, ::icu::Error> {
            let response = ::icu::interface::root::response::response(request).await?;

            Ok(response)
        }
    };
}
