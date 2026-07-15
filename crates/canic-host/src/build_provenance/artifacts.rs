use std::path::Path;

use crate::evidence_envelope::file_input_fingerprint;

use crate::canister_build::{
    ArtifactTransformKind, ArtifactTransformMode, ArtifactTransformOutcome,
};

use super::model::{
    ArtifactProvenanceKindV1, ArtifactProvenanceV1, ArtifactTransformKindV1,
    ArtifactTransformModeV1, ArtifactTransformOutcomeV1, ArtifactTransformProvenanceV1,
    BuildProvenanceRequest,
};

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
    Ok(artifacts)
}

pub(super) fn artifact_transform_provenance(
    request: &BuildProvenanceRequest,
) -> Result<Vec<ArtifactTransformProvenanceV1>, Box<dyn std::error::Error>> {
    request
        .output
        .transforms
        .iter()
        .map(|transform| {
            if transform.role.trim().is_empty() {
                return Err("artifact transform role must not be empty".into());
            }
            if transform.tool.trim().is_empty() {
                return Err("artifact transform tool must not be empty".into());
            }
            match transform.outcome {
                ArtifactTransformOutcome::Applied
                    if transform
                        .tool_version
                        .as_deref()
                        .is_none_or(|version| version.trim().is_empty()) =>
                {
                    return Err("applied artifact transform must record a tool version".into());
                }
                ArtifactTransformOutcome::ToolUnavailable
                | ArtifactTransformOutcome::NotRequested
                    if transform.tool_version.is_some() =>
                {
                    return Err(
                        "unapplied artifact transform must not record a tool version".into(),
                    );
                }
                _ => {}
            }
            Ok(ArtifactTransformProvenanceV1 {
                role: transform.role.clone(),
                transform: match transform.transform {
                    ArtifactTransformKind::Shrink => ArtifactTransformKindV1::Shrink,
                    ArtifactTransformKind::CandidMetadata => {
                        ArtifactTransformKindV1::CandidMetadata
                    }
                },
                mode: match transform.mode {
                    ArtifactTransformMode::Optional => ArtifactTransformModeV1::Optional,
                },
                tool: transform.tool.clone(),
                tool_version: transform.tool_version.clone(),
                outcome: match transform.outcome {
                    ArtifactTransformOutcome::Applied => ArtifactTransformOutcomeV1::Applied,
                    ArtifactTransformOutcome::ToolUnavailable => {
                        ArtifactTransformOutcomeV1::ToolUnavailable
                    }
                    ArtifactTransformOutcome::NotRequested => {
                        ArtifactTransformOutcomeV1::NotRequested
                    }
                },
            })
        })
        .collect()
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
