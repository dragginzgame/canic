//! Module: api::fleet_admission_projection
//!
//! Responsibility: map managed-role projection calls into workflow results.
//! Does not own: authentication, stable state, policy compilation, or Candid dispatch.
//! Boundary: macro endpoints authenticate first and call this synchronous facade.

use crate::{
    dto::{
        error::Error,
        fleet_admission::{
            FleetAdmissionActivateTargetRequest, FleetAdmissionOpenTargetRequest,
            FleetAdmissionPrepareTargetRequest, FleetAdmissionProjectionStatusResponse,
            FleetAdmissionTargetReceipt,
        },
        page::PageRequest,
    },
    workflow::fleet_admission_projection::FleetAdmissionProjectionWorkflow,
};

/// Synchronous managed-role Fleet-admission projection facade.
pub struct FleetAdmissionProjectionApi;

impl FleetAdmissionProjectionApi {
    pub fn prepare(
        request: FleetAdmissionPrepareTargetRequest,
    ) -> Result<FleetAdmissionTargetReceipt, Error> {
        FleetAdmissionProjectionWorkflow::prepare(request).map_err(Into::into)
    }

    pub fn activate(
        request: FleetAdmissionActivateTargetRequest,
    ) -> Result<FleetAdmissionTargetReceipt, Error> {
        FleetAdmissionProjectionWorkflow::activate(request).map_err(Into::into)
    }

    pub fn open(
        request: FleetAdmissionOpenTargetRequest,
    ) -> Result<FleetAdmissionTargetReceipt, Error> {
        FleetAdmissionProjectionWorkflow::open(request).map_err(Into::into)
    }

    pub fn status(request: PageRequest) -> Result<FleetAdmissionProjectionStatusResponse, Error> {
        FleetAdmissionProjectionWorkflow::status(request).map_err(Into::into)
    }

    #[doc(hidden)]
    pub fn open_fresh() -> Result<bool, Error> {
        FleetAdmissionProjectionWorkflow::open_fresh().map_err(Into::into)
    }
}
