//! Module: workflow::rpc::capability::envelope
//!
//! Responsibility: validate capability envelope wire headers.
//! Does not own: proof verification, request dispatch, or replay metadata projection.
//! Boundary: maps capability DTO header fields into typed proof views.

use crate::{
    dto::{
        capability::{CAPABILITY_VERSION_V1, CapabilityProof, CapabilityService},
        error::Error,
    },
    workflow::rpc::capability::RootCapabilityProof,
};

pub(super) fn validate_root_capability_envelope(
    service: CapabilityService,
    capability_version: u16,
    proof: &CapabilityProof,
) -> Result<RootCapabilityProof, Error> {
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

    Ok(RootCapabilityProof::validate(proof))
}
