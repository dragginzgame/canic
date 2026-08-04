//! Module: bootstrap_coordinator
//!
//! Responsibility: build the exact built-in Fleet Coordinator wrapper and artifact.
//! Does not own: Coordinator placement, installation effects, or Fleet Registry mutation.
//! Boundary: resolves the selected Canic package and emits one qualified current-build Wasm.

#[cfg(test)]
mod tests;

use crate::{
    artifact_io::{
        embed_candid_metadata, maybe_shrink_wasm_artifact, write_gzip_artifact, write_wasm_artifact,
    },
    bootstrap_store::{
        append_profile_config_args, generated_wasm_store_wrapper_patch_table, render_profile,
        resolved_canic_package, resolved_wrapper_dependencies,
    },
    canister_build::{
        ArtifactTransformKind, ArtifactTransformOutput, CanisterArtifactBuildOutput,
        CanisterBuildProfile, WorkspaceBuildContext,
        cache::{canister_build_target_root, configure_canister_cargo_command},
    },
    cargo_command,
    cargo_metadata::cargo_metadata,
    remove_optional_file,
    role_contract::{
        PackageValidationMode, RolePackageValidation, finding_detail,
        resolve_built_in_fleet_coordinator_contract, validate_built_in_fleet_coordinator_package,
    },
    should_export_candid_artifacts,
};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

const FLEET_COORDINATOR_ROLE: &str = "fleet_coordinator";
const GENERATED_WRAPPER_RELATIVE: &str = ".icp/local/generated/canic-fleet-coordinator";
const GENERATED_WRAPPER_PACKAGE_NAME: &str = "canic-generated-fleet-coordinator";
const GENERATED_WRAPPER_CRATE_NAME: &str = "canister_fleet_coordinator";
const COORDINATOR_RELEASE_PROFILE: &[(&str, &str)] = &[
    ("opt-level", "\"z\""),
    ("lto", "true"),
    ("codegen-units", "1"),
    ("strip", "\"symbols\""),
    ("debug", "false"),
    ("panic", "\"abort\""),
    ("overflow-checks", "false"),
    ("incremental", "false"),
];
const COORDINATOR_FAST_PROFILE: &[(&str, &str)] = &[
    ("inherits", "\"release\""),
    ("lto", "false"),
    ("codegen-units", "16"),
    ("incremental", "false"),
];

/// Build the dedicated Fleet Coordinator wrapper selected from the exact Canic dependency graph.
pub fn build_bootstrap_fleet_coordinator_artifact(
    context: &WorkspaceBuildContext,
) -> Result<CanisterArtifactBuildOutput, Box<dyn std::error::Error>> {
    let manifest_path = ensure_generated_wrapper(context)?;
    require_built_in_fleet_coordinator_contract(&manifest_path)?;
    run_coordinator_cargo_build(context, &manifest_path)?;

    let built_wasm_path = canister_build_target_root(&context.workspace_root)
        .join("wasm32-unknown-unknown")
        .join(context.profile.target_dir_name())
        .join(format!("{GENERATED_WRAPPER_CRATE_NAME}.wasm"));
    let artifact_root = context.artifact_root().join(FLEET_COORDINATOR_ROLE);
    fs::create_dir_all(&artifact_root)?;
    let wasm_path = artifact_root.join(format!("{FLEET_COORDINATOR_ROLE}.wasm"));
    let wasm_gz_path = artifact_root.join(format!("{FLEET_COORDINATOR_ROLE}.wasm.gz"));
    let did_path = artifact_root.join(format!("{FLEET_COORDINATOR_ROLE}.did"));

    write_wasm_artifact(&built_wasm_path, &wasm_path)?;
    let mut transforms = vec![maybe_shrink_wasm_artifact(&wasm_path)?];
    if should_export_candid_artifacts(context.build_network) {
        let debug_context = context.with_profile(CanisterBuildProfile::Debug);
        run_coordinator_cargo_build(&debug_context, &manifest_path)?;
        let debug_wasm_path = canister_build_target_root(&context.workspace_root)
            .join("wasm32-unknown-unknown")
            .join(CanisterBuildProfile::Debug.target_dir_name())
            .join(format!("{GENERATED_WRAPPER_CRATE_NAME}.wasm"));
        extract_candid(&debug_wasm_path, &did_path)?;
        transforms.push(embed_candid_metadata(&wasm_path, &did_path)?);
    } else {
        remove_optional_file(&did_path)?;
        transforms.push(ArtifactTransformOutput::not_requested(
            ArtifactTransformKind::CandidMetadata,
        ));
    }
    write_gzip_artifact(&wasm_path, &wasm_gz_path)?;

    Ok(CanisterArtifactBuildOutput {
        package_name: GENERATED_WRAPPER_PACKAGE_NAME.to_string(),
        artifact_root,
        wasm_path,
        wasm_gz_path,
        did_path,
        transforms,
    })
}

fn require_built_in_fleet_coordinator_contract(
    manifest_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let evidence = match validate_built_in_fleet_coordinator_package(
        manifest_path,
        PackageValidationMode::Build,
    ) {
        RolePackageValidation::Supported(evidence) => evidence,
        RolePackageValidation::Unsupported(finding) => {
            return Err(format!("{}: {}", finding.code(), finding_detail(&finding)).into());
        }
    };
    match resolve_built_in_fleet_coordinator_contract(&evidence) {
        canic_core::role_contract::RoleContractResolution::Resolved { .. } => Ok(()),
        canic_core::role_contract::RoleContractResolution::Rejected { errors } => Err(errors
            .iter()
            .map(|finding| format!("{}: {}", finding.code(), finding_detail(finding)))
            .collect::<Vec<_>>()
            .join("; ")
            .into()),
    }
}

fn ensure_generated_wrapper(
    context: &WorkspaceBuildContext,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let metadata = cargo_metadata(&context.workspace_root, true)?;
    let canic_package = resolved_canic_package(&metadata)?;
    let dependencies = resolved_wrapper_dependencies(&metadata, canic_package)?;
    let canic_root = canic_package
        .manifest_path
        .parent()
        .expect("Canic manifest path must have parent");
    let wrapper_root = context.icp_root.join(GENERATED_WRAPPER_RELATIVE);
    fs::create_dir_all(wrapper_root.join("src"))?;
    let patch_table = generated_wasm_store_wrapper_patch_table(
        &canic_package.manifest_path,
        &dependencies.canic_version,
    )?;
    let mut cargo_toml = format!(
        "[package]\n\
name = \"{GENERATED_WRAPPER_PACKAGE_NAME}\"\n\
version = \"0.0.0\"\n\
edition = \"2024\"\n\
publish = false\n\n\
[package.metadata.canic]\n\
app = \"fleet_coordinator\"\n\
role = \"fleet_coordinator\"\n\n\
[workspace]\n\
resolver = \"2\"\n\n\
[lib]\n\
name = \"{GENERATED_WRAPPER_CRATE_NAME}\"\n\
crate-type = [\"cdylib\", \"rlib\"]\n\n\
[dependencies]\n\
canic = {{ path = \"{}\", default-features = false, features = [\"fleet-coordinator-canister\"] }}\n\
ic-cdk = \"={}\"\n\
candid = {{ version = \"={}\", default-features = false }}\n",
        canic_root.display(),
        dependencies.ic_cdk_version,
        dependencies.candid_version,
    );
    render_profile(&mut cargo_toml, "release", COORDINATOR_RELEASE_PROFILE);
    render_profile(&mut cargo_toml, "fast", COORDINATOR_FAST_PROFILE);
    if !patch_table.is_empty() {
        cargo_toml.push('\n');
        cargo_toml.push_str(&patch_table);
    }
    fs::write(wrapper_root.join("Cargo.toml"), cargo_toml)?;
    fs::write(
        wrapper_root.join("src/lib.rs"),
        "canic::start_fleet_coordinator!();\ncanic::finish!();\n",
    )?;
    let workspace_lock = context.workspace_root.join("Cargo.lock");
    if workspace_lock.is_file() {
        fs::copy(workspace_lock, wrapper_root.join("Cargo.lock"))?;
    }
    Ok(wrapper_root.join("Cargo.toml"))
}

fn run_coordinator_cargo_build(
    context: &WorkspaceBuildContext,
    manifest_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut command = cargo_command();
    context.apply_to_command(&mut command);
    command.current_dir(&context.workspace_root).args([
        "build",
        "--manifest-path",
        &manifest_path.display().to_string(),
        "--target",
        "wasm32-unknown-unknown",
    ]);
    configure_canister_cargo_command(&mut command, &context.workspace_root);
    append_coordinator_profile_config_args(&mut command, context.profile);
    command.args(context.profile.cargo_args());

    let output = command.output()?;
    if output.status.success() {
        return Ok(());
    }
    Err(format!(
        "cargo build failed for Fleet Coordinator: {}",
        String::from_utf8_lossy(&output.stderr)
    )
    .into())
}

fn append_coordinator_profile_config_args(command: &mut Command, profile: CanisterBuildProfile) {
    match profile {
        CanisterBuildProfile::Debug => {}
        CanisterBuildProfile::Fast => {
            append_profile_config_args(command, "release", COORDINATOR_RELEASE_PROFILE);
            append_profile_config_args(command, "fast", COORDINATOR_FAST_PROFILE);
        }
        CanisterBuildProfile::Release => {
            append_profile_config_args(command, "release", COORDINATOR_RELEASE_PROFILE);
        }
    }
}

fn extract_candid(wasm_path: &Path, did_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new("candid-extractor").arg(wasm_path).output()?;
    if !output.status.success() {
        return Err(format!(
            "candid-extractor failed for Fleet Coordinator: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    fs::write(did_path, output.stdout)?;
    Ok(())
}
