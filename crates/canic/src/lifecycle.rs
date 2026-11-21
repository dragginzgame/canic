//! Explicit install and strict empty-argument reopen hooks.

#[cfg(all(feature = "metrics", any(feature = "db-store", feature = "db-schema")))]
impl crate::metrics::store::Source for ic_db::metrics::MetricsStorage {
    type Storage = Self;
}

#[cfg(all(feature = "metrics", feature = "entropy"))]
impl crate::metrics::store::Source for ic_entropy::MetricsStorage {
    type Storage = Self;
}

#[cfg(all(feature = "metrics", feature = "jobs"))]
impl crate::metrics::store::Source for ic_jobs::MetricsStorage {
    type Storage = Self;
}

#[cfg(all(feature = "metrics", feature = "timers"))]
impl crate::metrics::store::Source for ic_timers::MetricsStorage {
    type Storage = Self;
}

/// Initialize this canister's timer runtime before binding timers or restoring jobs.
/// With `metrics`, Timers writes to the shared metrics store. Repeated calls in
/// the same incarnation reuse its recorder. No callback or job is bound here.
///
/// # Errors
/// Propagates the timer provider's initialization error, including a conflicting recorder.
#[cfg(feature = "timers")]
pub fn initialize_timers() -> Result<(), ic_timers::Error> {
    #[cfg(feature = "metrics")]
    {
        use ic_timers::{MetricsStorage, initialize_with_metrics};

        initialize_with_metrics(crate::metrics::store::open::<MetricsStorage>())?;
        crate::metrics::start_timer_window();
        Ok(())
    }
    #[cfg(not(feature = "metrics"))]
    {
        ic_timers::initialize()
    }
}

/// Attach the selected suppliers to this canister’s diagnostics before application work.
/// Canic lifecycle mounts call this once on install and on reopen. Actors with
/// custom lifecycle entrypoints call it themselves at the same boundary.
///
/// # Panics
/// Traps if an enabled supplier has already selected a different recorder or log sink.
pub fn initialize_diagnostics() {
    #[cfg(feature = "logging")]
    crate::logging::initialize();

    #[cfg(all(feature = "logging", any(feature = "db-store", feature = "db-schema")))]
    ic_db::logging::initialize_with_logger(crate::logging::record).expect("attach DB log sink");

    #[cfg(all(feature = "metrics", any(feature = "db-store", feature = "db-schema")))]
    {
        use ic_db::metrics::{MetricsStorage, initialize_with_metrics};
        initialize_with_metrics(crate::metrics::store::open::<MetricsStorage>())
            .expect("attach DB metrics");
    }

    #[cfg(all(feature = "logging", feature = "entropy"))]
    ic_entropy::set_log_sink(Some(crate::logging::record)).expect("attach Entropy log sink");

    #[cfg(all(feature = "metrics", feature = "entropy"))]
    ic_entropy::initialize_with_metrics(crate::metrics::store::open::<
        ic_entropy::MetricsStorage,
    >())
    .expect("attach Entropy metrics");

    #[cfg(all(feature = "metrics", feature = "jobs"))]
    ic_jobs::initialize_with_metrics(crate::metrics::store::open::<ic_jobs::MetricsStorage>())
        .expect("attach Jobs metrics");
}

#[doc(hidden)]
pub fn empty_arguments(bytes: &[u8]) -> Result<(), &'static str> {
    bytes
        .is_empty()
        .then_some(())
        .ok_or("expected empty lifecycle arguments")
}

/// Mount zero-argument install and reopen handlers, or supply a bounded install decoder and a typed install handler.
/// The reopen handler takes no arguments. A custom upgrade payload can use Platform's
/// entrypoints directly. Panics in either handler trap the lifecycle message.
// Rustfmt 1.100 repeatedly reindents multiline attributes in this macro body.
#[rustfmt::skip]
#[macro_export]
macro_rules! mount_lifecycle {
    (input = $input:ty, max_bytes = $max:literal, decode = $decode:path,
     install = $install:path, reopen = $reopen:path $(,)?) => {
        $crate::mount_lifecycle!(@hooks input: $input, $max, $decode, $install(input), $reopen);
    };
    (install = $install:path, reopen = $reopen:path $(,)?) => {
        $crate::mount_lifecycle!(@hooks (): (), 0, $crate::lifecycle::empty_arguments, $install(), $reopen);
    };
    (@hooks $binding:tt: $input:ty, $max:literal, $decode:path,
     $call:expr, $reopen:path) => {
        const _: () = {
            use $crate::__private::runtime as platform;

            fn __canic_admit() -> Result<(), std::convert::Infallible> {
                Ok(())
            }

            fn __canic_fail(error: impl std::fmt::Debug) -> ! {
                platform::system::trap(&format!("lifecycle refused: {error:?}"))
            }

            #[platform::init(
                runtime = platform,
                guard = __canic_admit,
                decode = $decode,
                max_bytes = $max,
                on_error = __canic_fail,
            )]
            #[allow(clippy::needless_pass_by_value)]
            fn __canic_init($binding: $input) {
                $crate::lifecycle::initialize_diagnostics();
                $call;
            }

            #[platform::post_upgrade(
                runtime = platform,
                guard = __canic_admit,
                decode = $crate::lifecycle::empty_arguments,
                max_bytes = 0,
                on_error = __canic_fail,
            )]
            fn __canic_post_upgrade((): ()) {
                $crate::lifecycle::initialize_diagnostics();
                $reopen();
            }
        };
    };
}
