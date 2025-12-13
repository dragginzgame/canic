/// Log elapsed instruction counts since the last `perf!` invocation in this thread.
///
/// - Uses a thread-local `PERF_LAST` snapshot.
/// - Computes `delta = now - last`.
/// - Records the delta under the provided label (aggregated via `perf::record`).
/// - Prints a human-readable line for debugging.
///
/// Intended usage:
/// - Long-running maintenance tasks where you want *checkpoints* in a single call.
/// - Pair with `perf_scope!` to also capture the *full call total* at scope exit.
///
/// Notes:
/// - On non-wasm targets, `perf_counter()` returns 0, so this becomes a no-op-ish
///   counter (still records 0 deltas); this keeps unit tests compiling cleanly.
#[macro_export]
macro_rules! perf {
    ($($label:tt)*) => {{
        $crate::perf::PERF_LAST.with(|last| {
            // Use the wrapper so non-wasm builds compile.
            let now = $crate::perf::perf_counter();
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
            $crate::log!(
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

#[macro_export]
macro_rules! perf_scope {
    ($($label:tt)*) => {
        // Format the label eagerly at scope entry.
        let __perf_label = format!($($label)*);

        // Create an RAII guard whose Drop implementation records the
        // performance measurement when the surrounding scope exits.
        //
        // We call the `defer` *function* directly (not the `defer!` macro)
        // to avoid fixed-name shadowing issues that would cause early drops
        // if multiple defers were introduced in the same scope.
        //
        // The guard is bound to a local variable so it remains alive until
        // the end of the enclosing scope (including across `.await` points
        // in async functions).
        let _perf_scope_guard = $crate::__reexports::defer::defer(move || {
            let __perf_end = $crate::perf::perf_counter();
            ::canic::log!(Info, "perf_scope defer: {}", __perf_end);

            $crate::perf::record(__perf_label.into(), __perf_end);
        });
    };
}
