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
        return Err(Error::invalid(
            "capability envelope service must be Root for root dispatch",
        ));
    }

    if capability_version != CAPABILITY_VERSION_V1 {
        return Err(Error::invalid(format!(
            "unsupported capability_version: {capability_version}",
        )));
    }

    match proof {
        CapabilityProof::Structural => Ok(()),
    }
}
