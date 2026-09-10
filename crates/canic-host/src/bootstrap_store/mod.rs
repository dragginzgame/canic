//! Build the host-generated Fleet Store artifact against exact Canic authority.

#[cfg(test)]
mod tests;

use crate::{
    bootstrap_candid::resolve_infrastructure_candid,
    build_toolchain::BuildToolchain,
    canister_build::{
        CanisterArtifactBuildOutput, WorkspaceBuildContext,
        cache::{
            canister_build_target_root, configure_canister_cargo_command,
            configure_declaration_command, declaration_target_root,
        },
        compiled::CompiledCanisterArtifact,
    },
    cargo_command,
    cargo_metadata::cargo_metadata,
    fleet_package::{
        self, FleetPackageSpec, append_infrastructure_profile_args, resolved_canic_package,
        resolved_wrapper_dependencies,
    },
    role_contract::{
        PackageValidationMode, RolePackageValidation, finding_detail,
        resolve_built_in_wasm_store_contract, validate_built_in_wasm_store_package,
    },
};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::Instant,
};

const WASM_STORE_ROLE: &str = "wasm_store";
const CANONICAL_WASM_STORE_CRATE_NAME: &str = "canister_wasm_store";
const GENERATED_WRAPPER_PACKAGE_NAME: &str = "canic-fleet-wasm-store";
#[derive(Clone, Debug)]
struct BootstrapWasmStoreSource {
    manifest_path: PathBuf,
    package_name: String,
    package_version: String,
    canonical_did_path: PathBuf,
}

// Build the implicit bootstrap `wasm_store` artifact and populate the canonical
// local ICP artifact paths for downstream/root builds.
pub fn build_bootstrap_wasm_store_artifact(
    context: &WorkspaceBuildContext,
    toolchain: &BuildToolchain,
) -> Result<CanisterArtifactBuildOutput, Box<dyn std::error::Error>> {
    compile_bootstrap_wasm_store_artifact(context)?.finish(toolchain)
}

/// Compile and capture the exact infrastructure input before finalization.
pub fn compile_bootstrap_wasm_store_artifact(
    context: &WorkspaceBuildContext,
) -> Result<CompiledCanisterArtifact, Box<dyn std::error::Error>> {
    let source = resolve_bootstrap_wasm_store_source(context)?;
    require_built_in_wasm_store_contract(&source.manifest_path)?;
    let artifact_root = context.artifact_root().join(WASM_STORE_ROLE);
    fs::create_dir_all(&artifact_root)?;

    let target_root = canister_build_target_root(&context.workspace_root);
    let built_wasm_path = target_root
        .join("wasm32-unknown-unknown")
        .join(context.profile.target_dir_name())
        .join(format!("{CANONICAL_WASM_STORE_CRATE_NAME}.wasm"));
    let candid = resolve_wasm_store_candid(context, &source)?;
    let capabilities = canic_core::role_contract::built_in_role_capabilities(
        canic_core::role_contract::BuiltInRoleKind::WasmStore,
    );
    let profile = canic_core::role_contract::derive_protocol_profile_hashes(
        &source.package_version,
        &canic_core::ids::CanisterRole::new(WASM_STORE_ROLE),
        &capabilities,
        &candid,
    );
    run_wasm_store_cargo_build(
        context,
        &source.manifest_path,
        Some(profile.protocol_profile_digest),
        false,
    )?;

    let wasm_path = artifact_root.join(format!("{WASM_STORE_ROLE}.wasm"));
    let wasm_gz_path = artifact_root.join(format!("{WASM_STORE_ROLE}.wasm.gz"));
    let did_path = artifact_root.join(format!("{WASM_STORE_ROLE}.did"));
    let profile_path = artifact_root.join(".build-profile");
    let output = CanisterArtifactBuildOutput {
        package_name: source.package_name,
        package_version: source.package_version.clone(),
        protocol_release_identity: source.package_version,
        protocol_role: canic_core::ids::CanisterRole::new(WASM_STORE_ROLE),
        protocol_capabilities: capabilities,
        artifact_root,
        wasm_path,
        wasm_gz_path,
        did_path,
        candid_sha256: profile.candid_sha256,
        protocol_profile_digest: profile.protocol_profile_digest,
        transforms: Vec::new(),
    };
    CompiledCanisterArtifact::capture(
        context,
        &built_wasm_path,
        candid,
        output,
        Some(profile_path),
    )
}

fn require_built_in_wasm_store_contract(
    manifest_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let evidence =
        match validate_built_in_wasm_store_package(manifest_path, PackageValidationMode::Build) {
            RolePackageValidation::Supported(evidence) => evidence,
            RolePackageValidation::Unsupported(finding) => {
                return Err(format!("{}: {}", finding.code(), finding_detail(&finding)).into());
            }
        };
    match resolve_built_in_wasm_store_contract(&evidence) {
        canic_core::role_contract::RoleContractResolution::Resolved { .. } => Ok(()),
        canic_core::role_contract::RoleContractResolution::Rejected { errors } => Err(errors
            .iter()
            .map(|finding| format!("{}: {}", finding.code(), finding_detail(finding)))
            .collect::<Vec<_>>()
            .join("; ")
            .into()),
    }
}

// Build the generated Store entrypoint for one target profile.
fn run_wasm_store_cargo_build(
    context: &WorkspaceBuildContext,
    manifest_path: &Path,
    protocol_profile_digest: Option<canic_core::role_contract::ProtocolProfileDigest>,
    force_candid_export: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut command = wasm_store_cargo_build_command(context, manifest_path, force_candid_export);
    if let Some(digest) = protocol_profile_digest {
        command.env(
            canic_core::role_contract::PROTOCOL_PROFILE_DIGEST_ENV,
            digest.to_string(),
        );
    }

    let started = Instant::now();
    let output = command.output()?;
    eprintln!(
        "Build phase {} bootstrap_store: {:.2}s",
        if force_candid_export {
            "declaration"
        } else {
            "runtime Cargo/link"
        },
        started.elapsed().as_secs_f64()
    );

    if output.status.success() {
        return Ok(());
    }

    Err(format!(
        "cargo build failed for bootstrap wasm_store: {}",
        String::from_utf8_lossy(&output.stderr)
    )
    .into())
}

fn wasm_store_cargo_build_command(
    context: &WorkspaceBuildContext,
    manifest_path: &Path,
    force_candid_export: bool,
) -> Command {
    let mut command = cargo_command();
    context.apply_to_command(&mut command);
    command
        .env_remove(canic_core::role_contract::PROTOCOL_PROFILE_DIGEST_ENV)
        .current_dir(&context.workspace_root)
        .env(
            canic_core::role_contract::CANONICAL_BUILD_MARKER_ENV,
            canic_core::role_contract::CANONICAL_BUILD_MARKER_VALUE,
        )
        .args([
            "build",
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
        configure_declaration_command(&mut command, context);
    }
    command
}

// Copy or regenerate the `.did` file that matches the built bootstrap artifact.
fn resolve_wasm_store_candid(
    context: &WorkspaceBuildContext,
    source: &BootstrapWasmStoreSource,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let selected_wasm_path = declaration_target_root(&context.workspace_root)
        .join("wasm32-unknown-unknown")
        .join(context.profile.target_dir_name())
        .join(format!("{CANONICAL_WASM_STORE_CRATE_NAME}.wasm"));

    resolve_infrastructure_candid(
        WASM_STORE_ROLE,
        &source.canonical_did_path,
        context.refresh_canonical_infrastructure_did,
        None,
        &selected_wasm_path,
        || run_wasm_store_cargo_build(context, &source.manifest_path, None, true),
    )
}

fn resolve_bootstrap_wasm_store_source(
    context: &WorkspaceBuildContext,
) -> Result<BootstrapWasmStoreSource, Box<dyn std::error::Error>> {
    let metadata = cargo_metadata(&context.workspace_root, true)?;
    let canic = resolved_canic_package(&metadata)?;
    let dependencies = resolved_wrapper_dependencies(&metadata, canic)?;
    let manifest =
        fleet_package::manifest_path(&context.config_path, GENERATED_WRAPPER_PACKAGE_NAME);
    let config_env = canic_core::role_contract::CANONICAL_BUILD_CONFIG_PATH_ENV;
    let build_script = format!(
        "fn main() {{ let config = std::env::var({config_env:?}).expect(\"Canic build configuration must be set\"); canic::build!(config); }}\n"
    );
    fleet_package::materialize(
        &manifest,
        &context.workspace_root,
        &canic.manifest_path,
        &dependencies,
        &FleetPackageSpec {
            package: GENERATED_WRAPPER_PACKAGE_NAME,
            crate_name: CANONICAL_WASM_STORE_CRATE_NAME,
            app: "wasm_store",
            role: WASM_STORE_ROLE,
            features: &["wasm-store-canister"],
            entrypoint: "canic::start_wasm_store!();\ncanic::finish!();\n",
            build_script: Some(&build_script),
        },
    )?;
    Ok(BootstrapWasmStoreSource {
        manifest_path: manifest,
        package_name: GENERATED_WRAPPER_PACKAGE_NAME.to_string(),
        package_version: canic.version.clone(),
        canonical_did_path: canic
            .manifest_path
            .parent()
            .ok_or("Canic manifest has no parent")?
            .join("candid/wasm_store.did"),
    })
}
