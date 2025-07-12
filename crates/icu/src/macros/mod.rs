pub mod endpoints;
pub mod memory;

/// icu_start
#[macro_export]
macro_rules! icu_start {
    ($canister_path:expr) => {
        #[::icu::ic::init]
        fn init(
            root_pid: ::candid::Principal,
            parent_pid: ::candid::Principal,
            args: Option<Vec<u8>>,
        ) {
            use ::icu::interface::memory::canister::state;

            ::icu::log!(::icu::Log::Info, "init: {}", $canister_path);

            ::icu::memory::init();

            state::set_root_pid(root_pid).unwrap();
            state::set_parent_pid(parent_pid).unwrap();
            state::set_path($canister_path).unwrap();

            // automatically calls init_async
            let _ = ::icu::ic::timers::set_timer(::std::time::Duration::from_secs(0), move || {
                ::icu::ic::futures::spawn(init_async(args))
            });
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

            ::icu::log!(::icu::Log::Info, "init: {}", $canister_path);

            ::icu::memory::init();

            state::set_root_pid(::icu::ic::api::canister_self()).unwrap();
            state::set_path($canister_path).unwrap();

            // automatically calls init_async
            let _ = ::icu::ic::timers::set_timer(::std::time::Duration::from_secs(0), move || {
                ::icu::ic::futures::spawn(init_async())
            });
        }

        ::icu::icu_endpoints!();

        // app
        // modify app-level state
        // @todo eventually this will cascade down from an orchestrator canister
        #[::icu::ic::update]
        async fn icu_app(cmd: ::icu::memory::app::AppCommand) -> Result<(), ::icu::Error> {
            ::icu::interface::memory::app::state::command(cmd)?;
            ::icu::interface::cascade::app_state_cascade().await?;

            Ok(())
        }

        // response
        #[::icu::ic::update]
        async fn icu_response(
            request: ::icu::interface::request::Request,
        ) -> Result<::icu::interface::response::Response, ::icu::Error> {
            let response = ::icu::interface::response::response(request).await?;

            Ok(response)
        }
    };
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
            $crate::Log::Ok => format!("\x1b[32mOK\x1b[0m: {}", formatted_message),
            $crate::Log::Perf => format!("\x1b[35mPERF\x1b[0m: {}", formatted_message),
            $crate::Log::Info => format!("\x1b[34mINFO\x1b[0m: {}", formatted_message),
            $crate::Log::Warn => format!("\x1b[33mWARN\x1b[0m: {}", formatted_message),
            $crate::Log::Error => format!("\x1b[31mERROR\x1b[0m: {}", formatted_message),
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
            let end_fmt = ::icu::format_instructions(end);

            $crate::log!(
                ::icu::Log::Perf,
                "{} used {}i in this call",
                module_path!(),
                end_fmt,
            )
        });
    };
}
