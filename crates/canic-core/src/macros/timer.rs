//! Perf-instrumented timer helpers that auto-label with module + function name.
//!
//! These macros wrap [`TimerOps`](crate::ops::timer::TimerOps) so callers can
//! schedule work without manually threading labels. Labels are constructed
//! as `module_path!()::function_name`.

///
/// timer
/// Schedule a one-shot timer with an auto-generated label.
///
/// # Examples
/// - `timer!(Duration::from_secs(5), do_cleanup);`
/// - `timer!(Duration::ZERO, my_task, arg1, arg2);`
///
#[macro_export]
macro_rules! timer {
    ($delay:expr, $func:path $(, $($args:tt)*)? ) => {{
        let label = concat!(module_path!(), "::", stringify!($func));
        $crate::ops::ic::timer::TimerOps::set($delay, label, $func($($($args)*)?))
    }};
}

///
/// timer_interval
/// Schedule a repeating timer with an auto-generate label.
///
/// # Examples
/// - `timer_interval!(Duration::from_secs(60), heartbeat);`
/// - `timer_interval!(Duration::from_secs(10), tick, state.clone());`
///
#[macro_export]
macro_rules! timer_interval {
    ($interval:expr, $func:path $(, $($args:tt)*)? ) => {{
        let label = concat!(module_path!(), "::", stringify!($func));
        $crate::ops::ic::timer::TimerOps::set_interval($interval, label, move || $func($($($args)*)?))
    }};
}
