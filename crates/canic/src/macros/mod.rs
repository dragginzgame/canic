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

/// Log elapsed instruction counts since the last `perf!` invocation.
///
/// Records the delta in instructions between calls and emits a
/// [`Log::Perf`](crate::Log::Perf)
/// entry with the provided label (any tokens accepted by `format!`). Use this
/// to highlight hot paths in long-running maintenance tasks.
#[macro_export]
macro_rules! perf {
    ($($label:tt)*) => {{
        $crate::model::PERF_LAST.with(|last| {
            let now = $crate::cdk::api::performance_counter(1);
            let then = *last.borrow();
            let delta = now.saturating_sub(then);
            *last.borrow_mut() = now;

            let delta_fmt = $crate::utils::instructions::format_instructions(delta);
            let now_fmt = $crate::utils::instructions::format_instructions(now);

            $crate::cdk::println!(
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
        $crate::export::defer::defer!({
            let end = $crate::cdk::api::performance_counter(1);
            let end_fmt = $crate::utils::instructions::format_instructions(end);

            $crate::cdk::println!("{} used {}i in this call", module_path!(), end_fmt,)
        });
    };
}
