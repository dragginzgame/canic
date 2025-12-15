use crate::perf_scope;
use std::future::Future;

/// Dispatch a synchronous query endpoint.
pub fn dispatch_query<R>(label: &'static str, f: impl FnOnce() -> R) -> R {
    perf_scope!("{label}");
    f()
}

/// Dispatch an asynchronous query endpoint.
pub async fn dispatch_query_async<R, F>(label: &'static str, f: impl FnOnce() -> F) -> R
where
    F: Future<Output = R>,
{
    perf_scope!("{label}");
    f().await
}

/// Dispatch a synchronous update endpoint.
pub fn dispatch_update<R>(label: &'static str, f: impl FnOnce() -> R) -> R {
    perf_scope!("{label}");
    f()
}

/// Dispatch an asynchronous update endpoint.
pub async fn dispatch_update_async<R, F>(label: &'static str, f: impl FnOnce() -> F) -> R
where
    F: Future<Output = R>,
{
    perf_scope!("{label}");
    f().await
}
