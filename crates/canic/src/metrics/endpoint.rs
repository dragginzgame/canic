//! One guard precedes decoding and dispatch for every metrics source.
use super::{ReadMetrics, Report, Request, Snapshot};
use ic_platform_runtime::system;
use std::cell::OnceCell;

type Guard = fn() -> Result<(), &'static str>;

thread_local! {
    static GUARD: OnceCell<Guard> = const { OnceCell::new() };
}

/// Select one read guard during install and again on reopen. The default admits controllers.
/// # Panics
/// A second registration in the same incarnation is a lifecycle error.
pub fn set_guard(guard: fn() -> Result<(), &'static str>) {
    GUARD.with(|slot| assert!(slot.set(guard).is_ok(), "metrics guard already set"));
}

fn admit() -> Result<(), &'static str> {
    GUARD.with(|slot| match slot.get() {
        Some(guard) => guard(),
        None => system::is_controller(&system::caller())
            .then_some(())
            .ok_or("controller required"),
    })
}

// metrics
// Read one selected source from this canister without initializing a recorder.
// guard  : application metrics guard (controller by default)
// limits : ReadMetrics::REQUEST and ReadMetrics::REPLY
#[crate::query(contract = ReadMetrics, guard = admit)]
fn metrics(request: Request) -> Report {
    match request {
        Request::Timers => Report::Timers(Snapshot {
            observed_at_ns: system::time(),
            #[cfg(feature = "timers")]
            timers: timers(),
            #[cfg(not(feature = "timers"))]
            timers: None,
        }),
        #[cfg(feature = "endpoint-metrics")]
        Request::Endpoints(after) => Report::Endpoints(super::performance::page(after.as_deref())),
        #[cfg(not(feature = "endpoint-metrics"))]
        Request::Endpoints(_) => Report::Unavailable,
        #[cfg(feature = "blob")]
        Request::Blob => blob(),
        #[cfg(any(feature = "db-schema", feature = "db-store"))]
        Request::Db => super::store::get::<ic_db::metrics::MetricsStorage>().map_or(
            Report::Unavailable,
            |store| {
                let m = store.snapshot();
                Report::Db {
                    queries: m.queries.into(),
                    query_failures: m.query_failures.into(),
                    examined: m.examined.into(),
                    fetched: m.fetched.into(),
                    returned: m.returned.into(),
                    output_bytes: m.output_bytes.into(),
                    commits: m.commits.into(),
                    commit_failures: m.commit_failures.into(),
                    job_starts: m.job_starts.into(),
                    job_resumes: m.job_resumes.into(),
                    job_failures: m.job_failures.into(),
                }
            },
        ),
        #[cfg(feature = "jobs")]
        Request::Jobs => {
            super::store::get::<ic_jobs::MetricsStorage>().map_or(Report::Unavailable, |store| {
                let m = store.snapshot();
                Report::Jobs {
                    started: m.started.into(),
                    retry_attempts: m.retry_attempts.into(),
                    skipped_late: m.skipped_late.into(),
                    no_work: m.no_work.into(),
                    succeeded: m.succeeded.into(),
                    expected_failures: m.expected_failures.into(),
                    retryable_failures: m.retryable_failures.into(),
                }
            })
        }
        #[cfg(feature = "entropy")]
        Request::Entropy => {
            super::store::get::<ic_entropy::MetricsStorage>().map_or(Report::Unavailable, |store| {
                let m = store.snapshot().fresh;
                Report::Entropy {
                    attempts: m.attempts.into(),
                    succeeded: m.succeeded.into(),
                    failed: m.failed.into(),
                    cancelled: m.cancelled.into(),
                }
            })
        }
        #[cfg(not(feature = "blob"))]
        Request::Blob => Report::Unavailable,
        #[cfg(not(any(feature = "db-schema", feature = "db-store")))]
        Request::Db => Report::Unavailable,
        #[cfg(not(feature = "jobs"))]
        Request::Jobs => Report::Unavailable,
        #[cfg(not(feature = "entropy"))]
        Request::Entropy => Report::Unavailable,
    }
}

#[cfg(feature = "timers")]
thread_local! {
    static WINDOW: OnceCell<(u64, u64)> = const { OnceCell::new() };
}

#[cfg(feature = "timers")]
pub(crate) fn start_timer_window() {
    WINDOW.with(|w| {
        w.get_or_init(|| (system::canister_version(), system::time()));
    });
}

#[cfg(feature = "timers")]
fn timers() -> Option<super::Timers> {
    use super::{Timers, store};
    let (started_at_version, started_at_ns) = WINDOW.with(|window| window.get().copied())?;
    store::get::<ic_timers::MetricsStorage>().map(|storage| {
        let m = storage.snapshot();
        let [
            accepted,
            completed,
            abandoned,
            expected_failures,
            deferred_submissions,
            provider_faults,
        ] = [
            m.accepted,
            m.completed,
            m.abandoned,
            m.expected_failures,
            m.deferred_submissions,
            m.provider_faults,
        ]
        .map(Into::into);
        Timers {
            started_at_version,
            started_at_ns,
            accepted,
            completed,
            abandoned,
            expected_failures,
            deferred_submissions,
            provider_faults,
        }
    })
}

#[cfg(feature = "blob")]
fn blob() -> Report {
    use super::{BlobCommand, Counter, store};
    use ic_blob_storage::metrics::{CommandKind, MetricsStorage};
    let Some(storage) = store::get::<MetricsStorage>() else {
        return Report::Unavailable;
    };
    let snapshot = storage.snapshot();
    let rows = [
        (CommandKind::Authorize, "authorize"),
        (CommandKind::Register, "register"),
        (CommandKind::Retain, "retain"),
        (CommandKind::Release, "release"),
        (CommandKind::Expire, "expire"),
        (CommandKind::Drain, "drain"),
        (CommandKind::Activate, "activate"),
    ]
    .into_iter()
    .map(|(kind, name)| {
        let m = snapshot.commands(kind);
        let [succeeded, denied, capacity, expired, other_refusals] = [
            m.succeeded,
            m.denied,
            m.capacity,
            m.expired,
            m.other_refusals,
        ]
        .map(Into::into);
        BlobCommand {
            command: name.into(),
            succeeded,
            denied,
            capacity,
            expired,
            other_refusals,
        }
    })
    .collect();
    let histogram = snapshot.registered_bytes;
    let sizes = super::BlobSizes {
        upper_bounds: *histogram.upper_bounds(),
        buckets: std::array::from_fn(|i| Counter {
            total: histogram.bucket_counts()[i].into(),
            overflowed: histogram.bucket_overflowed()[i],
        }),
        above: Counter {
            total: histogram.overflow().into(),
            overflowed: histogram.overflow_count_overflowed(),
        },
    };
    Report::Blob {
        commands: rows,
        sizes,
    }
}
