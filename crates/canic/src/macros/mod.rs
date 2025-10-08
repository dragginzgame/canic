//! Public macro entry points used across Canic.
//!
//! The submodules host build-time helpers (`build`), canister lifecycle
//! scaffolding (`start`), memory registry shortcuts (`memory`), thread-local
//! eager initializers (`thread`), and endpoint generators (`endpoints`). This
//! top-level module exposes lightweight instrumentation macros shared across
//! those layers.

pub mod build;
pub mod endpoints;
pub mod memory;
pub mod runtime;
pub mod start;
pub mod storable;

/// Run `$body` during process start-up using `ctor`.
///
/// The macro expands to a `ctor` hook so eager TLS initializers can register
/// their work before any canister lifecycle hooks execute. Prefer wrapping
/// the body in a separate function for larger initializers to keep the hook
/// simple.
#[macro_export]
macro_rules! eager_init {
    ($body:block) => {
        #[ $crate::export::ctor::ctor(anonymous, crate_path = $crate::export::ctor) ]
        fn __canic_eager_init() {
            $body
        }
    };
}

/// Emit a structured log line with consistent coloring and headers.
///
/// Accepts an optional [`Log`](crate::Log) level followed by a format string
/// and arguments, mirroring `format!`. When the level is omitted the macro
/// defaults to [`Log::Debug`](crate::Log::Debug).
#[macro_export]
macro_rules! log {
    // Explicit level, no args
    ($level:expr, $fmt:expr) => {{
        $crate::log!(@inner $level, $fmt,);
    }};

    // Explicit level, with args
    ($level:expr, $fmt:expr, $($arg:tt)*) => {{
        $crate::log!(@inner $level, $fmt, $($arg)*);
    }};

    // No level given, default to Info
    ($fmt:expr) => {{
        $crate::log!(@inner $crate::Log::Debug, $fmt,);
    }};
    ($fmt:expr, $($arg:tt)*) => {{
        $crate::log!(@inner $crate::Log::Debug, $fmt, $($arg)*);
    }};


    // Inner logic
    (@inner $level:expr, $fmt:expr, $($arg:tt)*) => {{
        let message = format!($fmt, $($arg)*);
        let ty_raw = match $crate::memory::state::CanisterState::get() {
            Some(entry) => entry.ty.to_string(),
            None => "...".to_string(),
        };
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
            $crate::Log::Error => format!("\x1b[31mERROR\x1b[0m|{ty_col}| {message}"),
            $crate::Log::Debug => format!("DEBUG|{ty_col}| {message}"),
        };

        $crate::cdk::println!("{final_line}");
    }};
}

/// Log elapsed instruction counts since the last `perf!` invocation.
///
/// Records the delta in instructions between calls and emits a
/// [`Log::Perf`](crate::Log::Perf)
/// entry with the provided label (any tokens accepted by `format!`). Use this
/// to highlight hot paths in long-running maintenance tasks.
#[macro_export]
macro_rules! perf {
    ($($label:tt)*) => {{
        $crate::state::PERF_LAST.with(|last| {
            let now = ::canic::cdk::api::performance_counter(1);
            let then = *last.borrow();
            let delta = now.saturating_sub(then);
            *last.borrow_mut() = now;

            let delta_fmt = $crate::format_instructions(delta);
            let now_fmt = $crate::format_instructions(now);

            $crate::log!(
                ::canic::Log::Perf,
                "{}: '{}' used {}i since last (total: {}i)",
                module_path!(),
                format!($($label)*),
                delta_fmt,
                now_fmt
            );
        });
    }};
}

/// Record a single-call instruction counter snapshot when the surrounding
/// scope exits.
///
/// Expands to a `defer!` guard that logs the total instructions consumed in
/// the enclosing scope, tagged as [`Log::Perf`](crate::Log::Perf). Pair this
/// with manual checkpoints logged via [`macro@perf`] to track both cumulative and incremental
/// usage.
#[macro_export]
macro_rules! perf_start {
    () => {
        ::canic::export::defer::defer!({
            let end = ::canic::cdk::api::performance_counter(1);
            let end_fmt = ::canic::utils::instructions::format_instructions(end);

            $crate::log!(
                ::canic::Log::Perf,
                "{} used {}i in this call",
                module_path!(),
                end_fmt,
            )
        });
    };
}
