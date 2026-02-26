//! Endpoint dispatch adapters.
//!
//! This module provides minimal wrappers used by macro-generated endpoints to
//! execute query and update handlers with consistent instrumentation.
//!
//! Responsibilities:
//! - Ensure runtime memory bootstrap readiness at endpoint boundary
//! - Enter and exit endpoint performance tracking
//! - Invoke the supplied handler closure
//! - Preserve synchronous vs asynchronous execution semantics
//!
//! This module intentionally contains **no business logic**, **no policy
//! enforcement**, and **no orchestration**. It exists solely to adapt endpoint
//! functions to shared cross-cutting concerns (currently performance tracking).
//!
//! **DO NOT MERGE INTO WORKFLOW.**
//!
//! `dispatch` operates strictly at the *endpoint boundary*. It must remain a
//! thin adapter layer and must not:
//! - call `ops`
//! - call `storage`
//! - perform sequencing or lifecycle coordination
//! - enforce access, domain, or policy rules
//!
//! All application behavior belongs in `api` or `workflow`, not here.

pub mod icrc21;

use crate::{ids::EndpointCall, perf};
use std::future::Future;

#[allow(clippy::missing_const_for_fn)]
fn ensure_memory_bootstrap() {
    #[cfg(target_arch = "wasm32")]
    {
        if let Err(err) = crate::ops::runtime::memory::MemoryRegistryOps::ensure_bootstrap() {
            panic!("runtime memory bootstrap failed before endpoint dispatch: {err}");
        }
    }
}

/// Dispatch a synchronous query endpoint.
pub fn dispatch_query<R>(call: EndpointCall, f: impl FnOnce() -> R) -> R {
    ensure_memory_bootstrap();
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
    ensure_memory_bootstrap();
    perf::enter_endpoint();
    let res = f().await;
    perf::exit_endpoint(call);

    res
}

/// Dispatch a synchronous update endpoint.
pub fn dispatch_update<R>(call: EndpointCall, f: impl FnOnce() -> R) -> R {
    ensure_memory_bootstrap();
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
    ensure_memory_bootstrap();
    perf::enter_endpoint();
    let res = f().await;
    perf::exit_endpoint(call);

    res
}
