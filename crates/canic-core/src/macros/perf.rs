/// Log elapsed instruction counts since the last `perf!` invocation.
///
/// Records the delta in instructions between calls and emits a
/// [`Log::Perf`](crate::Log::Perf)
/// entry with the provided label (any tokens accepted by `format!`). Use this
/// to highlight hot paths in long-running maintenance tasks.
#[macro_export]
macro_rules! perf {
    ($($label:tt)*) => {{
        $crate::perf::PERF_LAST.with(|last| {
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
