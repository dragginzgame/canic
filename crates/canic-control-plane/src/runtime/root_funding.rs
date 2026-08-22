//! Module: runtime::root_funding
//!
//! Responsibility: adapt Root funding authority to the core-owned cycle timer.
//! Does not own: scheduling, journal transitions, protected policy, or Coordinator protocol.
//! Boundary: lifecycle registers this zero-state driver before core runtime restoration.

use async_trait::async_trait;
use canic_core::{
    api::runtime::root_funding::{
        RootFundingRuntime, RootFundingRuntimeApi, RootFundingRuntimeConfig,
    },
    control_plane_support::error::InternalError,
    dto::fleet_funding::{FleetRootFundingRequest, FleetRootFundingResponse},
};

struct ControlPlaneRootFundingRuntime;

#[async_trait]
impl RootFundingRuntime for ControlPlaneRootFundingRuntime {
    fn config(&self) -> Result<RootFundingRuntimeConfig, InternalError> {
        let schedule = crate::workflow::root_funding::schedule()?;
        Ok(RootFundingRuntimeConfig {
            request_threshold: schedule.request_threshold,
            cooldown_secs: schedule.cooldown_secs,
        })
    }

    fn current_request(&self) -> Result<Option<FleetRootFundingRequest>, InternalError> {
        crate::workflow::root_funding::current_request()
    }

    fn prepare_request(&self) -> Result<FleetRootFundingRequest, InternalError> {
        crate::workflow::root_funding::prepare_request()
    }

    async fn request(
        &self,
        request: FleetRootFundingRequest,
    ) -> Result<FleetRootFundingResponse, InternalError> {
        crate::workflow::root_funding::request_coordinator(request).await
    }

    fn record_response(
        &self,
        response: FleetRootFundingResponse,
    ) -> Result<FleetRootFundingResponse, InternalError> {
        crate::workflow::root_funding::record_response(response)
    }
}

static ROOT_FUNDING_RUNTIME: ControlPlaneRootFundingRuntime = ControlPlaneRootFundingRuntime;

/// Register the protected Root driver consumed by the core cycle owner.
pub fn register() {
    RootFundingRuntimeApi::register(&ROOT_FUNDING_RUNTIME);
}
