//! Module: workflow::rpc::capability::envelope
//!
//! Responsibility: validate capability envelope wire headers.
//! Does not own: proof verification, request dispatch, or replay metadata projection.
//! Boundary: validates the current capability DTO header before proof checks.

use crate::dto::{
    capability::{CAPABILITY_VERSION_V1, CapabilityProof, CapabilityService},
    error::Error,
};

pub(super) fn validate_root_capability_envelope(
    service: CapabilityService,
    capability_version: u16,
    proof: &CapabilityProof,
) -> Result<(), Error> {
    if service != CapabilityService::Root {
        return Err(Error::from_registered(
            crate::diagnostics::codes::REQUEST_INVALID,
        ));
    }

    if capability_version != CAPABILITY_VERSION_V1 {
        return Err(Error::from_registered(
            crate::diagnostics::codes::REQUEST_INVALID,
        ));
    }

    match proof {
        CapabilityProof::Structural => Ok(()),
    }
}
