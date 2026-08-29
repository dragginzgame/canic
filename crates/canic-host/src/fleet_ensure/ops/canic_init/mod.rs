//! Module: fleet_ensure::ops::canic_init
//!
//! Responsibility: compile binary infrastructure init arguments from generated current authority.
//! Does not own: estate identity discovery, artifact production, installation, or retry policy.
//! Boundary: exact desired authority, resolved Principals, and finalized release evidence must agree.

#[cfg(test)]
mod tests;

use crate::{
    fleet_ensure::{
        model::{
            DesiredCanisterInit, DesiredCanisterKind, DesiredFleet, DesiredFleetBootstrap,
            DesiredFleetBootstrapRoot,
        },
        ops::protocol::{self, ProtocolEffectError},
    },
    release_build::validate_finalized_release_build_manifest,
    release_set::{
        CanicInfrastructureArtifactEntry, CanicInfrastructureArtifactManifest,
        CanicInfrastructureRole, FleetSubnetRootReleaseSetManifest,
        load_persisted_application_artifact_union,
        load_persisted_canic_infrastructure_artifact_manifest,
        load_persisted_current_release_set_manifest,
    },
};
use candid::{Principal, encode_one};
use canic_control_plane::dto::fleet_coordinator::FleetCoordinatorInitArgs;
use canic_core::{
    dto::fleet_subnet_root::{
        FleetSubnetRootAuthority, FleetSubnetRootInitArgs, FleetSubnetWasmStoreInitArgs,
    },
    ids::{
        FleetBinding, FleetCoordinatorBinding, FleetKey, FleetRegistryAuthority,
        FleetSubnetRootBinding, FleetSubnetRootReleaseSet, FleetSubnetWasmStoreActivationAuthority,
        FleetSubnetWasmStoreAuthority,
    },
    shared_support::fleet_admission_policy::bind_initial_fleet_admission_policy,
};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};
use thiserror::Error as ThisError;

/// Typed failure while compiling one generated infrastructure initializer.
#[derive(Debug, ThisError)]
pub enum CanicInitError {
    #[error("generated Fleet bootstrap authority is missing")]
    MissingBootstrap,

    #[error("generated Fleet bootstrap references unknown logical canister {0}")]
    UnknownCanister(String),

    #[error("generated Fleet bootstrap has invalid Principal for {field}: {value}")]
    InvalidPrincipal { field: &'static str, value: String },

    #[error("generated Fleet bootstrap repeats Root role {0}")]
    DuplicateRoot(String),

    #[error("generated Fleet bootstrap has no authority for Root role {0}")]
    MissingRoot(String),

    #[error(
        "generated Fleet bootstrap {role} artifact path differs: expected {expected}, observed {actual}"
    )]
    ArtifactPath {
        actual: String,
        expected: String,
        role: &'static str,
    },

    #[error(
        "generated Fleet bootstrap {role} artifact changed: expected {expected}, observed {actual}"
    )]
    ArtifactChanged {
        actual: String,
        expected: String,
        role: &'static str,
    },

    #[error("generated Fleet bootstrap authority is invalid: {0}")]
    Authority(String),

    #[error("generated Fleet bootstrap release evidence is invalid: {0}")]
    Release(String),

    #[error("generated Fleet bootstrap init Candid encoding failed: {0}")]
    Candid(#[from] candid::Error),

    #[error(transparent)]
    Argument(#[from] ProtocolEffectError),
}

pub(super) struct CanicInitRequest<'a> {
    pub desired: &'a DesiredFleet,
    pub init: &'a DesiredCanisterInit,
    pub operation_id: &'a str,
    pub principals: &'a BTreeMap<String, String>,
    pub root: &'a Path,
    pub wasm: &'a str,
    pub wasm_sha256: &'a str,
}

pub(super) fn write_arguments(request: CanicInitRequest<'_>) -> Result<PathBuf, CanicInitError> {
    protocol::write_argument_file(&compile_arguments(&request)?).map_err(Into::into)
}

/// Compile the exact Root authorities installed by the generated current Fleet bootstrap.
pub(super) fn compile_root_authorities(
    root: &Path,
    desired: &DesiredFleet,
    principals: &BTreeMap<String, String>,
) -> Result<Vec<(String, FleetSubnetRootAuthority)>, CanicInitError> {
    let bootstrap = desired
        .bootstrap
        .as_ref()
        .ok_or(CanicInitError::MissingBootstrap)?;
    let coordinator = resolve_principal(principals, "Fleet Coordinator", &bootstrap.coordinator)?;
    let operator = parse_principal("operator", &desired.operator)?;
    let authority = registry_authority(bootstrap, coordinator);
    let infrastructure =
        load_persisted_canic_infrastructure_artifact_manifest(root, bootstrap.release_build_id)
            .map_err(|error| CanicInitError::Release(error.to_string()))?;
    let complete = load_persisted_current_release_set_manifest(root, bootstrap.release_build_id)
        .map_err(|error| CanicInitError::Release(error.to_string()))?;
    validate_finalized_release_build_manifest(root, bootstrap.release_build_id, &complete.path)
        .map_err(|error| CanicInitError::Release(error.to_string()))?;
    if complete.manifest.infrastructure_artifact_manifest_sha256 != infrastructure.digest {
        return Err(CanicInitError::Release(
            "complete release authority does not bind the infrastructure manifest".to_string(),
        ));
    }
    let root_artifact = infrastructure_entry(
        &infrastructure.manifest,
        CanicInfrastructureRole::FleetSubnetRoot,
    )?;
    let store_artifact =
        infrastructure_entry(&infrastructure.manifest, CanicInfrastructureRole::WasmStore)?;
    bootstrap
        .roots
        .iter()
        .map(|input| {
            compile_root_authority(
                root,
                desired,
                principals,
                &authority,
                operator,
                &input.root,
                root_artifact.wasm_sha256_hex.as_str(),
                store_artifact.wasm_sha256_hex.as_str(),
            )
            .map(|authority| (input.root.clone(), authority))
        })
        .collect()
}

#[expect(
    clippy::too_many_lines,
    reason = "one typed initializer compiler keeps all infrastructure role branches together"
)]
fn compile_arguments(request: &CanicInitRequest<'_>) -> Result<Vec<u8>, CanicInitError> {
    let bootstrap = request
        .desired
        .bootstrap
        .as_ref()
        .ok_or(CanicInitError::MissingBootstrap)?;
    let coordinator = resolve_principal(
        request.principals,
        "Fleet Coordinator",
        &bootstrap.coordinator,
    )?;
    let operator = parse_principal("operator", &request.desired.operator)?;
    let binding = FleetBinding {
        fleet: FleetKey {
            canonical_network_id: bootstrap.canonical_network_id,
            fleet_id: bootstrap.fleet_id,
        },
        app: bootstrap.app.clone(),
    };
    let authority = registry_authority(bootstrap, coordinator);
    let admission = bind_initial_fleet_admission_policy(binding, &bootstrap.admission)
        .map_err(|error| CanicInitError::Authority(error.to_string()))?;
    let infrastructure = load_persisted_canic_infrastructure_artifact_manifest(
        request.root,
        bootstrap.release_build_id,
    )
    .map_err(|error| CanicInitError::Release(error.to_string()))?;
    let complete =
        load_persisted_current_release_set_manifest(request.root, bootstrap.release_build_id)
            .map_err(|error| CanicInitError::Release(error.to_string()))?;
    validate_finalized_release_build_manifest(
        request.root,
        bootstrap.release_build_id,
        &complete.path,
    )
    .map_err(|error| CanicInitError::Release(error.to_string()))?;
    if complete.manifest.infrastructure_artifact_manifest_sha256 != infrastructure.digest {
        return Err(CanicInitError::Release(
            "complete release authority does not bind the infrastructure manifest".to_string(),
        ));
    }
    let root_artifact = infrastructure_entry(
        &infrastructure.manifest,
        CanicInfrastructureRole::FleetSubnetRoot,
    )?;
    let store_artifact =
        infrastructure_entry(&infrastructure.manifest, CanicInfrastructureRole::WasmStore)?;

    match request.init {
        DesiredCanisterInit::Coordinator => {
            verify_target_artifact(
                request,
                CanicInfrastructureRole::FleetCoordinator,
                &infrastructure.manifest,
            )?;
            encode_coordinator_arguments(bootstrap, authority, admission)
        }
        DesiredCanisterInit::Root { root } => {
            verify_target_artifact(
                request,
                CanicInfrastructureRole::FleetSubnetRoot,
                &infrastructure.manifest,
            )?;
            let root_authority = compile_root_authority(
                request.root,
                request.desired,
                request.principals,
                &authority,
                operator,
                root,
                root_artifact.wasm_sha256_hex.as_str(),
                store_artifact.wasm_sha256_hex.as_str(),
            )?;
            let root_input = root_input(bootstrap, root)?;
            let imports = root_input
                .canister_pool_imports
                .iter()
                .map(|name| resolve_principal(request.principals, "pool import", name))
                .collect::<Result<Vec<_>, _>>()?;
            let store_controllers = resolved_canister_controllers(
                request.desired,
                request.principals,
                &root_input.store,
                DesiredCanisterKind::Store,
            )?;
            encode_root_arguments(
                root_authority,
                request.operation_id,
                root,
                store_controllers,
                imports,
            )
        }
        DesiredCanisterInit::Store { root } => {
            verify_target_artifact(
                request,
                CanicInfrastructureRole::WasmStore,
                &infrastructure.manifest,
            )?;
            let root_authority = compile_root_authority(
                request.root,
                request.desired,
                request.principals,
                &authority,
                operator,
                root,
                root_artifact.wasm_sha256_hex.as_str(),
                store_artifact.wasm_sha256_hex.as_str(),
            )?;
            encode_store_arguments(
                root_authority.wasm_store_authority,
                request.operation_id,
                root,
            )
        }
    }
}

fn encode_coordinator_arguments(
    bootstrap: &DesiredFleetBootstrap,
    authority: FleetRegistryAuthority,
    admission: canic_core::ids::FleetAdmissionPolicy,
) -> Result<Vec<u8>, CanicInitError> {
    encode_one(FleetCoordinatorInitArgs {
        configured_app: bootstrap.app.clone(),
        authority,
        admission,
        component_deployment_configuration: bootstrap.component_deployment_configuration.clone(),
        root_funding: bootstrap.root_funding.clone(),
    })
    .map_err(Into::into)
}

fn encode_root_arguments(
    authority: FleetSubnetRootAuthority,
    operation_id: &str,
    root: &str,
    wasm_store_controllers: Vec<Principal>,
    canister_pool_imports: Vec<Principal>,
) -> Result<Vec<u8>, CanicInitError> {
    let wasm_store_activation = FleetSubnetWasmStoreActivationAuthority {
        fleet: authority.binding.authority.binding.fleet.clone(),
        operation_id: install_id(operation_id, "store", root),
        fleet_subnet_root: authority.binding.fleet_subnet_root,
        wasm_store: authority.wasm_store_authority.wasm_store,
        release_build_id: authority.initial_release_set.release_build_id,
        component_topology_digest: authority.binding.component_topology_digest,
        controllers: wasm_store_controllers,
        manifest_digest: authority.initial_release_set.manifest_digest,
    };
    encode_one(FleetSubnetRootInitArgs {
        authority,
        install_id: install_id(operation_id, "root", root),
        wasm_store_activation,
        canister_pool_imports,
    })
    .map_err(Into::into)
}

fn encode_store_arguments(
    authority: FleetSubnetWasmStoreAuthority,
    operation_id: &str,
    root: &str,
) -> Result<Vec<u8>, CanicInitError> {
    encode_one(FleetSubnetWasmStoreInitArgs {
        authority,
        install_id: install_id(operation_id, "store", root),
    })
    .map_err(Into::into)
}

#[expect(
    clippy::too_many_arguments,
    reason = "authority compilation receives each independently verified binding explicitly"
)]
fn compile_root_authority(
    workspace_root: &Path,
    desired: &DesiredFleet,
    principals: &BTreeMap<String, String>,
    authority: &FleetRegistryAuthority,
    operator: Principal,
    root_name: &str,
    root_wasm_sha256: &str,
    store_wasm_sha256: &str,
) -> Result<FleetSubnetRootAuthority, CanicInitError> {
    let bootstrap = desired.bootstrap.as_ref().expect("checked bootstrap");
    let root_input = root_input(bootstrap, root_name)?;
    let root_principal = resolve_principal(principals, "Fleet Subnet Root", &root_input.root)?;
    let store = resolve_principal(principals, "Wasm Store", &root_input.store)?;
    let binding = FleetSubnetRootBinding {
        authority: authority.clone(),
        placement_subnet: root_input.placement_subnet,
        fleet_subnet_root: root_principal,
        component_admissions: root_input.component_admissions.clone(),
        component_topology_digest: root_input.component_topology_digest,
        limits: root_input.limits.clone(),
        funding: root_input.funding.clone(),
    };
    bootstrap
        .component_deployment_configuration
        .component_topology
        .validate_root_binding(&binding)
        .map_err(|error| CanicInitError::Authority(error.to_string()))?;
    let union = load_persisted_application_artifact_union(
        workspace_root,
        &bootstrap
            .component_deployment_configuration
            .component_topology,
        bootstrap.release_build_id,
    )
    .map_err(|error| CanicInitError::Release(error.to_string()))?;
    let complete =
        load_persisted_current_release_set_manifest(workspace_root, bootstrap.release_build_id)
            .map_err(|error| CanicInitError::Release(error.to_string()))?;
    if complete.manifest.application_artifact_union_sha256 != union.digest {
        return Err(CanicInitError::Release(
            "complete release authority does not bind the application artifact union".to_string(),
        ));
    }
    let manifest = FleetSubnetRootReleaseSetManifest::project(
        &bootstrap
            .component_deployment_configuration
            .component_topology,
        &binding,
        &union.union,
    )
    .map_err(|error| CanicInitError::Release(error.to_string()))?;
    let manifest_digest = manifest
        .digest(
            &bootstrap
                .component_deployment_configuration
                .component_topology,
            &binding,
            &union.union,
        )
        .map_err(|error| CanicInitError::Release(error.to_string()))?;
    Ok(FleetSubnetRootAuthority {
        binding,
        initial_release_set: FleetSubnetRootReleaseSet {
            release_build_id: bootstrap.release_build_id,
            manifest_digest,
        },
        expected_module_hash: decode_sha256(root_wasm_sha256, "Fleet Subnet Root")?,
        wasm_store_authority: FleetSubnetWasmStoreAuthority {
            authority: authority.clone(),
            placement_subnet: root_input.placement_subnet,
            fleet_subnet_root: root_principal,
            wasm_store: store,
            installation_controller: operator,
            release_build_id: bootstrap.release_build_id,
            wasm_module_hash: decode_sha256(store_wasm_sha256, "Wasm Store")?,
        },
    })
}

fn registry_authority(
    bootstrap: &DesiredFleetBootstrap,
    coordinator: Principal,
) -> FleetRegistryAuthority {
    FleetRegistryAuthority {
        binding: FleetCoordinatorBinding {
            fleet: FleetBinding {
                fleet: FleetKey {
                    canonical_network_id: bootstrap.canonical_network_id,
                    fleet_id: bootstrap.fleet_id,
                },
                app: bootstrap.app.clone(),
            },
            coordinator_subnet: bootstrap.coordinator_subnet,
            coordinator,
        },
        epoch: 1,
    }
}

fn root_input<'a>(
    bootstrap: &'a DesiredFleetBootstrap,
    root: &str,
) -> Result<&'a DesiredFleetBootstrapRoot, CanicInitError> {
    let mut matching = bootstrap.roots.iter().filter(|entry| entry.root == root);
    let result = matching
        .next()
        .ok_or_else(|| CanicInitError::MissingRoot(root.to_string()))?;
    if matching.next().is_some() {
        return Err(CanicInitError::DuplicateRoot(root.to_string()));
    }
    Ok(result)
}

fn infrastructure_entry(
    manifest: &CanicInfrastructureArtifactManifest,
    role: CanicInfrastructureRole,
) -> Result<&CanicInfrastructureArtifactEntry, CanicInitError> {
    manifest
        .entries
        .iter()
        .find(|entry| entry.role == role)
        .ok_or_else(|| CanicInitError::Release(format!("missing {role:?} artifact")))
}

fn verify_target_artifact(
    request: &CanicInitRequest<'_>,
    role: CanicInfrastructureRole,
    manifest: &CanicInfrastructureArtifactManifest,
) -> Result<(), CanicInitError> {
    let entry = infrastructure_entry(manifest, role)?;
    if entry.wasm_relative_path != request.wasm {
        return Err(CanicInitError::ArtifactPath {
            actual: request.wasm.to_string(),
            expected: entry.wasm_relative_path.clone(),
            role: role.as_str(),
        });
    }
    if entry.wasm_sha256_hex != request.wasm_sha256 {
        return Err(CanicInitError::ArtifactChanged {
            actual: request.wasm_sha256.to_string(),
            expected: entry.wasm_sha256_hex.clone(),
            role: role.as_str(),
        });
    }
    Ok(())
}

fn resolve_principal(
    principals: &BTreeMap<String, String>,
    field: &'static str,
    name: &str,
) -> Result<Principal, CanicInitError> {
    let value = principals
        .get(name)
        .ok_or_else(|| CanicInitError::UnknownCanister(name.to_string()))?;
    parse_principal(field, value)
}

fn resolved_canister_controllers(
    desired: &DesiredFleet,
    principals: &BTreeMap<String, String>,
    name: &str,
    kind: DesiredCanisterKind,
) -> Result<Vec<Principal>, CanicInitError> {
    let configured = desired
        .canisters
        .iter()
        .find(|configured| configured.name == name && configured.kind == kind)
        .ok_or_else(|| CanicInitError::UnknownCanister(name.to_string()))?;
    let mut controllers = configured
        .controllers
        .iter()
        .map(|controller| parse_principal("Canister controller", controller))
        .chain(
            configured
                .controller_canisters
                .iter()
                .map(|controller| resolve_principal(principals, "Canister controller", controller)),
        )
        .collect::<Result<Vec<_>, _>>()?;
    controllers.sort();
    controllers.dedup();
    Ok(controllers)
}

fn parse_principal(field: &'static str, value: &str) -> Result<Principal, CanicInitError> {
    Principal::from_text(value).map_err(|_| CanicInitError::InvalidPrincipal {
        field,
        value: value.to_string(),
    })
}

fn decode_sha256(value: &str, role: &str) -> Result<[u8; 32], CanicInitError> {
    let bytes = canic_core::cdk::utils::hash::decode_hex(value)
        .map_err(|error| CanicInitError::Release(error.to_string()))?;
    bytes.try_into().map_err(|_| {
        CanicInitError::Release(format!("{role} artifact digest is not exactly 32 bytes"))
    })
}

fn install_id(operation_id: &str, role: &str, name: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"canic:fleet-ensure:infrastructure-install:v1\0");
    hasher.update(operation_id.as_bytes());
    hasher.update([0]);
    hasher.update(role.as_bytes());
    hasher.update([0]);
    hasher.update(name.as_bytes());
    hasher.finalize().into()
}
