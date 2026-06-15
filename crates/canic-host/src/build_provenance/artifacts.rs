use std::path::Path;

use crate::evidence_envelope::file_input_fingerprint;

use super::model::{ArtifactProvenanceKindV1, ArtifactProvenanceV1, BuildProvenanceRequest};

pub(super) fn artifact_provenance(
    request: &BuildProvenanceRequest,
) -> Result<Vec<ArtifactProvenanceV1>, Box<dyn std::error::Error>> {
    let mut artifacts = Vec::new();
    push_artifact(
        &mut artifacts,
        request,
        ArtifactProvenanceKindV1::Wasm,
        &request.output.wasm_path,
    )?;
    push_artifact(
        &mut artifacts,
        request,
        ArtifactProvenanceKindV1::WasmGzip,
        &request.output.wasm_gz_path,
    )?;
    push_existing_artifact(
        &mut artifacts,
        request,
        ArtifactProvenanceKindV1::Candid,
        &request.output.did_path,
    )?;
    if let Some(path) = &request.output.manifest_path {
        push_existing_artifact(
            &mut artifacts,
            request,
            ArtifactProvenanceKindV1::Metadata,
            path,
        )?;
    }

    Ok(artifacts)
}

fn push_existing_artifact(
    artifacts: &mut Vec<ArtifactProvenanceV1>,
    request: &BuildProvenanceRequest,
    kind: ArtifactProvenanceKindV1,
    path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    if path.is_file() {
        push_artifact(artifacts, request, kind, path)?;
    }
    Ok(())
}

fn push_artifact(
    artifacts: &mut Vec<ArtifactProvenanceV1>,
    request: &BuildProvenanceRequest,
    kind: ArtifactProvenanceKindV1,
    path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let fingerprint =
        file_input_fingerprint("build_artifact", path, &request.workspace_root, None, None)?;
    artifacts.push(ArtifactProvenanceV1 {
        role: request.role.clone(),
        fleet: request.fleet.clone(),
        artifact_kind: kind,
        path: fingerprint.path,
        path_display: fingerprint.path_display,
        hash_algorithm: "sha256".to_string(),
        sha256: fingerprint
            .sha256
            .ok_or_else(|| format!("missing sha256 for {}", path.display()))?,
        size_bytes: fingerprint
            .size_bytes
            .ok_or_else(|| format!("missing size for {}", path.display()))?,
        produced_by: "canic build".to_string(),
    });
    Ok(())
}
