//! Perf-instrumented timer helpers that auto-label with module + function name.
//!
//! These macros wrap [`TimerOps`](crate::ops::ic::timer::TimerOps) so callers can
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
        $crate::api::timer::set_lifecycle_timer($delay, label, $func($($($args)*)?))
    }};
}

///
/// timer_guarded
/// Schedule a one-shot timer if none is already scheduled for the slot.
/// Returns true when a new timer was scheduled.
///
/// # Examples
/// - `timer_guarded!(MY_TIMER, Duration::from_secs(5), do_cleanup);`
/// - `timer_guarded!(MY_TIMER, Duration::ZERO, my_task, arg1, arg2);`
///
#[macro_export]
macro_rules! timer_guarded {
    ($slot:path, $delay:expr, $func:path $(, $($args:tt)*)? ) => {{
        let label = concat!(module_path!(), "::", stringify!($func));
        $crate::api::timer::set_guarded(
            &$slot,
            $delay,
            label,
            $func($($($args)*)?),
        )
    }};
}

///
/// timer_interval
/// Schedule a repeating timer with an auto-generated label.
///
/// # Examples
/// - `timer_interval!(Duration::from_secs(60), heartbeat);`
/// - `timer_interval!(Duration::from_secs(10), tick, state.clone());`
///
#[macro_export]
macro_rules! timer_interval {
    ($interval:expr, $func:path $(, $($args:tt)*)? ) => {{
        let label = concat!(module_path!(), "::", stringify!($func));
        $crate::api::timer::set_interval(
            $interval,
            label,
            move || $func($($($args)*)?),
        )
    }};
}

///
/// timer_interval_guarded
/// Schedule an init timer that installs a repeating timer for the slot.
/// Returns true when a new timer was scheduled.
///
/// # Examples
/// - `timer_interval_guarded!(MY_TIMER, Duration::ZERO, init_task; Duration::from_secs(60), tick);`
/// - `timer_interval_guarded!(MY_TIMER, Duration::from_secs(2), init; Duration::from_secs(10), tick, state.clone());`
///
#[macro_export]
macro_rules! timer_interval_guarded {
    (
        $slot:path,
        $init_delay:expr,
        $init_func:path $(, $($init_args:tt)*)?
        ;
        $interval:expr,
        $tick_func:path $(, $($tick_args:tt)*)?
    ) => {{
        let init_label = concat!(module_path!(), "::", stringify!($init_func));
        let tick_label = concat!(module_path!(), "::", stringify!($tick_func));

        $crate::api::timer::set_guarded_interval(
            &$slot,
            $init_delay,
            init_label,
            move || $init_func($($($init_args)*)?),
            $interval,
            tick_label,
            move || $tick_func($($($tick_args)*)?),
        )
    }};
}
