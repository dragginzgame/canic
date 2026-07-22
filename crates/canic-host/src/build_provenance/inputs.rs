use std::path::Path;

use crate::evidence_envelope::{
    InputFingerprintV1, InputPathDisplayV1, PayloadSchemaRefV1, file_input_fingerprint,
};

use super::model::BuildProvenanceRequest;

pub(super) fn build_input_fingerprints(
    request: &BuildProvenanceRequest,
    package_manifest: &Path,
) -> Result<Vec<InputFingerprintV1>, Box<dyn std::error::Error>> {
    let mut inputs = vec![file_input_fingerprint(
        "cargo_package_manifest",
        package_manifest,
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
    inputs.push(InputFingerprintV1 {
        kind: "build_network".to_string(),
        path: None,
        path_display: InputPathDisplayV1::Omitted,
        sha256: None,
        size_bytes: None,
        modified_unix_secs: None,
        schema: Some(PayloadSchemaRefV1::internal("canic.build_network", "1")),
        note: Some(format!(
            "environment={};build_network={}",
            request.environment, request.build_network
        )),
    });
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
