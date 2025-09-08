pub mod build;
pub mod endpoints;
pub mod memory;
pub mod start;

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
        let message = format!($fmt, $($arg)*);  // Apply formatting with args
        let ty_raw = match $crate::memory::CanisterState::get_type() {
            Some(ty) => ty.to_string(),
            None => "-".to_string(),
        };

        // Ellipsize long types to keep the pipe-aligned column centered
        let ty_disp = $crate::utils::format::ellipsize_middle(
            &ty_raw,
            $crate::LOG_CANISTER_TYPE_ELLIPSIS_THRESHOLD,
            4,
            4,
        );
        let ty_col = format!("{:^width$}", ty_disp, width = $crate::LOG_CANISTER_TYPE_WIDTH);

        let final_line = match $level {
            $crate::Log::Ok => format!("\x1b[32m OK  \x1b[0m|{ty_col}| {message}"),
            $crate::Log::Perf => format!("\x1b[35mPERF \x1b[0m|{ty_col}| {message}"),
            $crate::Log::Info => format!("\x1b[34mINFO \x1b[0m|{ty_col}| {message}"),
            $crate::Log::Warn => format!("\x1b[33mWARN \x1b[0m|{ty_col}| {message}"),
            $crate::Log::Error => format!("\x1b[31mERR  \x1b[0m|{ty_col}| {message}"),
        };

        $crate::cdk::println!("{final_line}");
    }};
}

// perf
#[macro_export]
macro_rules! perf {
    ($($label:tt)*) => {{
        $crate::state::PERF_LAST.with(|last| {
            let now = ::icu::cdk::api::performance_counter(1);
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
            let end = ::icu::cdk::api::performance_counter(1);
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
