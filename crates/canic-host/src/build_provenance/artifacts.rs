use std::path::Path;

use crate::{
    artifact_io::{IC_WASM_TOOL, WASM_OPT_TOOL},
    evidence_envelope::file_input_fingerprint,
};

use crate::canister_build::{ArtifactTransformKind, ArtifactTransformOutcome};

use super::model::{
    ArtifactProvenanceKindV1, ArtifactProvenanceV1, ArtifactTransformKindV1,
    ArtifactTransformOutcomeV1, ArtifactTransformProvenanceV1, BuildProvenanceRequest,
    WasmArtifactMetricsV1, WasmTransformMetricsV1,
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
            match (transform.transform, transform.outcome, &transform.metrics) {
                (ArtifactTransformKind::Optimize, ArtifactTransformOutcome::Applied, None) => {
                    return Err(
                        "applied release Wasm optimization must record before/after metrics".into(),
                    );
                }
                (ArtifactTransformKind::Optimize, ArtifactTransformOutcome::Applied, Some(_))
                | (_, _, None) => {}
                _ => {
                    return Err(
                        "only an applied release Wasm optimization may record transform metrics"
                            .into(),
                    );
                }
            }
            Ok(ArtifactTransformProvenanceV1 {
                role: request.role.clone(),
                transform: match transform.transform {
                    ArtifactTransformKind::Shrink => ArtifactTransformKindV1::Shrink,
                    ArtifactTransformKind::CandidMetadata => {
                        ArtifactTransformKindV1::CandidMetadata
                    }
                    ArtifactTransformKind::Optimize => ArtifactTransformKindV1::Optimize,
                },
                tool: match transform.transform {
                    ArtifactTransformKind::Shrink | ArtifactTransformKind::CandidMetadata => {
                        IC_WASM_TOOL
                    }
                    ArtifactTransformKind::Optimize => WASM_OPT_TOOL,
                }
                .to_string(),
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
                metrics: transform
                    .metrics
                    .as_ref()
                    .map(|metrics| WasmTransformMetricsV1 {
                        before: WasmArtifactMetricsV1 {
                            raw_bytes: metrics.before.raw_bytes,
                            gzip_bytes: metrics.before.gzip_bytes,
                            code_section_bytes: metrics.before.code_section_bytes,
                            data_section_bytes: metrics.before.data_section_bytes,
                            defined_functions: metrics.before.defined_functions,
                        },
                        after: WasmArtifactMetricsV1 {
                            raw_bytes: metrics.after.raw_bytes,
                            gzip_bytes: metrics.after.gzip_bytes,
                            code_section_bytes: metrics.after.code_section_bytes,
                            data_section_bytes: metrics.after.data_section_bytes,
                            defined_functions: metrics.after.defined_functions,
                        },
                    }),
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
        app: request.app.clone(),
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
