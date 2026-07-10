use std::path::Path;

use crate::{
    evidence_envelope::{InputFingerprintV1, PayloadSchemaRefV1, file_input_fingerprint},
    role_contract::{declared_role_manifest_path, finding_detail},
};

use super::model::BuildProvenanceRequest;

pub(super) fn build_input_fingerprints(
    request: &BuildProvenanceRequest,
) -> Result<Vec<InputFingerprintV1>, Box<dyn std::error::Error>> {
    let role = canic_core::ids::CanisterRole::owned(request.role.clone());
    let package_manifest = declared_role_manifest_path(&request.config_path, &role)
        .map_err(|finding| finding_detail(&finding))?;
    let mut inputs = vec![file_input_fingerprint(
        "cargo_package_manifest",
        &package_manifest,
        &request.workspace_root,
        Some(PayloadSchemaRefV1::internal(
            "cargo.package_manifest.toml",
            "1",
        )),
        None,
    )?];
    let cargo_lock_path = request.workspace_root.join("Cargo.lock");
    if cargo_lock_path.is_file() {
        inputs.push(file_input_fingerprint(
            "cargo_lock",
            &cargo_lock_path,
            &request.workspace_root,
            Some(PayloadSchemaRefV1::internal("cargo.lock", "1")),
            None,
        )?);
    }
    inputs.extend(cargo_config_fingerprints(&request.workspace_root)?);
    Ok(inputs)
}

pub(super) fn cargo_config_fingerprints(
    workspace_root: &Path,
) -> Result<Vec<InputFingerprintV1>, Box<dyn std::error::Error>> {
    [".cargo/config.toml", ".cargo/config"]
        .into_iter()
        .map(|relative| workspace_root.join(relative))
        .filter(|path| path.is_file())
        .map(|path| {
            Ok(file_input_fingerprint(
                "cargo_config",
                &path,
                workspace_root,
                Some(PayloadSchemaRefV1::internal("cargo.config.toml", "1")),
                None,
            )?)
        })
        .collect()
}
