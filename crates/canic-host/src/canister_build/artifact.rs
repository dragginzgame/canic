use std::{
    env, fs,
    path::{Path, PathBuf},
};

use toml::Value as TomlValue;

use crate::{
    artifact_io::{
        embed_candid_metadata, maybe_shrink_wasm_artifact, write_gzip_artifact, write_wasm_artifact,
    },
    bootstrap_store::build_bootstrap_wasm_store_artifact,
    cargo_command, icp_environment_from_env,
    release_set::{
        canister_manifest_path, config_path, configured_role_lifecycle,
        emit_root_release_set_manifest_if_ready, icp_root, workspace_root,
    },
    remove_optional_file, should_export_candid_artifacts,
};

use super::{
    CanisterBuildProfile,
    candid::{extract_candid, remove_stale_icp_candid_sidecars},
    model::{
        CanisterArtifactBuildOutput, LOCAL_ARTIFACT_ROOT_RELATIVE, ROOT_ROLE, WASM_STORE_ROLE,
        WASM_TARGET,
    },
    wasm_store::build_hidden_wasm_store_artifact,
};

pub fn build_current_workspace_canister_artifact(
    canister_name: &str,
    profile: CanisterBuildProfile,
) -> Result<CanisterArtifactBuildOutput, Box<dyn std::error::Error>> {
    let workspace_root = workspace_root()?;
    let icp_root = icp_root()?;
    build_canister_artifact(&workspace_root, &icp_root, canister_name, profile)
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
    workspace_root: &Path,
    icp_root: &Path,
    canister_name: &str,
    profile: CanisterBuildProfile,
) -> Result<CanisterArtifactBuildOutput, Box<dyn std::error::Error>> {
    if canister_name == WASM_STORE_ROLE {
        return build_hidden_wasm_store_artifact(workspace_root, icp_root, profile);
    }

    validate_artifact_role_attached(workspace_root, canister_name)?;
    let canister_manifest_path = canister_manifest_path(workspace_root, canister_name)?;
    let canister_package_name = load_canister_package_name(&canister_manifest_path)?;
    let artifact_root = icp_root
        .join(LOCAL_ARTIFACT_ROOT_RELATIVE)
        .join(canister_name);
    let wasm_path = artifact_root.join(format!("{canister_name}.wasm"));
    let wasm_gz_path = artifact_root.join(format!("{canister_name}.wasm.gz"));
    let did_path = artifact_root.join(format!("{canister_name}.did"));
    let require_embedded_release_artifacts = canister_name == ROOT_ROLE;

    if require_embedded_release_artifacts {
        build_bootstrap_wasm_store_artifact(workspace_root, icp_root, profile)?;
    }

    fs::create_dir_all(&artifact_root)?;
    remove_stale_icp_candid_sidecars(&artifact_root)?;

    let release_wasm_path = run_canister_build(
        workspace_root,
        icp_root,
        &canister_manifest_path,
        &canister_package_name,
        profile,
        require_embedded_release_artifacts,
    )?;
    write_wasm_artifact(&release_wasm_path, &wasm_path)?;
    maybe_shrink_wasm_artifact(&wasm_path)?;

    let network = icp_environment_from_env();
    if should_export_candid_artifacts(&network) {
        let debug_wasm_path = run_canister_build(
            workspace_root,
            icp_root,
            &canister_manifest_path,
            &canister_package_name,
            CanisterBuildProfile::Debug,
            require_embedded_release_artifacts,
        )?;
        extract_candid(&debug_wasm_path, &did_path)?;
        embed_candid_metadata(&wasm_path, &did_path)?;
    } else {
        remove_optional_file(&did_path)?;
    }
    write_gzip_artifact(&wasm_path, &wasm_gz_path)?;

    let manifest_path =
        emit_root_release_set_manifest_if_ready(workspace_root, icp_root, &network)?;

    Ok(CanisterArtifactBuildOutput {
        artifact_root,
        wasm_path,
        wasm_gz_path,
        did_path,
        manifest_path,
    })
}

fn validate_artifact_role_attached(
    workspace_root: &Path,
    canister_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let config_path = config_path(workspace_root);
    let roles = configured_role_lifecycle(&config_path)?;
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

// Read the real package name from one canister manifest so downstreams are not
// forced to mirror the reference `canister_<role>` naming scheme.
fn load_canister_package_name(manifest_path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let manifest_source = fs::read_to_string(manifest_path)?;
    let manifest = toml::from_str::<TomlValue>(&manifest_source)?;
    let package_name = manifest
        .get("package")
        .and_then(TomlValue::as_table)
        .and_then(|package| package.get("name"))
        .and_then(TomlValue::as_str)
        .ok_or_else(|| format!("missing package.name in {}", manifest_path.display()))?;

    Ok(package_name.to_string())
}

// Run one wasm-target cargo build for the requested canister manifest/profile.
fn run_canister_build(
    workspace_root: &Path,
    icp_root: &Path,
    manifest_path: &Path,
    package_name: &str,
    profile: CanisterBuildProfile,
    require_embedded_release_artifacts: bool,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let target_root = std::env::var_os("CARGO_TARGET_DIR")
        .map_or_else(|| workspace_root.join("target"), PathBuf::from);
    let mut command = cargo_command();
    command
        .current_dir(workspace_root)
        .env("CARGO_TARGET_DIR", &target_root)
        .env("CANIC_ICP_ROOT", icp_root)
        .args([
            "build",
            "--manifest-path",
            &manifest_path.display().to_string(),
            "--target",
            WASM_TARGET,
        ])
        .args(profile.cargo_args());

    if require_embedded_release_artifacts {
        command.env("CANIC_REQUIRE_EMBEDDED_RELEASE_ARTIFACTS", "1");
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
        .join(profile.target_dir_name())
        .join(format!("{}.wasm", package_name.replace('-', "_"))))
}
