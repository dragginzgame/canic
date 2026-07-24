//! Module: api::fleet_activation
//!
//! Responsibility: expose protected Fleet activation diagnostics to endpoint callers.
//! Does not own: storage projection, phase validation, or controller authorization.
//! Boundary: maps the typed internal status failure into Canic's public error contract.

use crate::{
    dto::{
        error::Error,
        fleet_activation::{
            FleetActivationRequest, FleetActivationResumeRequest, FleetActivationStatusResponse,
            FleetCredentialGenerationRequest,
        },
    },
    view::fleet_activation::FleetActivationTransition,
    workflow::runtime::fleet_activation::FleetActivationWorkflow,
};

///
/// FleetActivationApi
///

pub struct FleetActivationApi;

impl FleetActivationApi {
    pub fn status() -> Result<FleetActivationStatusResponse, Error> {
        FleetActivationWorkflow::status().map_err(Error::from)
    }

    pub fn require_active() -> Result<(), Error> {
        FleetActivationWorkflow::require_active().map_err(Error::from)
    }

    pub async fn prepare_root() -> Result<FleetActivationStatusResponse, Error> {
        FleetActivationWorkflow::prepare_root()
            .await
            .map_err(Error::from)
    }

    pub async fn resume_root(
        request: FleetActivationResumeRequest,
    ) -> Result<FleetActivationTransition, Error> {
        FleetActivationWorkflow::resume_root(request)
            .await
            .map_err(Error::from)
    }

    pub fn prepare_nonroot_credential_generation(
        request: FleetCredentialGenerationRequest,
    ) -> Result<FleetActivationStatusResponse, Error> {
        FleetActivationWorkflow::prepare_nonroot_credential_generation(request).map_err(Error::from)
    }

    pub fn activate_nonroot(
        request: FleetActivationRequest,
    ) -> Result<FleetActivationTransition, Error> {
        FleetActivationWorkflow::activate_nonroot(request).map_err(Error::from)
    }
}
