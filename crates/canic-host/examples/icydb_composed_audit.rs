//! Build the ICYDB-033 controlled pair through the existing artifact owner.
//! This audit tool does not install canisters or publish release artifacts.

use canic_core::{
    cdk::utils::hash::hex_bytes,
    ids::{BuildNetwork, ReleaseBuildId, ReleaseBuildNonce},
};
use canic_host::canister_build::{
    CanisterArtifactBuildOptions, CanisterArtifactBuilder, CanisterBuildProfile,
    WorkspaceBuildContext, read_wasm_artifact_metrics,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, env, fs, path::Path};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().skip(1).collect();
    match args.as_slice() {
        [command, wasm] if command == "measure" => {
            println!("{}", serde_json::to_string_pretty(&measure(Path::new(wasm))?)?);
        }
        [command, workspace, evidence, variant] if command == "build" => {
            build(Path::new(workspace), Path::new(evidence), variant)?;
        }
        _ => return Err("usage: icydb_composed_audit build <workspace> <evidence> <host|participant> | measure <wasm>".into()),
    }
    Ok(())
}

fn build(
    workspace: &Path,
    evidence: &Path,
    variant: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let cargo_features = match variant {
        "host" => BTreeSet::new(),
        "participant" => std::iter::once("participant".to_owned()).collect(),
        _ => return Err("variant must be host or participant".into()),
    };
    let workspace = workspace.canonicalize()?;
    fs::create_dir_all(evidence)?;
    let evidence = evidence.canonicalize()?;
    // Deliberately synthetic, shared identity; this is not a deployment build plan.
    let release_build_id =
        ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes([0; 32]));
    let context = WorkspaceBuildContext {
        role: "probe".to_owned(),
        profile: CanisterBuildProfile::Release,
        environment: "local".to_owned(),
        build_network: BuildNetwork::Local,
        config_path: workspace.join("canic.toml"),
        workspace_root: workspace.clone(),
        icp_root: workspace,
        local_replica: None,
        refresh_canonical_infrastructure_did: false,
        release_build_id: Some(release_build_id),
    };
    let builder = CanisterArtifactBuilder::for_profile(context.profile)?;
    let output = builder.build_workspace_canister_artifact_with_options(
        &context,
        &CanisterArtifactBuildOptions {
            cargo_features,
            default_features: false,
            sidecar_only_candid: true,
        },
    )?;
    let target =
        env::var_os("CARGO_TARGET_DIR").ok_or("set absolute CARGO_TARGET_DIR for the audit")?;
    let compiler_wasm =
        Path::new(&target).join("wasm32-unknown-unknown/release/canic_composed_wasm_probe.wasm");
    for (source, extension) in [
        (&output.wasm_path, "wasm"),
        (&output.wasm_gz_path, "wasm.gz"),
        (&output.did_path, "did"),
        (&compiler_wasm, "compiler.wasm"),
    ] {
        fs::copy(source, evidence.join(format!("{variant}.{extension}")))?;
    }
    let report = json!({
        "variant": variant,
        "release_build_id": release_build_id.to_string(),
        "profile": "release",
        "network": "local",
        "tools": builder.diagnostic_lines(),
        "canonical": measure(&output.wasm_path)?,
        "compiler": measure(&compiler_wasm)?,
        "gzip_bytes": fs::metadata(&output.wasm_gz_path)?.len(),
        "transforms": format!("{:#?}", output.transforms),
    });
    fs::write(
        evidence.join(format!("{variant}.json")),
        serde_json::to_vec_pretty(&report)?,
    )?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

fn measure(wasm: &Path) -> Result<Value, Box<dyn std::error::Error>> {
    let bytes = fs::read(wasm)?;
    // This Linux audit measures raw sections only; no transport size is claimed here.
    let metrics = read_wasm_artifact_metrics(wasm, Path::new("/dev/null"))?;
    Ok(json!({
        "sha256": hex_bytes(Sha256::digest(&bytes)),
        "raw_bytes": metrics.raw_bytes,
        "code_section_bytes": metrics.code_section_bytes,
        "data_section_bytes": metrics.data_section_bytes,
        "defined_functions": metrics.defined_functions,
    }))
}
