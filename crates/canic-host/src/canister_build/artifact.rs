use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

use crate::{
    artifact_io::{
        embed_candid_metadata, maybe_shrink_wasm_artifact, write_gzip_artifact, write_wasm_artifact,
    },
    bootstrap_coordinator::build_bootstrap_fleet_coordinator_artifact,
    bootstrap_store::build_bootstrap_wasm_store_artifact,
    cargo_command,
    release_set::AppConfigSnapshot,
    remove_optional_file,
    role_contract::{
        PackageValidationMode, RoleCargoGraphEvidence, RolePackageValidation, finding_detail,
        resolve_declared_role_package_contract, validate_declared_role_package,
        validate_declared_role_packages,
    },
    should_export_candid_artifacts,
};

use super::{
    CanisterBuildProfile, WorkspaceBuildContext,
    cache::{canister_build_target_root, configure_canister_cargo_command},
    candid::{extract_candid, remove_stale_icp_candid_sidecars},
    model::{
        ArtifactTransformKind, ArtifactTransformOutput, CanisterArtifactBuildOutput,
        CanisterArtifactBuildSpec, CanisterArtifactSource, ConfiguredCanisterArtifactBuildOutput,
        WASM_TARGET,
    },
};

pub fn build_workspace_canister_artifact(
    context: &WorkspaceBuildContext,
) -> Result<CanisterArtifactBuildOutput, Box<dyn std::error::Error>> {
    match CanisterArtifactSource::for_role(&context.role) {
        CanisterArtifactSource::FleetCoordinator => {
            return build_bootstrap_fleet_coordinator_artifact(context);
        }
        CanisterArtifactSource::WasmStore => {
            return build_bootstrap_wasm_store_artifact(context);
        }
        CanisterArtifactSource::DeclaredRole => {}
    }

    let config = AppConfigSnapshot::load(&context.config_path)?;
    let spec = resolve_canister_artifact_build_spec(context, config.model())?;
    build_workspace_canister_artifact_from_spec(context, &spec)
}

/// Build the requested configured roles in one Cargo invocation per workspace and profile.
pub fn build_workspace_configured_canister_artifacts(
    context: &WorkspaceBuildContext,
    roles: &[String],
) -> Result<Vec<ConfiguredCanisterArtifactBuildOutput>, Box<dyn std::error::Error>> {
    let config = AppConfigSnapshot::load(&context.config_path)?;
    let specs = resolve_canister_artifact_build_specs(context, config.model(), roles)?;
    let outputs = build_workspace_canister_artifacts_from_specs(context, &specs)?;

    Ok(roles
        .iter()
        .cloned()
        .zip(outputs)
        .map(|(role, output)| ConfiguredCanisterArtifactBuildOutput { role, output })
        .collect())
}

/// Copy the uncompressed artifact to the path requested by ICP custom builds.
///
/// ICP CLI sets `ICP_WASM_OUTPUT_PATH` for script-backed canister builds. Normal
/// direct `canic build <app> <role>` calls leave it unset and only write Canic's
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
fn build_workspace_canister_artifact_from_spec(
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

    prepare_canister_artifact_output(spec)?;

    let release_wasm_path =
        run_canister_build(context, &spec.package_manifest_path, &spec.package_name)?;
    let candid_wasm_path = should_export_candid_artifacts(context.build_network)
        .then_some(release_wasm_path.as_path());
    finish_canister_artifact_output(context, spec, &release_wasm_path, candid_wasm_path)
}

/// Build all admitted configured roles in one Cargo invocation per workspace and profile.
pub fn build_workspace_canister_artifacts_from_specs(
    context: &WorkspaceBuildContext,
    specs: &[CanisterArtifactBuildSpec],
) -> Result<Vec<CanisterArtifactBuildOutput>, Box<dyn std::error::Error>> {
    if specs.is_empty() {
        return Ok(Vec::new());
    }

    for spec in specs {
        prepare_canister_artifact_output(spec)?;
    }
    let workspace_groups = group_build_specs_by_workspace(specs);

    for (cargo_workspace_root, group) in &workspace_groups {
        run_canister_build_batch(context, cargo_workspace_root, group, context.profile)?;
    }
    let export_candid = should_export_candid_artifacts(context.build_network);

    std::thread::scope(|scope| {
        let mut handles = Vec::with_capacity(specs.len());
        for spec in specs {
            handles.push(scope.spawn(move || {
                let release_wasm_path =
                    built_canister_wasm_path(context, context.profile, spec.package_name.as_str());
                finish_canister_artifact_output(
                    context,
                    spec,
                    &release_wasm_path,
                    export_candid.then_some(release_wasm_path.as_path()),
                )
                .map_err(|error| error.to_string())
            }));
        }

        handles
            .into_iter()
            .map(|handle| {
                handle
                    .join()
                    .map_err(|_| "configured artifact finalization thread panicked".to_string())?
                    .map_err(Into::into)
            })
            .collect()
    })
}

fn group_build_specs_by_workspace(
    specs: &[CanisterArtifactBuildSpec],
) -> BTreeMap<PathBuf, Vec<&CanisterArtifactBuildSpec>> {
    let mut groups = BTreeMap::<PathBuf, Vec<&CanisterArtifactBuildSpec>>::new();
    for spec in specs {
        groups
            .entry(spec.cargo_workspace_root.clone())
            .or_default()
            .push(spec);
    }
    groups
}

fn prepare_canister_artifact_output(
    spec: &CanisterArtifactBuildSpec,
) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(&spec.artifact_root)?;
    remove_stale_icp_candid_sidecars(&spec.artifact_root)?;
    Ok(())
}

fn finish_canister_artifact_output(
    context: &WorkspaceBuildContext,
    spec: &CanisterArtifactBuildSpec,
    release_wasm_path: &Path,
    candid_wasm_path: Option<&Path>,
) -> Result<CanisterArtifactBuildOutput, Box<dyn std::error::Error>> {
    let mut transforms = Vec::new();
    write_wasm_artifact(release_wasm_path, &spec.wasm_path)?;
    transforms.push(maybe_shrink_wasm_artifact(&spec.wasm_path)?);

    if should_export_candid_artifacts(context.build_network) {
        let candid_wasm_path = candid_wasm_path.ok_or_else(|| {
            format!(
                "configured role {} is missing its local Wasm for Candid extraction",
                spec.role
            )
        })?;
        extract_candid(candid_wasm_path, &spec.did_path)?;
        transforms.push(embed_candid_metadata(&spec.wasm_path, &spec.did_path)?);
    } else {
        remove_optional_file(&spec.did_path)?;
        transforms.push(ArtifactTransformOutput::not_requested(
            ArtifactTransformKind::CandidMetadata,
        ));
    }
    write_gzip_artifact(&spec.wasm_path, &spec.wasm_gz_path)?;

    Ok(CanisterArtifactBuildOutput {
        package_name: spec.package_name.clone(),
        package_version: spec.package_version.clone(),
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
    validate_artifact_role_deployable(config, canister_name)?;
    let validation = validate_declared_role_package(
        &context.config_path,
        config,
        &role,
        PackageValidationMode::Build,
    );
    resolve_canister_artifact_build_spec_from_validation(context, config, canister_name, validation)
}

pub fn resolve_canister_artifact_build_specs(
    context: &WorkspaceBuildContext,
    config: &canic_core::bootstrap::compiled::ConfigModel,
    roles: &[String],
) -> Result<Vec<CanisterArtifactBuildSpec>, Box<dyn std::error::Error>> {
    let role_ids = roles
        .iter()
        .map(|role| {
            validate_artifact_role_deployable(config, role)?;
            Ok(canic_core::ids::CanisterRole::owned(role.clone()))
        })
        .collect::<Result<Vec<_>, Box<dyn std::error::Error>>>()?;
    let validations = validate_declared_role_packages(
        &context.config_path,
        config,
        &role_ids,
        PackageValidationMode::Build,
    );

    roles
        .iter()
        .zip(validations)
        .map(|(role, validation)| {
            resolve_canister_artifact_build_spec_from_validation(context, config, role, validation)
        })
        .collect()
}

fn resolve_canister_artifact_build_spec_from_validation(
    context: &WorkspaceBuildContext,
    config: &canic_core::bootstrap::compiled::ConfigModel,
    canister_name: &str,
    validation: RolePackageValidation,
) -> Result<CanisterArtifactBuildSpec, Box<dyn std::error::Error>> {
    let evidence = match validation {
        RolePackageValidation::Supported(evidence) => evidence,
        RolePackageValidation::Unsupported(finding) => {
            return Err(format!("{}: {}", finding.code(), finding_detail(&finding)).into());
        }
    };
    require_declared_role_contract(config, &evidence)?;

    let artifact_root = context.artifact_root().join(canister_name);
    Ok(CanisterArtifactBuildSpec {
        role: canister_name.to_string(),
        package_name: evidence.role_package_name,
        package_version: evidence.role_package_version,
        package_manifest_path: evidence.role_manifest_path,
        cargo_workspace_root: evidence.cargo_workspace_root,
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

fn validate_artifact_role_deployable(
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
    if !config.deployable_roles().contains(&role) {
        let app = config.app_id().as_str();
        return Err(format!(
            "role {app}.{canister_name} is declared but not attached to topology; run `canic app role attach {app} {canister_name} --component-spec <component-spec>` before building an artifact"
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
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let mut command = canister_cargo_build_command(context, manifest_path, context.profile);

    let output = command.output()?;
    if !output.status.success() {
        return Err(format!(
            "cargo build failed for {}: {}",
            manifest_path.display(),
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    Ok(built_canister_wasm_path(
        context,
        context.profile,
        package_name,
    ))
}

fn run_canister_build_batch(
    context: &WorkspaceBuildContext,
    cargo_workspace_root: &Path,
    specs: &[&CanisterArtifactBuildSpec],
    profile: CanisterBuildProfile,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut command = canister_cargo_batch_command(context, cargo_workspace_root, specs, profile);

    let output = command.output()?;
    if output.status.success() {
        return Ok(());
    }
    let roles = specs
        .iter()
        .map(|spec| spec.role.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    Err(format!(
        "Cargo batch build failed for configured roles {roles}: {}",
        String::from_utf8_lossy(&output.stderr)
    )
    .into())
}

fn canister_cargo_batch_command(
    context: &WorkspaceBuildContext,
    cargo_workspace_root: &Path,
    specs: &[&CanisterArtifactBuildSpec],
    profile: CanisterBuildProfile,
) -> Command {
    let manifest_path = cargo_workspace_root.join("Cargo.toml");
    let mut command = canister_cargo_build_command(context, &manifest_path, profile);
    for spec in specs {
        command.arg("--package").arg(&spec.package_name);
    }
    command
}

fn canister_cargo_build_command(
    context: &WorkspaceBuildContext,
    manifest_path: &Path,
    profile: CanisterBuildProfile,
) -> Command {
    let build_context = context.with_profile(profile);
    let mut command = cargo_command();
    build_context.apply_to_command(&mut command);
    command
        .current_dir(&build_context.workspace_root)
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
        .args(profile.cargo_args());
    configure_canister_cargo_command(&mut command, &build_context.workspace_root);
    command
}

fn built_canister_wasm_path(
    context: &WorkspaceBuildContext,
    profile: CanisterBuildProfile,
    package_name: &str,
) -> PathBuf {
    canister_build_target_root(&context.workspace_root)
        .join(WASM_TARGET)
        .join(profile.target_dir_name())
        .join(format!("{}.wasm", package_name.replace('-', "_")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use canic_core::ids::BuildNetwork;

    #[test]
    fn configured_specs_group_into_one_cargo_command_per_workspace() {
        let specs = [
            build_spec("root", "canister-root", "/workspace"),
            build_spec("hub", "canister-hub", "/workspace"),
            build_spec("remote", "canister-remote", "/remote"),
        ];

        let groups = group_build_specs_by_workspace(&specs);

        assert_eq!(groups.len(), 2);
        assert_eq!(groups[Path::new("/workspace")].len(), 2);
        assert_eq!(groups[Path::new("/remote")].len(), 1);
    }

    #[test]
    fn configured_batch_command_selects_every_group_package_once() {
        let context = build_context();
        let specs = [
            build_spec("root", "canister-root", "/workspace"),
            build_spec("hub", "canister-hub", "/workspace"),
        ];
        let spec_refs = specs.iter().collect::<Vec<_>>();

        let command = canister_cargo_batch_command(
            &context,
            Path::new("/workspace"),
            &spec_refs,
            CanisterBuildProfile::Debug,
        );
        let args = command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect::<Vec<_>>();

        assert_eq!(
            args,
            [
                "build",
                "--manifest-path",
                "/workspace/Cargo.toml",
                "--target",
                WASM_TARGET,
                "--package",
                "canister-root",
                "--package",
                "canister-hub",
            ]
        );
    }

    #[test]
    fn repository_configured_specs_share_workspace_build_authority() {
        let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let config_path = workspace_root.join("apps/demo/canic.toml");
        let config = AppConfigSnapshot::load(&config_path).expect("load demo App config");
        let context = WorkspaceBuildContext {
            role: "root".to_string(),
            profile: CanisterBuildProfile::Fast,
            environment: "local".to_string(),
            build_network: BuildNetwork::Local,
            workspace_root: workspace_root.clone(),
            icp_root: workspace_root.clone(),
            config_path,
            local_replica: None,
            refresh_canonical_infrastructure_did: false,
            release_build_id: None,
        };
        let roles = ["root", "app", "user_hub", "user_shard"].map(str::to_string);

        let specs = resolve_canister_artifact_build_specs(&context, config.model(), &roles)
            .expect("resolve configured demo build specs");

        assert_eq!(specs.len(), roles.len());
        assert!(
            specs
                .iter()
                .all(|spec| spec.package_version == env!("CARGO_PKG_VERSION"))
        );
        assert!(specs.iter().all(|spec| {
            spec.cargo_workspace_root
                .canonicalize()
                .expect("canonical Cargo workspace")
                == workspace_root.canonicalize().expect("canonical repository")
        }));
    }

    fn build_context() -> WorkspaceBuildContext {
        WorkspaceBuildContext {
            role: "root".to_string(),
            profile: CanisterBuildProfile::Release,
            environment: "local".to_string(),
            build_network: BuildNetwork::Local,
            workspace_root: PathBuf::from("/workspace"),
            icp_root: PathBuf::from("/workspace"),
            config_path: PathBuf::from("/workspace/apps/demo/canic.toml"),
            local_replica: None,
            refresh_canonical_infrastructure_did: false,
            release_build_id: None,
        }
    }

    fn build_spec(
        role: &str,
        package_name: &str,
        cargo_workspace_root: &str,
    ) -> CanisterArtifactBuildSpec {
        let artifact_root = PathBuf::from("/artifacts").join(role);
        CanisterArtifactBuildSpec {
            role: role.to_string(),
            package_name: package_name.to_string(),
            package_version: "0.101.51".to_string(),
            package_manifest_path: PathBuf::from(cargo_workspace_root)
                .join(role)
                .join("Cargo.toml"),
            cargo_workspace_root: PathBuf::from(cargo_workspace_root),
            wasm_path: artifact_root.join(format!("{role}.wasm")),
            wasm_gz_path: artifact_root.join(format!("{role}.wasm.gz")),
            did_path: artifact_root.join(format!("{role}.did")),
            artifact_root,
        }
    }
}
