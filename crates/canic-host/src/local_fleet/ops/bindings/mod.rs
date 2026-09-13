//! Materialize exact sealed Candid into the maintained selected-environment binding layout.

use crate::{
    durable_io, fleet_ensure::model::DesiredFleet, local_fleet::LocalFleetError, release_set,
};
use canic_core::cdk::utils::hash::{hex_bytes, sha256_hex};
use std::path::Path;

fn failure(error: impl std::fmt::Display) -> LocalFleetError {
    LocalFleetError::Preparation(error.to_string())
}

/// Reuse the complete selected release manifests; a missing or altered sidecar never gets rebuilt.
pub fn stage(workspace: &Path, desired: &DesiredFleet) -> Result<(), LocalFleetError> {
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
    for entry in &infrastructure.manifest.entries {
        publish(
            workspace,
            &desired.environment,
            entry.protocol_role.as_str(),
            &entry.wasm_relative_path,
            entry.candid_sha256,
        )?;
    }
    for entry in &application.union.entries {
        publish(
            workspace,
            &desired.environment,
            entry.role.as_str(),
            &entry.wasm_relative_path,
            entry.candid_sha256,
        )?;
    }
    Ok(())
}

fn publish(
    workspace: &Path,
    environment: &str,
    role: &str,
    wasm: &str,
    digest: [u8; 32],
) -> Result<(), LocalFleetError> {
    crate::component_operation::policy::validate_label(role)
        .map_err(|_| LocalFleetError::Identity)?;
    release_set::validate_release_artifact_relative_path(wasm).map_err(failure)?;
    let bytes = durable_io::read_regular_bytes(
        &workspace.join(wasm).with_extension("did"),
        crate::frontend::ops::MAX_FRONTEND_FILE_BYTES,
    )?;
    if sha256_hex(&bytes) != hex_bytes(digest) {
        return Err(LocalFleetError::Identity);
    }
    let destination = crate::icp::local_canister_candid_path(workspace, environment, role);
    durable_io::write_bytes(&destination, &bytes)?;
    Ok(())
}
