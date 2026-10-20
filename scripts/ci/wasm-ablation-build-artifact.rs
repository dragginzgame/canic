//! Method-owned artifact builder for controlled Wasm-ablation measurements.
//!
//! This executable is compiled outside the measured product worktree against
//! that worktree's exact `canic-host` and `canic-core` packages. It keeps
//! structured measurement reporting out of the product build helper while
//! preserving the historical product source as the build authority.

use canic_core::ids::ReleaseBuildId;
use canic_host::canister_build::{
    ArtifactTransformKind, ArtifactTransformOutcome, CanisterArtifactBuildOutput,
    CanisterBuildProfile, WasmArtifactMetrics, WorkspaceBuildContext,
    build_workspace_canister_artifact, copy_icp_wasm_output,
};
use canic_host::icp_config::resolve_icp_build_network_from_root;
use serde::Serialize;
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Serialize)]
struct TransformMetricsReportV1<'a> {
    schema_version: u8,
    role: &'a str,
    transforms: Vec<TransformMetricsV1>,
}

#[derive(Serialize)]
struct TransformMetricsV1 {
    transform: &'static str,
    outcome: &'static str,
    metrics: Option<WasmTransformMetricsV1>,
}

#[derive(Serialize)]
struct WasmTransformMetricsV1 {
    before: WasmArtifactMetricsV1,
    after: WasmArtifactMetricsV1,
}

#[derive(Serialize)]
struct WasmArtifactMetricsV1 {
    raw_bytes: u64,
    gzip_bytes: u64,
    code_section_bytes: u64,
    data_section_bytes: u64,
    defined_functions: u32,
}

/// Build one measurement artifact from its exact product workspace.
#[derive(clap::Parser)]
struct Arguments {
    canister_name: String,
    profile: CanisterBuildProfile,
    workspace_root: PathBuf,
    icp_root: PathBuf,
    config_path: String,
    transform_metrics_path: PathBuf,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let Arguments {
        canister_name,
        profile,
        workspace_root,
        icp_root,
        config_path,
        transform_metrics_path,
    } = <Arguments as clap::Parser>::parse();
    let workspace_root = workspace_root.canonicalize()?;
    let icp_root = icp_root.canonicalize()?;
    let config_path = resolve_path(&workspace_root, &config_path).canonicalize()?;
    let environment = std::env::var("ICP_ENVIRONMENT").unwrap_or_else(|_| "local".to_string());
    let build_network = resolve_icp_build_network_from_root(&icp_root, &environment)?;
    let context = WorkspaceBuildContext {
        role: canister_name.clone(),
        profile,
        environment,
        build_network,
        config_path,
        workspace_root,
        icp_root,
        local_replica: None,
        refresh_canonical_infrastructure_did: false,
        release_build_id: None::<ReleaseBuildId>,
    };
    eprintln!("{}", context.lines().join("\n"));
    let output = build_workspace_canister_artifact(&context)?;
    copy_icp_wasm_output(&canister_name, &output)?;
    write_transform_metrics(Path::new(&transform_metrics_path), &canister_name, &output)?;
    println!("{}", output.wasm_gz_path.display());
    Ok(())
}

fn write_transform_metrics(
    path: &Path,
    role: &str,
    output: &CanisterArtifactBuildOutput,
) -> Result<(), Box<dyn std::error::Error>> {
    let report = TransformMetricsReportV1 {
        schema_version: 1,
        role,
        transforms: output
            .transforms
            .iter()
            .map(|transform| TransformMetricsV1 {
                transform: match transform.transform {
                    ArtifactTransformKind::Shrink => "shrink",
                    ArtifactTransformKind::CandidMetadata => "candid_metadata",
                    ArtifactTransformKind::Optimize => "optimize",
                },
                outcome: match transform.outcome {
                    ArtifactTransformOutcome::Applied => "applied",
                    ArtifactTransformOutcome::ToolUnavailable => "tool_unavailable",
                    ArtifactTransformOutcome::NotRequested => "not_requested",
                },
                metrics: transform
                    .metrics
                    .as_ref()
                    .map(|metrics| WasmTransformMetricsV1 {
                        before: wasm_artifact_metrics(&metrics.before),
                        after: wasm_artifact_metrics(&metrics.after),
                    }),
            })
            .collect(),
    };
    let mut encoded = serde_json::to_vec_pretty(&report)?;
    encoded.push(b'\n');
    fs::write(path, encoded)?;
    Ok(())
}

const fn wasm_artifact_metrics(metrics: &WasmArtifactMetrics) -> WasmArtifactMetricsV1 {
    WasmArtifactMetricsV1 {
        raw_bytes: metrics.raw_bytes,
        gzip_bytes: metrics.gzip_bytes,
        code_section_bytes: metrics.code_section_bytes,
        data_section_bytes: metrics.data_section_bytes,
        defined_functions: metrics.defined_functions,
    }
}

fn resolve_path(root: &Path, path: &str) -> PathBuf {
    let path = PathBuf::from(path);
    if path.is_absolute() {
        path
    } else {
        root.join(path)
    }
}
