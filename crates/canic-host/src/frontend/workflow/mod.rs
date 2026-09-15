//! Frontend handoff export and read-only asset verification orchestration.
//!
//! Export uses terminal Fleet authority; verification queries uploaded assets without mutation.

mod uploaded;

use crate::{
    fleet_ensure::ops::{EnsurePaths, lock_operation},
    frontend::{
        FrontendError,
        model::{FrontendEnvironmentInput, FrontendManifestRecord},
        ops, policy,
    },
};
use std::path::Path;

pub use uploaded::verify_uploaded;

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
