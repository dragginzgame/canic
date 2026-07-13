use std::{
    env, fs,
    path::{Path, PathBuf},
};

use crate::{
    artifact_io::{
        embed_candid_metadata, maybe_shrink_wasm_artifact, write_gzip_artifact, write_wasm_artifact,
    },
    bootstrap_store::build_bootstrap_wasm_store_artifact,
    cargo_command,
    release_set::{configured_role_lifecycle, emit_root_release_set_manifest_if_ready_with_config},
    remove_optional_file,
    role_contract::{
        PackageValidationMode, RolePackageEvidence, RolePackageValidation, finding_detail,
        resolve_declared_role_package_contract, validate_declared_role_package,
    },
    should_export_candid_artifacts,
};

use super::{
    CanisterBuildProfile, WorkspaceBuildContext,
    cache::{canister_build_target_root, configure_canister_cargo_command},
    candid::{extract_candid, remove_stale_icp_candid_sidecars},
    model::{
        CanisterArtifactBuildOutput, LOCAL_ARTIFACT_ROOT_RELATIVE, ROOT_ROLE, WASM_STORE_ROLE,
        WASM_TARGET,
    },
    wasm_store::build_hidden_wasm_store_artifact,
};

pub fn build_workspace_canister_artifact(
    context: &WorkspaceBuildContext,
) -> Result<CanisterArtifactBuildOutput, Box<dyn std::error::Error>> {
    build_canister_artifact(context)
}

/// Copy the uncompressed artifact to the path requested by ICP custom builds.
///
/// ICP CLI sets `ICP_WASM_OUTPUT_PATH` for script-backed canister builds. Normal
/// direct `canic build <fleet> <role>` calls leave it unset and only write Canic's
/// canonical `.icp/local/canisters/<role>/` artifacts.
pub fn copy_icp_wasm_output(
    canister_name: &str,
    output: &CanisterArtifactBuildOutput,
) -> Result<(), Box<dyn std::error::Error>> {
    let Some(path) = env::var_os("ICP_WASM_OUTPUT_PATH").map(PathBuf::from) else {
        return Ok(());
    };

    if !output.wasm_path.is_file() {
        return Err(format!(
            "missing ICP wasm output source for {canister_name}: {}",
            output.wasm_path.display()
        )
        .into());
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(&output.wasm_path, Path::new(&path))?;
    Ok(())
}

// Build one visible Canic canister artifact and keep the thin-root special cases.
fn build_canister_artifact(
    context: &WorkspaceBuildContext,
) -> Result<CanisterArtifactBuildOutput, Box<dyn std::error::Error>> {
    let canister_name = context.role.as_str();
    if canister_name == WASM_STORE_ROLE {
        return build_hidden_wasm_store_artifact(context);
    }

    validate_artifact_role_attached(&context.config_path, canister_name)?;
    let role_package = require_declared_role_contract(&context.config_path, canister_name)?;
    let canister_manifest_path = role_package.role_manifest_path;
    let canister_package_name = role_package.role_package_name;
    let artifact_root = context
        .icp_root
        .join(LOCAL_ARTIFACT_ROOT_RELATIVE)
        .join(canister_name);
    let wasm_path = artifact_root.join(format!("{canister_name}.wasm"));
    let wasm_gz_path = artifact_root.join(format!("{canister_name}.wasm.gz"));
    let did_path = artifact_root.join(format!("{canister_name}.did"));
    let require_embedded_release_artifacts = canister_name == ROOT_ROLE;

    if require_embedded_release_artifacts {
        build_bootstrap_wasm_store_artifact(context)?;
    }

    fs::create_dir_all(&artifact_root)?;
    remove_stale_icp_candid_sidecars(&artifact_root)?;

    let release_wasm_path = run_canister_build(
        context,
        &canister_manifest_path,
        &canister_package_name,
        require_embedded_release_artifacts,
    )?;
    write_wasm_artifact(&release_wasm_path, &wasm_path)?;
    maybe_shrink_wasm_artifact(&wasm_path)?;

    let network = &context.build_network;
    if should_export_candid_artifacts(network) {
        let debug_context = context.with_profile(CanisterBuildProfile::Debug);
        let debug_wasm_path = run_canister_build(
            &debug_context,
            &canister_manifest_path,
            &canister_package_name,
            require_embedded_release_artifacts,
        )?;
        extract_candid(&debug_wasm_path, &did_path)?;
        embed_candid_metadata(&wasm_path, &did_path)?;
    } else {
        remove_optional_file(&did_path)?;
    }
    write_gzip_artifact(&wasm_path, &wasm_gz_path)?;

    let manifest_path = emit_root_release_set_manifest_if_ready_with_config(
        &context.workspace_root,
        &context.icp_root,
        network,
        &context.config_path,
    )?;

    Ok(CanisterArtifactBuildOutput {
        artifact_root,
        wasm_path,
        wasm_gz_path,
        did_path,
        manifest_path,
    })
}

fn require_declared_role_contract(
    config_path: &Path,
    canister_name: &str,
) -> Result<RolePackageEvidence, Box<dyn std::error::Error>> {
    let role = canic_core::ids::CanisterRole::owned(canister_name.to_string());
    let evidence =
        match validate_declared_role_package(config_path, &role, PackageValidationMode::Build) {
            RolePackageValidation::Supported(evidence) => evidence,
            RolePackageValidation::Unsupported(finding) => {
                return Err(format!("{}: {}", finding.code(), finding_detail(&finding)).into());
            }
        };
    match resolve_declared_role_package_contract(config_path, &evidence) {
        canic_core::role_contract::RoleContractResolution::Resolved { .. } => Ok(evidence),
        canic_core::role_contract::RoleContractResolution::Rejected { errors } => Err(errors
            .iter()
            .map(|finding| format!("{}: {}", finding.code(), finding_detail(finding)))
            .collect::<Vec<_>>()
            .join("; ")
            .into()),
    }
}

fn validate_artifact_role_attached(
    config_path: &Path,
    canister_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let roles = configured_role_lifecycle(config_path)?;
    let Some(row) = roles.iter().find(|row| row.role == canister_name) else {
        return Err(format!(
            "role {canister_name} is not declared in {}; declare the role before building an artifact",
            config_path.display()
        )
        .into());
    };
    if !row.attached {
        return Err(format!(
            "role {}.{} is declared but not attached to topology; run `canic fleet role attach {} {} --subnet <subnet>` before building an artifact",
            row.fleet, row.role, row.fleet, row.role
        )
        .into());
    }
    Ok(())
}

// Run one wasm-target cargo build for the requested canister manifest/profile.
fn run_canister_build(
    context: &WorkspaceBuildContext,
    manifest_path: &Path,
    package_name: &str,
    require_embedded_release_artifacts: bool,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let target_root = canister_build_target_root(&context.workspace_root);
    let mut command = cargo_command();
    context.apply_to_command(&mut command);
    command
        .current_dir(&context.workspace_root)
        .env(
            canic_core::role_contract::CANONICAL_BUILD_MARKER_ENV,
            canic_core::role_contract::CANONICAL_BUILD_MARKER_VALUE,
        )
        .args([
            "build",
            "--manifest-path",
            &manifest_path.display().to_string(),
            "--target",
            WASM_TARGET,
        ])
        .args(context.profile.cargo_args());
    configure_canister_cargo_command(&mut command, &context.workspace_root);

    if require_embedded_release_artifacts {
        command.env(
            canic_core::role_contract::CANONICAL_BUILD_REQUIRE_EMBEDDED_ARTIFACTS_ENV,
            canic_core::role_contract::CANONICAL_BUILD_MARKER_VALUE,
        );
    }

    let output = command.output()?;
    if !output.status.success() {
        return Err(format!(
            "cargo build failed for {}: {}",
            manifest_path.display(),
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    Ok(target_root
        .join(WASM_TARGET)
        .join(context.profile.target_dir_name())
        .join(format!("{}.wasm", package_name.replace('-', "_"))))
}
