use crate::{api::Call, perf};
use std::future::Future;

/// Dispatch a synchronous query endpoint.
pub fn dispatch_query<R>(call: Call, f: impl FnOnce() -> R) -> R {
    perf::enter_endpoint();
    let result = f();
    perf::exit_endpoint(call);
    result
}

/// Dispatch an asynchronous query endpoint.
pub async fn dispatch_query_async<R, F>(call: Call, f: impl FnOnce() -> F) -> R
where
    F: Future<Output = R>,
{
    perf::enter_endpoint();
    let result = f().await;
    perf::exit_endpoint(call);
    result
}

/// Dispatch a synchronous update endpoint.
pub fn dispatch_update<R>(call: Call, f: impl FnOnce() -> R) -> R {
    perf::enter_endpoint();
    let result = f();
    perf::exit_endpoint(call);
    result
}

/// Dispatch an asynchronous update endpoint.
pub async fn dispatch_update_async<R, F>(call: Call, f: impl FnOnce() -> F) -> R
where
    F: Future<Output = R>,
{
    perf::enter_endpoint();
    let result = f().await;
    perf::exit_endpoint(call);
    result
}
