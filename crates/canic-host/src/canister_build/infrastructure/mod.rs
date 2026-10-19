//! Module: canister_build::infrastructure
//!
//! Responsibility: build all three infrastructure roles through one admitted pipeline.
//! Does not own: Coordinator placement, installation effects, or Fleet Registry mutation.
//! Boundary: resolves the selected Canic package and emits one qualified current-build Wasm.

#[cfg(test)]
mod tests;

use crate::{
    bootstrap_candid::resolve_infrastructure_candid,
    build_toolchain::BuildToolchain,
    canister_build::{
        CanisterArtifactBuildOutput, WorkspaceBuildContext,
        cache::{
            CargoBuildOutput, CargoBuildProgress, declaration_target_root,
            output_canister_cargo_command,
        },
        compiled::CompiledCanisterArtifact,
    },
    cargo_observation::cargo_metadata,
    fleet_package,
    fleet_package::{
        FleetPackageSpec, append_infrastructure_profile_args, resolved_canic_package,
        resolved_wrapper_dependencies,
    },
    role_contract::{
        PackageValidationMode, RolePackageValidation, finding_detail,
        resolve_built_in_fleet_coordinator_contract, resolve_built_in_wasm_store_contract,
        validate_built_in_fleet_coordinator_package, validate_built_in_wasm_store_package,
    },
};
use ic_build::cargo::execution::{CargoInvocation, CargoRecipe};
use std::{
    fs,
    path::{Path, PathBuf},
    time::Instant,
};

/// Canic-owned infrastructure source and declaration policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum InfrastructureRole {
    Coordinator,
    Root,
    Store,
}

impl InfrastructureRole {
    pub(super) const fn name(self) -> &'static str {
        match self {
            Self::Coordinator => "fleet_coordinator",
            Self::Root => "root",
            Self::Store => "wasm_store",
        }
    }

    const fn package(self) -> &'static str {
        match self {
            Self::Coordinator => "canic-fleet-coordinator",
            Self::Root => "canic-fleet-root",
            Self::Store => "canic-fleet-wasm-store",
        }
    }

    const fn crate_name(self) -> &'static str {
        match self {
            Self::Coordinator => "canister_fleet_coordinator",
            Self::Root => "canic_fleet_root",
            Self::Store => "canister_wasm_store",
        }
    }
}

#[derive(Clone, Debug)]
struct InfrastructureSource {
    manifest_path: PathBuf,
    package_name: String,
    package_version: String,
    canonical_did_path: Option<PathBuf>,
    capabilities: std::collections::BTreeSet<canic_core::role_contract::RoleCapabilityKey>,
}

/// Materialize and admit generated package inputs before an artifact cache snapshots them.
pub(super) fn prepare(
    role: InfrastructureRole,
    context: &WorkspaceBuildContext,
) -> Result<(), Box<dyn std::error::Error>> {
    let source = resolve_source(role, context)?;
    require_contract(role, &source.manifest_path)
}

/// Build a qualified infrastructure wrapper from the selected Canic dependency graph.
pub(super) fn build(
    role: InfrastructureRole,
    context: &WorkspaceBuildContext,
    toolchain: &BuildToolchain,
) -> Result<CanisterArtifactBuildOutput, Box<dyn std::error::Error>> {
    compile(role, context)?.finish(toolchain)
}

/// Compile and capture the exact infrastructure input before finalization.
pub(super) fn compile(
    role: InfrastructureRole,
    context: &WorkspaceBuildContext,
) -> Result<CompiledCanisterArtifact, Box<dyn std::error::Error>> {
    let source = resolve_source(role, context)?;
    require_contract(role, &source.manifest_path)?;
    let candid = resolve_candid(role, context, &source)?;
    let capabilities = source.capabilities.clone();
    let profile = canic_core::role_contract::derive_protocol_profile_hashes(
        &source.package_version,
        &canic_contracts::ids::CanisterRole::new(role.name()),
        &capabilities,
        &candid,
    );
    let built_wasm_path = run_build(
        role,
        context,
        &source.manifest_path,
        Some(profile.protocol_profile_digest),
        false,
    )?;
    let artifact_root = context.artifact_root().join(role.name());
    fs::create_dir_all(&artifact_root)?;
    let wasm_path = artifact_root.join(format!("{}.wasm", role.name()));
    let wasm_gz_path = artifact_root.join(format!("{}.wasm.gz", role.name()));
    let did_path = artifact_root.join(format!("{}.did", role.name()));

    let output = CanisterArtifactBuildOutput {
        package_name: source.package_name,
        package_version: source.package_version.clone(),
        protocol_release_identity: source.package_version,
        protocol_role: canic_contracts::ids::CanisterRole::new(role.name()),
        protocol_capabilities: capabilities,
        artifact_root,
        wasm_path,
        wasm_gz_path,
        did_path,
        candid_sha256: profile.candid_sha256,
        protocol_profile_digest: profile.protocol_profile_digest,
        transforms: Vec::new(),
    };
    let marker =
        (role == InfrastructureRole::Store).then(|| output.artifact_root.join(".build-profile"));
    super::candid::remove_stale_icp_candid_sidecars(&output.artifact_root)?;
    CompiledCanisterArtifact::capture(context, &built_wasm_path, candid, output, marker)
}

fn resolve_source(
    role: InfrastructureRole,
    context: &WorkspaceBuildContext,
) -> Result<InfrastructureSource, Box<dyn std::error::Error>> {
    if role == InfrastructureRole::Root {
        let config = crate::release_set::AppConfigSnapshot::load(&context.config_path)?;
        let spec = super::artifact::resolve_canister_artifact_build_spec(
            &context.with_role("root"),
            config.model(),
            &super::CanisterArtifactBuildOptions::default(),
        )?;
        return Ok(InfrastructureSource {
            manifest_path: spec.package_manifest_path,
            package_name: spec.package_name,
            package_version: spec.canic_version,
            canonical_did_path: None,
            capabilities: spec.capabilities,
        });
    }
    let metadata = cargo_metadata(&context.workspace_root, true)?;
    let canic = resolved_canic_package(&metadata)?;
    let dependencies = resolved_wrapper_dependencies(&metadata, canic)?;
    let manifest = fleet_package::manifest_path(&context.config_path, role.package());
    let config_env = canic_core::role_contract::CANONICAL_BUILD_CONFIG_PATH_ENV;
    let build_script = format!(
        "fn main() {{ let config = std::env::var({config_env:?}).expect(\"Canic build configuration must be set\"); canic::build!(config); }}\n"
    );
    fleet_package::materialize(
        &manifest,
        &context.workspace_root,
        canic.manifest_path.as_std_path(),
        &dependencies,
        &FleetPackageSpec {
            package: role.package(),
            crate_name: role.crate_name(),
            app: role.name(),
            role: role.name(),
            features: if role == InfrastructureRole::Coordinator {
                &["fleet-coordinator-canister"]
            } else {
                &["wasm-store-canister"]
            },
            entrypoint: if role == InfrastructureRole::Coordinator {
                "canic::start_fleet_coordinator!();\ncanic::finish!();\n"
            } else {
                "canic::start_wasm_store!();\ncanic::finish!();\n"
            },
            build_script: (role == InfrastructureRole::Store).then_some(build_script.as_str()),
        },
    )?;
    Ok(InfrastructureSource {
        manifest_path: manifest,
        package_name: role.package().to_string(),
        package_version: canic.version.to_string(),
        canonical_did_path: Some(
            canic
                .manifest_path
                .parent()
                .ok_or("Canic manifest has no parent")?
                .join(format!("candid/{}.did", role.name()))
                .into_std_path_buf(),
        ),
        capabilities: canic_core::role_contract::built_in_role_capabilities(
            if role == InfrastructureRole::Coordinator {
                canic_core::role_contract::BuiltInRoleKind::FleetCoordinator
            } else {
                canic_core::role_contract::BuiltInRoleKind::WasmStore
            },
        ),
    })
}

fn require_contract(
    role: InfrastructureRole,
    manifest_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let validation = match role {
        InfrastructureRole::Root => return Ok(()), // resolve_source already admits the configured Root contract.
        InfrastructureRole::Coordinator => {
            validate_built_in_fleet_coordinator_package(manifest_path, PackageValidationMode::Build)
        }
        InfrastructureRole::Store => {
            validate_built_in_wasm_store_package(manifest_path, PackageValidationMode::Build)
        }
    };
    let evidence = match validation {
        RolePackageValidation::Supported(evidence) => evidence,
        RolePackageValidation::Unsupported(finding) => {
            return Err(format!("{}: {}", finding.code(), finding_detail(&finding)).into());
        }
    };
    match if role == InfrastructureRole::Coordinator {
        resolve_built_in_fleet_coordinator_contract(&evidence)
    } else {
        resolve_built_in_wasm_store_contract(&evidence)
    } {
        canic_core::role_contract::RoleContractResolution::Resolved { .. } => Ok(()),
        canic_core::role_contract::RoleContractResolution::Rejected { errors } => Err(errors
            .iter()
            .map(|finding| format!("{}: {}", finding.code(), finding_detail(finding)))
            .collect::<Vec<_>>()
            .join("; ")
            .into()),
    }
}

fn run_build(
    role: InfrastructureRole,
    context: &WorkspaceBuildContext,
    manifest_path: &Path,
    protocol_profile_digest: Option<canic_core::role_contract::ProtocolProfileDigest>,
    force_candid_export: bool,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let mut command = cargo_command_for_role(role, context, manifest_path, force_candid_export);
    if let Some(digest) = protocol_profile_digest {
        command.environment.insert(
            canic_core::role_contract::PROTOCOL_PROFILE_DIGEST_ENV.into(),
            digest.to_string().into(),
        );
    }
    let started = Instant::now();
    let CargoBuildOutput {
        execution: output,
        messages,
    } = output_canister_cargo_command(
        &command,
        CargoBuildProgress::batch([role.name()], 1, 1, started),
    )?;
    eprintln!(
        "Build phase {} {}: {:.2}s",
        if force_candid_export {
            "declaration"
        } else {
            "runtime Cargo/link"
        },
        role.name(),
        started.elapsed().as_secs_f64()
    );

    if output.status.success() {
        crate::canister_build::retain_cargo_include_exports(context, &messages)?;
        return Ok(ic_build::cargo::execution::select_cdylib_wasm(
            &messages,
            manifest_path,
        )?);
    }
    Err(format!(
        "Cargo failed to build {}: {}",
        role.name(),
        String::from_utf8_lossy(&output.stderr)
    )
    .into())
}

fn cargo_command_for_role(
    role: InfrastructureRole,
    context: &WorkspaceBuildContext,
    manifest_path: &Path,
    force_candid_export: bool,
) -> CargoInvocation {
    let mut command = super::artifact::canister_cargo_build_command(
        context,
        manifest_path,
        context.profile,
        force_candid_export,
    );
    command.keep_going = false;
    if role == InfrastructureRole::Coordinator {
        command.environment.remove(std::ffi::OsStr::new(
            canic_core::role_contract::CANONICAL_BUILD_MARKER_ENV,
        ));
    }
    append_infrastructure_profile_args(&mut command.configurations, context.profile);
    if force_candid_export && role == InfrastructureRole::Coordinator {
        command.recipe = CargoRecipe::RustcLibrary(
            [
                "--cfg",
                "canic_export_candid",
                "--check-cfg=cfg(canic_export_candid)",
            ]
            .map(Into::into)
            .to_vec(),
        );
    }
    command
}

fn resolve_candid(
    role: InfrastructureRole,
    context: &WorkspaceBuildContext,
    source: &InfrastructureSource,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let Some(canonical_did_path) = &source.canonical_did_path else {
        let wasm = run_build(role, context, &source.manifest_path, None, true)?;
        let cache = super::candid_cache::CandidExtractionCache::prepare(context);
        return super::candid_cache::extract_configured_candid(cache.as_ref(), role.name(), &wasm);
    };
    resolve_infrastructure_candid(
        role.name(),
        canonical_did_path,
        context.refresh_canonical_infrastructure_did,
        None,
        &declaration_target_root(&context.workspace_root)
            .join("wasm32-unknown-unknown")
            .join(context.profile.target_dir_name())
            .join(format!("{}.wasm", role.crate_name())),
        || run_build(role, context, &source.manifest_path, None, true).map(|_| ()),
    )
}
