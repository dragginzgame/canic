//! Module: bootstrap_coordinator
//!
//! Responsibility: build the exact built-in Fleet Coordinator source package and artifact.
//! Does not own: Coordinator placement, installation effects, or Fleet Registry mutation.
//! Boundary: resolves the selected Canic package and emits one qualified current-build Wasm.

#[cfg(test)]
mod tests;

use crate::{
    artifact_io::{WasmArtifactFinalization, finalize_wasm_artifact},
    bootstrap_candid::resolve_infrastructure_candid,
    build_toolchain::BuildToolchain,
    canister_build::{
        CanisterArtifactBuildOutput, WorkspaceBuildContext,
        cache::{canister_build_target_root, configure_canister_cargo_command},
    },
    cargo_command,
    cargo_metadata::cargo_metadata,
    fleet_package::{
        self, FleetPackageSpec, append_infrastructure_profile_args, resolved_canic_package,
        resolved_wrapper_dependencies,
    },
    role_contract::{
        PackageValidationMode, RolePackageValidation, finding_detail,
        resolve_built_in_fleet_coordinator_contract, validate_built_in_fleet_coordinator_package,
    },
    should_embed_candid_metadata,
};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

const FLEET_COORDINATOR_ROLE: &str = "fleet_coordinator";
const GENERATED_WRAPPER_PACKAGE_NAME: &str = "canic-fleet-coordinator";
const GENERATED_WRAPPER_CRATE_NAME: &str = "canister_fleet_coordinator";

#[derive(Clone, Debug)]
struct BootstrapFleetCoordinatorSource {
    manifest_path: PathBuf,
    package_name: String,
    package_version: String,
    canonical_did_path: PathBuf,
}

/// Build the dedicated Fleet Coordinator wrapper selected from the exact Canic dependency graph.
pub fn build_bootstrap_fleet_coordinator_artifact(
    context: &WorkspaceBuildContext,
    toolchain: &BuildToolchain,
) -> Result<CanisterArtifactBuildOutput, Box<dyn std::error::Error>> {
    let source = resolve_bootstrap_fleet_coordinator_source(context)?;
    require_built_in_fleet_coordinator_contract(&source.manifest_path)?;
    let built_wasm_path = canister_build_target_root(&context.workspace_root)
        .join("wasm32-unknown-unknown")
        .join(context.profile.target_dir_name())
        .join(format!("{GENERATED_WRAPPER_CRATE_NAME}.wasm"));
    let candid = resolve_fleet_coordinator_candid(context, &source, &built_wasm_path)?;
    let capabilities = canic_core::role_contract::built_in_role_capabilities(
        canic_core::role_contract::BuiltInRoleKind::FleetCoordinator,
    );
    let profile = canic_core::role_contract::derive_protocol_profile_hashes(
        &source.package_version,
        &canic_core::ids::CanisterRole::new(FLEET_COORDINATOR_ROLE),
        &capabilities,
        &candid,
    );
    run_coordinator_cargo_build(
        context,
        &source.manifest_path,
        Some(profile.protocol_profile_digest),
        false,
    )?;
    let artifact_root = context.artifact_root().join(FLEET_COORDINATOR_ROLE);
    fs::create_dir_all(&artifact_root)?;
    let wasm_path = artifact_root.join(format!("{FLEET_COORDINATOR_ROLE}.wasm"));
    let wasm_gz_path = artifact_root.join(format!("{FLEET_COORDINATOR_ROLE}.wasm.gz"));
    let did_path = artifact_root.join(format!("{FLEET_COORDINATOR_ROLE}.did"));

    let embed_candid = should_embed_candid_metadata(context.build_network);
    let transforms = finalize_wasm_artifact(
        &WasmArtifactFinalization {
            profile: context.profile,
            build_network: context.build_network,
            embed_candid,
            validate_sidecar_only: false,
            source_wasm_path: &built_wasm_path,
            candid: &candid,
            wasm_path: &wasm_path,
            did_path: &did_path,
            wasm_gz_path: &wasm_gz_path,
        },
        toolchain,
    )?;

    Ok(CanisterArtifactBuildOutput {
        package_name: source.package_name,
        package_version: source.package_version.clone(),
        protocol_release_identity: source.package_version,
        protocol_role: canic_core::ids::CanisterRole::new(FLEET_COORDINATOR_ROLE),
        protocol_capabilities: capabilities,
        artifact_root,
        wasm_path,
        wasm_gz_path,
        did_path,
        candid_sha256: profile.candid_sha256,
        protocol_profile_digest: profile.protocol_profile_digest,
        transforms,
    })
}

fn resolve_bootstrap_fleet_coordinator_source(
    context: &WorkspaceBuildContext,
) -> Result<BootstrapFleetCoordinatorSource, Box<dyn std::error::Error>> {
    let metadata = cargo_metadata(&context.workspace_root, true)?;
    let canic = resolved_canic_package(&metadata)?;
    let dependencies = resolved_wrapper_dependencies(&metadata, canic)?;
    let manifest =
        fleet_package::manifest_path(&context.config_path, GENERATED_WRAPPER_PACKAGE_NAME);
    fleet_package::materialize(
        &manifest,
        &context.workspace_root,
        &canic.manifest_path,
        &dependencies,
        &FleetPackageSpec {
            package: GENERATED_WRAPPER_PACKAGE_NAME,
            crate_name: GENERATED_WRAPPER_CRATE_NAME,
            app: FLEET_COORDINATOR_ROLE,
            role: FLEET_COORDINATOR_ROLE,
            features: &["fleet-coordinator-canister"],
            entrypoint: "canic::start_fleet_coordinator!();\ncanic::finish!();\n",
            build_script: None,
        },
    )?;
    Ok(BootstrapFleetCoordinatorSource {
        manifest_path: manifest,
        package_name: GENERATED_WRAPPER_PACKAGE_NAME.to_string(),
        package_version: canic.version.clone(),
        canonical_did_path: canic
            .manifest_path
            .parent()
            .ok_or("Canic manifest has no parent")?
            .join("candid/fleet_coordinator.did"),
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

fn run_coordinator_cargo_build(
    context: &WorkspaceBuildContext,
    manifest_path: &Path,
    protocol_profile_digest: Option<canic_core::role_contract::ProtocolProfileDigest>,
    force_candid_export: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut command = coordinator_cargo_build_command(context, manifest_path, force_candid_export);
    if let Some(digest) = protocol_profile_digest {
        command.env(
            canic_core::role_contract::PROTOCOL_PROFILE_DIGEST_ENV,
            digest.to_string(),
        );
    }
    let output = command.output()?;
    if output.status.success() {
        return Ok(());
    }
    Err(format!(
        "Cargo failed to build the Fleet Coordinator: {}",
        String::from_utf8_lossy(&output.stderr)
    )
    .into())
}

fn coordinator_cargo_build_command(
    context: &WorkspaceBuildContext,
    manifest_path: &Path,
    force_candid_export: bool,
) -> Command {
    let mut command = cargo_command();
    context.apply_to_command(&mut command);
    let cargo_subcommand = if force_candid_export {
        "rustc"
    } else {
        "build"
    };
    command
        .env_remove(canic_core::role_contract::PROTOCOL_PROFILE_DIGEST_ENV)
        .current_dir(&context.workspace_root)
        .args([
            cargo_subcommand,
            "--locked",
            "--manifest-path",
            &manifest_path.display().to_string(),
            "--target",
            "wasm32-unknown-unknown",
        ]);
    configure_canister_cargo_command(&mut command, &context.workspace_root);
    append_infrastructure_profile_args(&mut command, context.profile);
    command.args(context.profile.cargo_args());
    if force_candid_export {
        command.env(canic_core::role_contract::CANONICAL_CANDID_BUILD_ENV, "1");
        command.args([
            "--lib",
            "--",
            "--cfg",
            "canic_export_candid",
            "--check-cfg=cfg(canic_export_candid)",
        ]);
    }
    command
}

fn resolve_fleet_coordinator_candid(
    context: &WorkspaceBuildContext,
    source: &BootstrapFleetCoordinatorSource,
    built_wasm_path: &Path,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    resolve_infrastructure_candid(
        FLEET_COORDINATOR_ROLE,
        &source.canonical_did_path,
        context.refresh_canonical_infrastructure_did,
        None,
        built_wasm_path,
        || run_coordinator_cargo_build(context, &source.manifest_path, None, true),
    )
}
