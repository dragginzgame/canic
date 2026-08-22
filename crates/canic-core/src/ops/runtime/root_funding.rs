//! Module: ops::runtime::root_funding
//!
//! Responsibility: bridge the sole core cycle-top-up owner to Root funding authority.
//! Does not own: funding policy, durable journals, Coordinator calls, or timer scheduling.
//! Boundary: a Root control plane registers one authority-owning driver before runtime start.

use crate::{
    InternalError,
    dto::fleet_funding::{FleetRootFundingRequest, FleetRootFundingResponse},
};
use async_trait::async_trait;
use std::sync::OnceLock;

/// Immutable scheduling inputs selected from one Root's protected funding authority.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RootFundingRuntimeConfig {
    pub request_threshold: u128,
    pub cooldown_secs: u64,
}

/// Driver implemented by the Root control plane while core retains timer ownership.
#[async_trait]
pub trait RootFundingRuntime: Send + Sync {
    /// Return the protected normal funding schedule.
    fn config(&self) -> Result<RootFundingRuntimeConfig, InternalError>;

    /// Validate and return the exact durable nonterminal request, if one exists.
    fn current_request(&self) -> Result<Option<FleetRootFundingRequest>, InternalError>;

    /// Persist the next request before its first outbound call.
    fn prepare_request(&self) -> Result<FleetRootFundingRequest, InternalError>;

    /// Invoke the exact Coordinator request without changing Root journal state.
    async fn request(
        &self,
        request: FleetRootFundingRequest,
    ) -> Result<FleetRootFundingResponse, InternalError>;

    /// Commit one exact terminal Coordinator response.
    fn record_response(
        &self,
        response: FleetRootFundingResponse,
    ) -> Result<FleetRootFundingResponse, InternalError>;
}

static ROOT_FUNDING_RUNTIME: OnceLock<&'static dyn RootFundingRuntime> = OnceLock::new();

/// Process-local Root funding driver registry used by the cycle workflow.
pub struct RootFundingRuntimeApi;

impl RootFundingRuntimeApi {
    /// Register the control-plane driver before Root lifecycle restoration.
    pub fn register(runtime: &'static dyn RootFundingRuntime) {
        let _ = ROOT_FUNDING_RUNTIME.set(runtime);
    }

    pub(crate) fn config() -> Result<RootFundingRuntimeConfig, InternalError> {
        runtime()?.config()
    }

    pub(crate) fn current_request() -> Result<Option<FleetRootFundingRequest>, InternalError> {
        runtime()?.current_request()
    }

    pub(crate) fn prepare_request() -> Result<FleetRootFundingRequest, InternalError> {
        runtime()?.prepare_request()
    }

    pub(crate) async fn request(
        request: FleetRootFundingRequest,
    ) -> Result<FleetRootFundingResponse, InternalError> {
        runtime()?.request(request).await
    }

    pub(crate) fn record_response(
        response: FleetRootFundingResponse,
    ) -> Result<FleetRootFundingResponse, InternalError> {
        runtime()?.record_response(response)
    }
}

fn runtime() -> Result<&'static dyn RootFundingRuntime, InternalError> {
    ROOT_FUNDING_RUNTIME
        .get()
        .copied()
        .ok_or_else(InternalError::lifecycle_failure)
}
