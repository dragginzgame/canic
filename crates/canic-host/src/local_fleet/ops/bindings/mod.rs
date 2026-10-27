//! Verify retained native schemas before allocating a local Fleet.

use crate::{fleet_ensure::model::DesiredFleet, local_fleet::LocalFleetError, release_set};
use std::path::Path;

fn failure(error: impl std::fmt::Display) -> LocalFleetError {
    LocalFleetError::Preparation(error.to_string())
}

/// Admit the selected release's schemas without copying a parallel ICP binding tree.
pub fn verify(workspace: &Path, desired: &DesiredFleet) -> Result<(), LocalFleetError> {
    let bootstrap = desired
        .bootstrap
        .as_ref()
        .ok_or(LocalFleetError::Identity)?;
    let protocol = desired.protocol.as_ref().ok_or(LocalFleetError::Identity)?;
    let release = bootstrap.release_build_id;
    let current = release_set::load_persisted_current_release_set_manifest(workspace, release)
        .map_err(failure)?;
    let infrastructure =
        release_set::load_persisted_canic_infrastructure_artifact_manifest(workspace, release)
            .map_err(failure)?;
    let config = release_set::AppConfigSnapshot::load(&workspace.join(&protocol.app_config))
        .map_err(failure)?;
    let deployment = config
        .model()
        .compile_component_deployment_configuration()
        .map_err(failure)?;
    let application = release_set::load_persisted_application_artifact_union(
        workspace,
        &deployment.component_topology,
        release,
    )
    .map_err(failure)?;
    if infrastructure.digest != current.manifest.infrastructure_artifact_manifest_sha256
        || application.digest != current.manifest.application_artifact_union_sha256
    {
        return Err(LocalFleetError::Identity);
    }
    for (role, expected) in infrastructure
        .manifest
        .entries
        .iter()
        .map(|entry| (entry.protocol_role.as_str(), entry.wire_schema_sha256))
        .chain(
            application
                .union
                .entries
                .iter()
                .map(|entry| (entry.role.as_str(), entry.wire_schema_sha256)),
        )
    {
        let schema =
            release_set::load_built_wire_schema(workspace, release, role).map_err(failure)?;
        if schema.schema_sha256 != expected {
            return Err(LocalFleetError::Identity);
        }
    }
    Ok(())
}
