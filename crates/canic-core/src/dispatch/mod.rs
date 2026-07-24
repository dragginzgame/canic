//! Endpoint dispatch adapters.
//!
//! This module provides minimal wrappers used by macro-generated endpoints to
//! execute query and update handlers with consistent instrumentation.
//!
//! Responsibilities:
//! - Ensure runtime memory bootstrap readiness at endpoint boundary
//! - Enter and exit endpoint performance tracking
//! - Invoke the supplied handler closure
//! - Enforce the protected Fleet-activation phase before application dispatch
//! - Preserve synchronous vs asynchronous execution semantics
//!
//! This module contains no activation policy itself. It delegates the
//! cross-cutting phase decision to the runtime workflow before invoking the
//! endpoint handler.
//!
//! **DO NOT MERGE INTO WORKFLOW.**
//!
//! `dispatch` operates strictly at the *endpoint boundary*. It must remain a
//! thin adapter layer and must not:
//! - call application `ops` beyond minimal runtime bootstrap readiness
//! - call `storage`
//! - perform sequencing or lifecycle coordination
//! - duplicate activation or access policy
//!
//! All application behavior belongs in `api` or `workflow`, not here.

pub mod icrc21;

use crate::{ids::EndpointCall, perf};
use std::future::Future;

#[cfg_attr(not(target_arch = "wasm32"), expect(clippy::missing_const_for_fn))]
fn ensure_memory_bootstrap() {
    #[cfg(target_arch = "wasm32")]
    {
        if let Err(err) = crate::ops::runtime::memory::MemoryRegistryOps::ensure_bootstrap() {
            panic!("runtime memory bootstrap failed before endpoint dispatch: {err}");
        }
    }
}

fn enter_endpoint() {
    ensure_memory_bootstrap();
    perf::enter_endpoint();
}

/// Enforce cross-cutting endpoint prerequisites before access evaluation.
pub fn preflight_endpoint(call: EndpointCall) {
    ensure_memory_bootstrap();
    enforce_fleet_activation_fence(call);
}

#[cfg_attr(not(target_arch = "wasm32"), expect(clippy::missing_const_for_fn))]
fn enforce_fleet_activation_fence(call: EndpointCall) {
    #[cfg(target_arch = "wasm32")]
    if let Err(error) =
        crate::workflow::runtime::fleet_activation::FleetActivationWorkflow::require_endpoint_allowed(
            call,
        )
    {
        panic!(
            "Fleet activation fence rejected endpoint {}: {error}",
            call.endpoint.name
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = call;
        let _ = crate::workflow::runtime::fleet_activation::FleetActivationWorkflow::require_endpoint_allowed;
    }
}

/// Dispatch a synchronous query endpoint.
pub fn dispatch_query<R>(call: EndpointCall, f: impl FnOnce() -> R) -> R {
    enter_endpoint();
    let res = f();
    perf::exit_endpoint(call);

    res
}

/// Dispatch an asynchronous query endpoint.
pub async fn dispatch_query_async<R, F>(call: EndpointCall, f: impl FnOnce() -> F) -> R
where
    F: Future<Output = R>,
{
    enter_endpoint();
    let res = f().await;
    perf::exit_endpoint(call);

    res
}

/// Dispatch a synchronous update endpoint.
pub fn dispatch_update<R>(call: EndpointCall, f: impl FnOnce() -> R) -> R {
    enter_endpoint();
    let res = f();
    perf::exit_endpoint(call);

    res
}

/// Dispatch an asynchronous update endpoint.
pub async fn dispatch_update_async<R, F>(call: EndpointCall, f: impl FnOnce() -> F) -> R
where
    F: Future<Output = R>,
{
    enter_endpoint();
    let res = f().await;
    perf::exit_endpoint(call);

    res
}
