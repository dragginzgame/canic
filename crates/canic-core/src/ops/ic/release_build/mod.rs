//! Module: ops::ic::release_build
//!
//! Responsibility: expose the qualified Wasm's embedded release-build identity.
//! Does not own: build planning, install payloads, or activation persistence.
//! Boundary: lifecycle workflows use this facade before accepting install identity.

use crate::{
    InternalError,
    ids::ReleaseBuildId,
    infra::ic::{IcInfraError, release_build::ReleaseBuildInfra},
};

///
/// ReleaseBuildOps
///

pub struct ReleaseBuildOps;

impl ReleaseBuildOps {
    pub fn embedded_release_build_id() -> Result<ReleaseBuildId, InternalError> {
        ReleaseBuildInfra::embedded_release_build_id()
            .map_err(IcInfraError::from)
            .map_err(Into::into)
    }
}
