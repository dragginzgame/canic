//! Module: bootstrap_coordinator
//!
//! Responsibility: build the exact built-in Fleet Coordinator source package and artifact.
//! Does not own: Coordinator placement, installation effects, or Fleet Registry mutation.
//! Boundary: resolves the selected Canic package and emits one qualified current-build Wasm.

#[cfg(test)]
mod tests;

use crate::{
    artifact_io::{
        embed_candid_metadata, enforce_wasm_code_section_limit, maybe_shrink_wasm_artifact,
        write_gzip_artifact, write_wasm_artifact,
    },
    bootstrap_candid::materialize_infrastructure_candid,
    bootstrap_store::{
        append_profile_config_args, generated_wasm_store_wrapper_patch_table,
        registry_package_version_suffix, render_profile, require_package_manifest_identity,
        resolved_canic_package, resolved_wrapper_dependencies,
    },
    canister_build::{
        ArtifactTransformKind, ArtifactTransformOutput, CanisterArtifactBuildOutput,
        CanisterBuildProfile, WorkspaceBuildContext,
        cache::{canister_build_target_root, configure_canister_cargo_command},
        extract_candid_bytes,
    },
    cargo_command,
    cargo_metadata::{CargoMetadata, CargoMetadataPackage, cargo_metadata},
    durable_io::write_bytes,
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
const CANONICAL_PACKAGE_NAME: &str = "canic-fleet-coordinator";
const CANONICAL_FLEET_COORDINATOR_DID_FILE: &str = "fleet_coordinator.did";
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

#[derive(Clone, Debug)]
struct BootstrapFleetCoordinatorSource {
    manifest_path: PathBuf,
    package_name: String,
    package_version: String,
    canonical_did_path: Option<PathBuf>,
}

/// Build the dedicated Fleet Coordinator wrapper selected from the exact Canic dependency graph.
pub fn build_bootstrap_fleet_coordinator_artifact(
    context: &WorkspaceBuildContext,
) -> Result<CanisterArtifactBuildOutput, Box<dyn std::error::Error>> {
    let source = resolve_bootstrap_fleet_coordinator_source(context)?;
    require_built_in_fleet_coordinator_contract(&source.manifest_path)?;
    run_coordinator_cargo_build(context, &source.manifest_path, None, true)?;

    let built_wasm_path = canister_build_target_root(&context.workspace_root)
        .join("wasm32-unknown-unknown")
        .join(context.profile.target_dir_name())
        .join(format!("{GENERATED_WRAPPER_CRATE_NAME}.wasm"));
    let candid = extract_candid_bytes(&built_wasm_path)?;
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

    write_wasm_artifact(&built_wasm_path, &wasm_path)?;
    let mut transforms = vec![maybe_shrink_wasm_artifact(&wasm_path)?];
    if should_embed_candid_metadata(context.build_network) {
        ensure_fleet_coordinator_did(context, &source, &did_path)?;
    } else {
        write_bytes(&did_path, &candid)?;
    }
    if fs::read(&did_path)? != candid {
        return Err(
            "Fleet Coordinator materialized Candid differs from its compiled profile".into(),
        );
    }
    if should_embed_candid_metadata(context.build_network) {
        transforms.push(embed_candid_metadata(&wasm_path, &did_path)?);
    } else {
        transforms.push(ArtifactTransformOutput::not_requested(
            ArtifactTransformKind::CandidMetadata,
        ));
    }
    enforce_wasm_code_section_limit(&wasm_path)?;
    write_gzip_artifact(&wasm_path, &wasm_gz_path)?;

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

// Resolve the canonical published/workspace Coordinator source or fall back
// to a generated runtime-only wrapper when downstreams only depend on `canic`.
fn resolve_bootstrap_fleet_coordinator_source(
    context: &WorkspaceBuildContext,
) -> Result<BootstrapFleetCoordinatorSource, Box<dyn std::error::Error>> {
    let metadata = cargo_metadata(&context.workspace_root, true)?;
    let canic_package = resolved_canic_package(&metadata)?;
    if let Some(source) = resolve_canonical_fleet_coordinator_source(&metadata, canic_package)? {
        return Ok(source);
    }

    let manifest_path = ensure_generated_wrapper(context)?;
    Ok(BootstrapFleetCoordinatorSource {
        manifest_path,
        package_name: GENERATED_WRAPPER_PACKAGE_NAME.to_string(),
        package_version: canic_package.version.clone(),
        canonical_did_path: None,
    })
}

// Prefer the exact resolved canonical package, then the exact sibling source
// belonging to the selected Canic package.
fn resolve_canonical_fleet_coordinator_source(
    metadata: &CargoMetadata,
    canic_package: &CargoMetadataPackage,
) -> Result<Option<BootstrapFleetCoordinatorSource>, Box<dyn std::error::Error>> {
    let matches = metadata
        .packages
        .iter()
        .filter(|package| {
            package.name == CANONICAL_PACKAGE_NAME
                && package.version == canic_package.version
                && package.source == canic_package.source
        })
        .collect::<Vec<_>>();
    if matches.len() > 1 {
        return Err(
            "Fleet Coordinator source resolved more than once for the selected Canic package"
                .into(),
        );
    }
    if let [package] = matches.as_slice() {
        let source_root = package
            .manifest_path
            .parent()
            .expect("manifest path must have parent");
        return Ok(Some(BootstrapFleetCoordinatorSource {
            manifest_path: package.manifest_path.clone(),
            package_name: package.name.clone(),
            package_version: package.version.clone(),
            canonical_did_path: Some(source_root.join(CANONICAL_FLEET_COORDINATOR_DID_FILE)),
        }));
    }

    let canic_root = canic_package
        .manifest_path
        .parent()
        .expect("Canic manifest path must have parent");
    let sibling_root = canic_root.parent().expect("Canic root must have parent");
    let registry_version = registry_package_version_suffix(&canic_package.manifest_path, "canic")
        .filter(|version| *version == canic_package.version);
    let sibling_dir = registry_version.map_or_else(
        || CANONICAL_PACKAGE_NAME.to_string(),
        |version| format!("{CANONICAL_PACKAGE_NAME}-{version}"),
    );
    let sibling_manifest = sibling_root.join(sibling_dir).join("Cargo.toml");
    if sibling_manifest.is_file() {
        require_package_manifest_identity(
            &sibling_manifest,
            CANONICAL_PACKAGE_NAME,
            &canic_package.version,
        )?;
        let source_root = sibling_manifest
            .parent()
            .expect("manifest path must have parent");
        return Ok(Some(BootstrapFleetCoordinatorSource {
            manifest_path: sibling_manifest.clone(),
            package_name: CANONICAL_PACKAGE_NAME.to_string(),
            package_version: canic_package.version.clone(),
            canonical_did_path: Some(source_root.join(CANONICAL_FLEET_COORDINATOR_DID_FILE)),
        }));
    }

    Ok(None)
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
    command
        .env_remove(canic_core::role_contract::PROTOCOL_PROFILE_DIGEST_ENV)
        .current_dir(&context.workspace_root)
        .args([
            if force_candid_export {
                "rustc"
            } else {
                "build"
            },
            "--locked",
            "--manifest-path",
            &manifest_path.display().to_string(),
            "--target",
            "wasm32-unknown-unknown",
        ]);
    configure_canister_cargo_command(&mut command, &context.workspace_root);
    append_coordinator_profile_config_args(&mut command, context.profile);
    command.args(context.profile.cargo_args());
    if force_candid_export {
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

fn ensure_fleet_coordinator_did(
    context: &WorkspaceBuildContext,
    source: &BootstrapFleetCoordinatorSource,
    artifact_did_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let selected_wasm_path = canister_build_target_root(&context.workspace_root)
        .join("wasm32-unknown-unknown")
        .join(context.profile.target_dir_name())
        .join(format!("{GENERATED_WRAPPER_CRATE_NAME}.wasm"));

    materialize_infrastructure_candid(
        FLEET_COORDINATOR_ROLE,
        source.canonical_did_path.as_deref(),
        artifact_did_path,
        context.refresh_canonical_infrastructure_did,
        &selected_wasm_path,
        || Ok(()),
    )
}
