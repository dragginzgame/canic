use crate::perf;
use std::future::Future;

/// Dispatch a synchronous query endpoint.
pub fn dispatch_query<R>(label: &'static str, f: impl FnOnce() -> R) -> R {
    perf::enter_endpoint();
    let result = f();
    perf::exit_endpoint(label);
    result
}

/// Dispatch an asynchronous query endpoint.
pub async fn dispatch_query_async<R, F>(label: &'static str, f: impl FnOnce() -> F) -> R
where
    F: Future<Output = R>,
{
    perf::enter_endpoint();
    let result = f().await;
    perf::exit_endpoint(label);
    result
}

/// Dispatch a synchronous update endpoint.
pub fn dispatch_update<R>(label: &'static str, f: impl FnOnce() -> R) -> R {
    perf::enter_endpoint();
    let result = f();
    perf::exit_endpoint(label);
    result
}

/// Dispatch an asynchronous update endpoint.
pub async fn dispatch_update_async<R, F>(label: &'static str, f: impl FnOnce() -> F) -> R
where
    F: Future<Output = R>,
{
    perf::enter_endpoint();
    let result = f().await;
    perf::exit_endpoint(label);
    result
}
