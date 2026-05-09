use crate::dto::{
    capability::{CAPABILITY_VERSION_V1, CapabilityProof, CapabilityService},
    error::Error,
};

use super::RootCapabilityProof;

pub(super) fn validate_root_capability_envelope(
    service: CapabilityService,
    capability_version: u16,
    proof: &CapabilityProof,
) -> Result<RootCapabilityProof<'_>, Error> {
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

    RootCapabilityProof::validate(proof)
}
