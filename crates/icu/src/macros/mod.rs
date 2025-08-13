pub mod endpoints;
pub mod memory;
pub mod root;

/// icu_start
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

            // automatically calls icu_init
            let _ = ::icu::ic::timers::set_timer(::std::time::Duration::from_secs(0), move || {
                ::icu::ic::futures::spawn(icu_init(args));
                ::icu::ic::futures::spawn(icu_startup());
            });
        }

        ::icu::icu_endpoints!();
    };
}

/// icu_start_root
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

            // automatically calls init_async
            let _ = ::icu::ic::timers::set_timer(::std::time::Duration::from_secs(0), move || {
                ::icu::ic::futures::spawn(icu_init());
                ::icu::ic::futures::spawn(icu_startup());
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

// icu_config
#[macro_export]
macro_rules! icu_config {
    ($file:expr) => {{
        let config_str = include_str!($file);
        $crate::config::Config::init_from_toml(config_str).unwrap()
    }};
}

// log
#[macro_export]
macro_rules! log {
    ($level:expr, $fmt:expr) => {{
        // Pass an empty set of arguments to @inner
        $crate::log!(@inner $level, $fmt,);
    }};

    // Match when additional arguments are provided
    ($level:expr, $fmt:expr, $($arg:tt)*) => {{
        $crate::log!(@inner $level, $fmt, $($arg)*);
    }};

    // Inner macro for actual logging logic to avoid code duplication
    (@inner $level:expr, $fmt:expr, $($arg:tt)*) => {{
        let formatted_message = format!($fmt, $($arg)*);  // Apply formatting with args

        let msg = match $level {
            $crate::Log::Ok => format!("\x1b[32mOK  \x1b[0m {}", formatted_message),
            $crate::Log::Perf => format!("\x1b[35mPERF\x1b[0m {}", formatted_message),
            $crate::Log::Info => format!("\x1b[34mINFO\x1b[0m {}", formatted_message),
            $crate::Log::Warn => format!("\x1b[33mWARN\x1b[0m {}", formatted_message),
            $crate::Log::Error => format!("\x1b[31mERR \x1b[0m {}", formatted_message),
        };

        $crate::ic::println!("{}", msg);
    }};
}

// perf
#[macro_export]
macro_rules! perf {
    ($($label:tt)*) => {{
        $crate::state::PERF_LAST.with(|last| {
            let now = ::icu::ic::api::performance_counter(1);
            let then = *last.borrow();
            let delta = now.saturating_sub(then);
            *last.borrow_mut() = now;

            let delta_fmt = $crate::format_instructions(delta);
            let now_fmt = $crate::format_instructions(now);

            $crate::log!(
                ::icu::Log::Perf,
                "{}: '{}' used {}i since last (total: {}i)",
                module_path!(),
                format!($($label)*),
                delta_fmt,
                now_fmt
            );
        });
    }};
}

// perf_start
#[macro_export]
macro_rules! perf_start {
    () => {
        ::icu::export::defer::defer!({
            let end = ::icu::ic::api::performance_counter(1);
            let end_fmt = ::icu::utils::instructions::format_instructions(end);

            $crate::log!(
                ::icu::Log::Perf,
                "{} used {}i in this call",
                module_path!(),
                end_fmt,
            )
        });
    };
}
