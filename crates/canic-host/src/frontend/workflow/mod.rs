//! Export orchestration over the selected terminal Fleet review.
//!
//! Network effects and static-asset deployment are not part of this workflow.

use crate::{
    fleet_ensure::ops::{EnsurePaths, lock_operation},
    frontend::{
        FrontendError,
        model::{FrontendEnvironmentInput, FrontendManifestRecord},
        ops, policy,
    },
};
use std::path::Path;

/// Export an explicit selection from one stable terminal review under its Fleet operation lock.
pub fn prepare_handoff(
    root: &Path,
    fleet: &str,
    input: &FrontendEnvironmentInput,
    directory: &Path,
) -> Result<FrontendManifestRecord, FrontendError> {
    crate::component_operation::policy::validate_label(&input.environment)
        .map_err(|_| FrontendError::Environment)?;
    crate::component_operation::policy::validate_label(fleet)
        .map_err(|_| FrontendError::Environment)?;
    let _lock = lock_operation(&EnsurePaths::under(root, &input.environment, fleet))?;
    let authority = ops::authority(root, &input.environment, fleet)?;
    policy::validate_input(
        input,
        &input.environment,
        authority.network,
        authority.admission_nonempty,
        authority.admission_origin.as_deref(),
    )?;
    let bundle = ops::prepare_bundle(root, input, authority)?;
    ops::publish_bundle(directory, &bundle)?;
    Ok(bundle.manifest)
}
