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
    release_set::{FleetConfigSnapshot, artifact_root_path},
    remove_optional_file,
    role_contract::{
        PackageValidationMode, RoleCargoGraphEvidence, RolePackageValidation, finding_detail,
        resolve_declared_role_package_contract, validate_declared_role_package,
    },
    should_export_candid_artifacts,
};

use super::{
    CanisterBuildProfile, WorkspaceBuildContext,
    cache::{canister_build_target_root, configure_canister_cargo_command},
    candid::{extract_candid, remove_stale_icp_candid_sidecars},
    model::{
        ArtifactTransformKind, ArtifactTransformOutput, CanisterArtifactBuildOutput,
        CanisterArtifactBuildSpec, ROOT_ROLE, WASM_STORE_ROLE, WASM_TARGET,
    },
    wasm_store::build_hidden_wasm_store_artifact,
};

pub fn build_workspace_canister_artifact(
    context: &WorkspaceBuildContext,
) -> Result<CanisterArtifactBuildOutput, Box<dyn std::error::Error>> {
    if context.role == WASM_STORE_ROLE {
        return build_hidden_wasm_store_artifact(context);
    }

    let config = FleetConfigSnapshot::load(&context.config_path)?;
    let spec = resolve_canister_artifact_build_spec(context, config.model())?;
    build_workspace_canister_artifact_from_spec(context, &spec)
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

// Build one visible Canic canister artifact from already-admitted package and path authority.
pub fn build_workspace_canister_artifact_from_spec(
    context: &WorkspaceBuildContext,
    spec: &CanisterArtifactBuildSpec,
) -> Result<CanisterArtifactBuildOutput, Box<dyn std::error::Error>> {
    if context.role != spec.role {
        return Err(format!(
            "build context role {} does not match admitted role {}",
            context.role, spec.role
        )
        .into());
    }

    let canister_name = spec.role.as_str();
    let require_embedded_release_artifacts = canister_name == ROOT_ROLE;

    let mut transforms = if require_embedded_release_artifacts {
        build_bootstrap_wasm_store_artifact(context)?.transforms
    } else {
        Vec::new()
    };

    fs::create_dir_all(&spec.artifact_root)?;
    remove_stale_icp_candid_sidecars(&spec.artifact_root)?;

    let release_wasm_path = run_canister_build(
        context,
        &spec.package_manifest_path,
        &spec.package_name,
        require_embedded_release_artifacts,
    )?;
    write_wasm_artifact(&release_wasm_path, &spec.wasm_path)?;
    transforms.push(maybe_shrink_wasm_artifact(&spec.wasm_path)?);

    if should_export_candid_artifacts(context.build_network) {
        let debug_context = context.with_profile(CanisterBuildProfile::Debug);
        let debug_wasm_path = run_canister_build(
            &debug_context,
            &spec.package_manifest_path,
            &spec.package_name,
            require_embedded_release_artifacts,
        )?;
        extract_candid(&debug_wasm_path, &spec.did_path)?;
        transforms.push(embed_candid_metadata(&spec.wasm_path, &spec.did_path)?);
    } else {
        remove_optional_file(&spec.did_path)?;
        transforms.push(ArtifactTransformOutput::not_requested(
            ArtifactTransformKind::CandidMetadata,
        ));
    }
    write_gzip_artifact(&spec.wasm_path, &spec.wasm_gz_path)?;

    Ok(CanisterArtifactBuildOutput {
        artifact_root: spec.artifact_root.clone(),
        wasm_path: spec.wasm_path.clone(),
        wasm_gz_path: spec.wasm_gz_path.clone(),
        did_path: spec.did_path.clone(),
        transforms,
    })
}

pub fn resolve_canister_artifact_build_spec(
    context: &WorkspaceBuildContext,
    config: &canic_core::bootstrap::compiled::ConfigModel,
) -> Result<CanisterArtifactBuildSpec, Box<dyn std::error::Error>> {
    let canister_name = context.role.as_str();
    let role = canic_core::ids::CanisterRole::owned(canister_name.to_string());
    validate_artifact_role_attached(config, canister_name)?;
    let evidence = match validate_declared_role_package(
        &context.config_path,
        config,
        &role,
        PackageValidationMode::Build,
    ) {
        RolePackageValidation::Supported(evidence) => evidence,
        RolePackageValidation::Unsupported(finding) => {
            return Err(format!("{}: {}", finding.code(), finding_detail(&finding)).into());
        }
    };
    require_declared_role_contract(config, &evidence)?;

    let artifact_root = artifact_root_path(&context.icp_root, "local").join(canister_name);
    Ok(CanisterArtifactBuildSpec {
        role: canister_name.to_string(),
        package_name: evidence.role_package_name,
        package_manifest_path: evidence.role_manifest_path,
        wasm_path: artifact_root.join(format!("{canister_name}.wasm")),
        wasm_gz_path: artifact_root.join(format!("{canister_name}.wasm.gz")),
        did_path: artifact_root.join(format!("{canister_name}.did")),
        artifact_root,
    })
}

fn require_declared_role_contract(
    config: &canic_core::bootstrap::compiled::ConfigModel,
    evidence: &RoleCargoGraphEvidence,
) -> Result<(), Box<dyn std::error::Error>> {
    match resolve_declared_role_package_contract(config, evidence) {
        canic_core::role_contract::RoleContractResolution::Resolved { .. } => Ok(()),
        canic_core::role_contract::RoleContractResolution::Rejected { errors } => Err(errors
            .iter()
            .map(|finding| format!("{}: {}", finding.code(), finding_detail(finding)))
            .collect::<Vec<_>>()
            .join("; ")
            .into()),
    }
}

fn validate_artifact_role_attached(
    config: &canic_core::bootstrap::compiled::ConfigModel,
    canister_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let role = canic_core::ids::CanisterRole::owned(canister_name.to_string());
    if !config.roles.contains_key(&role) {
        return Err(format!(
            "role {canister_name} is not declared; declare the role before building an artifact"
        )
        .into());
    }
    if !config.attached_roles().contains(&role) {
        let fleet = config.fleet_name().unwrap_or("<unknown>");
        return Err(format!(
            "role {fleet}.{canister_name} is declared but not attached to topology; run `canic fleet role attach {fleet} {canister_name} --subnet <subnet>` before building an artifact"
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
