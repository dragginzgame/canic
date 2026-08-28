use std::{
    collections::BTreeMap,
    env,
    fmt::{self, Display, Formatter},
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use crate::{
    artifact_io::{
        embed_candid_metadata, enforce_wasm_code_section_limit, maybe_shrink_wasm_artifact,
        write_gzip_artifact, write_wasm_artifact,
    },
    bootstrap_coordinator::build_bootstrap_fleet_coordinator_artifact,
    bootstrap_store::build_bootstrap_wasm_store_artifact,
    cargo_command,
    durable_io::write_bytes,
    release_set::AppConfigSnapshot,
    role_contract::{
        PackageValidationMode, RoleCargoGraphEvidence, RolePackageValidation, finding_detail,
        resolve_declared_role_package_contract, validate_declared_role_package,
        validate_declared_role_packages,
    },
    should_embed_candid_metadata,
};

use super::{
    CanisterBuildProfile, WorkspaceBuildContext,
    cache::{
        canister_build_target_root, configure_canister_cargo_command, lock_canister_build_target,
    },
    candid::{extract_candid_bytes, remove_stale_icp_candid_sidecars},
    model::{
        ArtifactTransformKind, ArtifactTransformOutput, CanisterArtifactBuildOutput,
        CanisterArtifactBuildSpec, CanisterArtifactSource, ConfiguredCanisterArtifactBuildOutput,
        WASM_TARGET,
    },
};

pub fn build_workspace_canister_artifact(
    context: &WorkspaceBuildContext,
) -> Result<CanisterArtifactBuildOutput, Box<dyn std::error::Error>> {
    let _build_target_lock = lock_canister_build_target(&context.workspace_root)?;
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

    let release_wasm_path = run_canister_profile_candid_build(
        context,
        &spec.package_manifest_path,
        &spec.package_name,
    )?;
    let candid = extract_candid_bytes(&release_wasm_path)?;
    let profile = canic_core::role_contract::derive_protocol_profile_hashes(
        &spec.canic_version,
        &canic_core::ids::CanisterRole::owned(spec.role.clone()),
        &spec.capabilities,
        &candid,
    );
    let release_wasm_path = run_canister_build(
        context,
        &spec.package_manifest_path,
        &spec.package_name,
        Some(profile.protocol_profile_digest),
    )?;
    finish_canister_artifact_output(context, spec, &release_wasm_path, &candid, profile)
}

/// Build all admitted configured roles in one Cargo invocation per workspace and profile.
pub fn build_workspace_canister_artifacts_from_specs(
    context: &WorkspaceBuildContext,
    specs: &[CanisterArtifactBuildSpec],
) -> Result<Vec<CanisterArtifactBuildOutput>, Box<dyn std::error::Error>> {
    if specs.is_empty() {
        return Ok(Vec::new());
    }
    let _build_target_lock = lock_canister_build_target(&context.workspace_root)?;
    let embed_candid = should_embed_candid_metadata(context.build_network);

    for spec in specs {
        prepare_canister_artifact_output(spec)?;
    }
    let workspace_groups = group_build_specs_by_workspace(specs);

    if embed_candid {
        for (cargo_workspace_root, group) in &workspace_groups {
            run_canister_build_batch(context, cargo_workspace_root, group, context.profile)?;
        }
    } else {
        for spec in specs {
            run_canister_profile_candid_build(
                context,
                &spec.package_manifest_path,
                &spec.package_name,
            )?;
        }
    }
    let mut profiles = Vec::with_capacity(specs.len());
    for spec in specs {
        let release_wasm_path =
            built_canister_wasm_path(context, context.profile, spec.package_name.as_str());
        let candid = extract_candid_bytes(&release_wasm_path)?;
        let profile = canic_core::role_contract::derive_protocol_profile_hashes(
            &spec.canic_version,
            &canic_core::ids::CanisterRole::owned(spec.role.clone()),
            &spec.capabilities,
            &candid,
        );
        run_canister_build(
            context,
            &spec.package_manifest_path,
            &spec.package_name,
            Some(profile.protocol_profile_digest),
        )?;
        profiles.push((candid, profile));
    }
    std::thread::scope(|scope| {
        let mut handles = Vec::with_capacity(specs.len());
        for (spec, (candid, profile)) in specs.iter().zip(profiles) {
            handles.push(scope.spawn(move || {
                let release_wasm_path =
                    built_canister_wasm_path(context, context.profile, spec.package_name.as_str());
                finish_canister_artifact_output(context, spec, &release_wasm_path, &candid, profile)
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
    candid: &[u8],
    profile: canic_core::role_contract::ProtocolProfileHashes,
) -> Result<CanisterArtifactBuildOutput, Box<dyn std::error::Error>> {
    let mut transforms = Vec::new();
    write_wasm_artifact(release_wasm_path, &spec.wasm_path)?;
    transforms.push(maybe_shrink_wasm_artifact(&spec.wasm_path)?);
    write_bytes(&spec.did_path, candid)?;

    if should_embed_candid_metadata(context.build_network) {
        transforms.push(embed_candid_metadata(&spec.wasm_path, &spec.did_path)?);
    } else {
        transforms.push(ArtifactTransformOutput::not_requested(
            ArtifactTransformKind::CandidMetadata,
        ));
    }
    enforce_wasm_code_section_limit(&spec.wasm_path)?;
    write_gzip_artifact(&spec.wasm_path, &spec.wasm_gz_path)?;

    Ok(CanisterArtifactBuildOutput {
        package_name: spec.package_name.clone(),
        package_version: spec.package_version.clone(),
        protocol_release_identity: spec.canic_version.clone(),
        protocol_role: canic_core::ids::CanisterRole::owned(spec.role.clone()),
        protocol_capabilities: spec.capabilities.clone(),
        artifact_root: spec.artifact_root.clone(),
        wasm_path: spec.wasm_path.clone(),
        wasm_gz_path: spec.wasm_gz_path.clone(),
        did_path: spec.did_path.clone(),
        candid_sha256: profile.candid_sha256,
        protocol_profile_digest: profile.protocol_profile_digest,
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
    let mut failures = Vec::new();
    let mut admitted = Vec::with_capacity(roles.len());
    for role in roles {
        match validate_artifact_role_deployable(config, role) {
            Ok(()) => admitted.push((role, canic_core::ids::CanisterRole::owned(role.clone()))),
            Err(source) => failures.push(ConfiguredBuildSpecFailure {
                role: role.clone(),
                source,
            }),
        }
    }
    if !failures.is_empty() {
        return Err(ConfiguredBuildSpecFailures(failures).into());
    }
    let role_ids = admitted
        .iter()
        .map(|(_, role_id)| role_id.clone())
        .collect::<Vec<_>>();
    let validations = validate_declared_role_packages(
        &context.config_path,
        config,
        &role_ids,
        PackageValidationMode::Build,
    );

    let mut specs = Vec::with_capacity(roles.len());
    for ((role, _), validation) in admitted.into_iter().zip(validations) {
        match resolve_canister_artifact_build_spec_from_validation(
            context, config, role, validation,
        ) {
            Ok(spec) => specs.push(spec),
            Err(source) => failures.push(ConfiguredBuildSpecFailure {
                role: role.clone(),
                source,
            }),
        }
    }
    if failures.is_empty() {
        Ok(specs)
    } else {
        Err(ConfiguredBuildSpecFailures(failures).into())
    }
}

#[derive(Debug)]
struct ConfiguredBuildSpecFailures(Vec<ConfiguredBuildSpecFailure>);

#[derive(Debug)]
struct ConfiguredBuildSpecFailure {
    role: String,
    source: Box<dyn std::error::Error>,
}

impl Display for ConfiguredBuildSpecFailure {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.role, self.source)
    }
}

impl Display for ConfiguredBuildSpecFailures {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "configured build specification failed: {}",
            self.0
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("; ")
        )
    }
}

impl std::error::Error for ConfiguredBuildSpecFailures {}

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
    let contract = require_declared_role_contract(config, &evidence)?;

    let artifact_root = context.artifact_root().join(canister_name);
    Ok(CanisterArtifactBuildSpec {
        role: canister_name.to_string(),
        package_name: evidence.role_package_name,
        package_version: evidence.role_package_version,
        canic_version: evidence.canic_version,
        capabilities: contract.capabilities,
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
) -> Result<canic_core::role_contract::ResolvedRoleContract, Box<dyn std::error::Error>> {
    match resolve_declared_role_package_contract(config, evidence) {
        canic_core::role_contract::RoleContractResolution::Resolved { contract } => Ok(contract),
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
    protocol_profile_digest: Option<canic_core::role_contract::ProtocolProfileDigest>,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let mut command = canister_cargo_build_command(context, manifest_path, context.profile);
    command.env_remove(canic_core::role_contract::PROTOCOL_PROFILE_DIGEST_ENV);
    if let Some(digest) = protocol_profile_digest {
        command.env(
            canic_core::role_contract::PROTOCOL_PROFILE_DIGEST_ENV,
            digest.to_string(),
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

    Ok(built_canister_wasm_path(
        context,
        context.profile,
        package_name,
    ))
}

fn run_canister_profile_candid_build(
    context: &WorkspaceBuildContext,
    manifest_path: &Path,
    package_name: &str,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let mut command = canister_profile_candid_command(context, manifest_path, context.profile);
    let output = command.output()?;
    if !output.status.success() {
        return Err(format!(
            "profile Candid build failed for {}: {}",
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

fn canister_profile_candid_command(
    context: &WorkspaceBuildContext,
    manifest_path: &Path,
    profile: CanisterBuildProfile,
) -> Command {
    let mut command = canister_cargo_command(context, manifest_path, profile, "rustc");
    command.args([
        "--lib",
        "--",
        "--cfg",
        "canic_export_candid",
        "--check-cfg=cfg(canic_export_candid)",
    ]);
    command
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
    command.env(canic_core::role_contract::CANONICAL_CANDID_BUILD_ENV, "1");
    for spec in specs {
        command.arg("--package").arg(&spec.package_name);
    }
    command
}

pub(super) fn canister_cargo_build_command(
    context: &WorkspaceBuildContext,
    manifest_path: &Path,
    profile: CanisterBuildProfile,
) -> Command {
    canister_cargo_command(context, manifest_path, profile, "build")
}

fn canister_cargo_command(
    context: &WorkspaceBuildContext,
    manifest_path: &Path,
    profile: CanisterBuildProfile,
    cargo_subcommand: &str,
) -> Command {
    let build_context = context.with_profile(profile);
    let mut command = cargo_command();
    build_context.apply_to_command(&mut command);
    command
        .env_remove(canic_core::role_contract::PROTOCOL_PROFILE_DIGEST_ENV)
        .current_dir(&build_context.workspace_root)
        .env(
            canic_core::role_contract::CANONICAL_BUILD_MARKER_ENV,
            canic_core::role_contract::CANONICAL_BUILD_MARKER_VALUE,
        )
        .args([
            cargo_subcommand,
            "--locked",
            "--keep-going",
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
        let environment = command.get_envs().collect::<BTreeMap<_, _>>();

        assert_eq!(
            args,
            [
                "build",
                "--locked",
                "--keep-going",
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
        assert_eq!(
            environment.get(std::ffi::OsStr::new(
                canic_core::role_contract::CANONICAL_CANDID_BUILD_ENV
            )),
            Some(&Some(std::ffi::OsStr::new("1")))
        );
    }

    #[test]
    fn profile_candid_pass_is_explicit_for_nonlocal_binding_derivation() {
        let context = build_context();
        let command = canister_profile_candid_command(
            &context,
            Path::new("/workspace/app/Cargo.toml"),
            CanisterBuildProfile::Fast,
        );
        let args = command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect::<Vec<_>>();

        assert_eq!(args.first().map(String::as_str), Some("rustc"));
        assert!(
            args.windows(2)
                .any(|args| args == ["--cfg", "canic_export_candid"])
        );
        assert!(args.contains(&"--check-cfg=cfg(canic_export_candid)".to_string()));
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
        let app = specs
            .iter()
            .find(|spec| spec.role == "app")
            .expect("app spec");
        let user_hub = specs
            .iter()
            .find(|spec| spec.role == "user_hub")
            .expect("user_hub spec");
        assert!(
            !app.capabilities
                .contains(&canic_core::role_contract::RoleCapabilityKey::AutomaticTopup)
        );
        assert!(
            user_hub
                .capabilities
                .contains(&canic_core::role_contract::RoleCapabilityKey::AutomaticTopup)
        );
    }

    #[test]
    fn configured_spec_resolution_reports_every_invalid_role() {
        let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let config_path = workspace_root.join("apps/demo/canic.toml");
        let config = AppConfigSnapshot::load(&config_path).expect("load demo App config");
        let context = WorkspaceBuildContext {
            role: "root".to_string(),
            profile: CanisterBuildProfile::Fast,
            environment: "local".to_string(),
            build_network: BuildNetwork::Local,
            workspace_root: workspace_root.clone(),
            icp_root: workspace_root,
            config_path,
            local_replica: None,
            refresh_canonical_infrastructure_did: false,
            release_build_id: None,
        };
        let roles = ["missing-first", "missing-second"].map(str::to_string);

        let error = resolve_canister_artifact_build_specs(&context, config.model(), &roles)
            .expect_err("both invalid configured roles must fail");
        let failures = error
            .downcast_ref::<ConfiguredBuildSpecFailures>()
            .expect("typed configured build failure");
        let failed_roles = failures
            .0
            .iter()
            .map(|failure| failure.role.as_str())
            .collect::<Vec<_>>();

        assert_eq!(failed_roles, ["missing-first", "missing-second"]);
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
            canic_version: "0.101.51".to_string(),
            capabilities: std::collections::BTreeSet::new(),
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
