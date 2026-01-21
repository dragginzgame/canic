//! Facade macros for downstream canister crates.
mod build;
mod endpoints;
mod start;
mod timer;

// -----------------------------------------------------------------------------
// Log macro
// -----------------------------------------------------------------------------

/// Log a runtime entry using Canic's structured logger.
#[macro_export]
macro_rules! log {
    ($($tt:tt)*) => {{
        $crate::__internal::core::log!($($tt)*);
    }};
}

// -----------------------------------------------------------------------------
// Perf macro
// -----------------------------------------------------------------------------

/// Log elapsed instruction counts since the last `perf!` invocation in this thread.
///
/// - Uses a thread-local `PERF_LAST` snapshot.
/// - Computes `delta = now - last`.
/// - Prints a human-readable line for debugging.
///
/// Intended usage:
/// - Long-running maintenance tasks where you want *checkpoints* in a single call.
///
/// Note: `perf!` is independent of endpoint perf scopes and does not touch the
/// endpoint stack used by dispatch.
///
/// Notes:
/// - On non-wasm targets, `perf_counter()` returns 0, so this becomes a no-op-ish
///   counter (still records 0 deltas); this keeps unit tests compiling cleanly.
#[macro_export]
macro_rules! perf {
    ($($label:tt)*) => {{
        $crate::__internal::core::perf::PERF_LAST.with(|last| {
            // Use the wrapper so non-wasm builds compile.
            let now = $crate::__internal::core::perf::perf_counter();
            let then = *last.borrow();
            let delta = now.saturating_sub(then);

            // Update last checkpoint.
            *last.borrow_mut() = now;

            // Format label + pretty-print counters.
            let label = format!($($label)*);
            let delta_fmt = $crate::utils::instructions::format_instructions(delta);
            let now_fmt = $crate::utils::instructions::format_instructions(now);

            // ❌ NO structured recording here
            // ✔️ Debug log only
            $crate::__internal::core::log!(
                Info,
                Topic::Perf,
                "{}: '{}' used {}i since last (total: {}i)",
                module_path!(),
                label,
                delta_fmt,
                now_fmt
            );
        });
    }};
}
