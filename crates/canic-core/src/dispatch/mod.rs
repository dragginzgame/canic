pub mod icrc21;

use crate::{api::EndpointCall, perf};
use std::future::Future;

/// Dispatch a synchronous query endpoint.
pub fn dispatch_query<R>(call: EndpointCall, f: impl FnOnce() -> R) -> R {
    perf::enter_endpoint();
    let res = f();
    perf::exit_endpoint(call);

    res
}

/// Dispatch an asynchronous query endpoint.
pub async fn dispatch_query_async<R, F>(call: EndpointCall, f: impl FnOnce() -> F) -> R
where
    F: Future<Output = R>,
{
    perf::enter_endpoint();
    let res = f().await;
    perf::exit_endpoint(call);

    res
}

/// Dispatch a synchronous update endpoint.
pub fn dispatch_update<R>(call: EndpointCall, f: impl FnOnce() -> R) -> R {
    perf::enter_endpoint();
    let res = f();
    perf::exit_endpoint(call);

    res
}

/// Dispatch an asynchronous update endpoint.
pub async fn dispatch_update_async<R, F>(call: EndpointCall, f: impl FnOnce() -> F) -> R
where
    F: Future<Output = R>,
{
    perf::enter_endpoint();
    let res = f().await;
    perf::exit_endpoint(call);

    res
}
