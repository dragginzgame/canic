//! Module: state_manifest::resolution
//!
//! Responsibility: resolve declared and built-in role packages into one host
//! state manifest.
//! Does not own: descriptor definitions, audit checks, report aggregation, or
//! rendering.
//! Boundary: reads passive project/package metadata and returns a complete
//! resolved manifest or blocking role-contract findings.

use crate::role_contract::{
    PackageValidationMode, RoleCargoGraphEvidence, RolePackageValidation,
    materialize_state_manifest, resolve_built_in_wasm_store_contract,
    resolve_declared_role_package_contract, validate_built_in_wasm_store_package,
    validate_declared_role_package,
};
use canic_core::{
    bootstrap::parse_config_model,
    ids::CanisterRole,
    role_contract::{ResolvedRoleContract, RoleContractFinding, RoleContractResolution},
    state_contract::StateManifest,
};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StateManifestResolution {
    Rejected {
        errors: Vec<RoleContractFinding>,
    },
    Resolved {
        manifest: StateManifest,
        contracts: Vec<ResolvedRoleContract>,
    },
}

#[must_use]
pub fn resolve_project_state_manifest(
    project_root: &Path,
    config_paths: &[PathBuf],
    role_filter: Option<&str>,
) -> StateManifestResolution {
    let mut contracts = BTreeMap::<String, ResolvedRoleContract>::new();
    let mut evidence = Vec::<RoleCargoGraphEvidence>::new();
    let mut errors = Vec::new();
    let mut matched_declared_role = false;

    if role_filter != Some(CanisterRole::WASM_STORE.as_str()) {
        for config_path in config_paths {
            let source = match fs::read_to_string(config_path) {
                Ok(source) => source,
                Err(error) => {
                    errors.push(RoleContractFinding::DependencyShapeUnsupported {
                        reason: format!("failed to read {}: {error}", config_path.display()),
                    });
                    continue;
                }
            };
            let config = match parse_config_model(&source) {
                Ok(config) => config,
                Err(error) => {
                    errors.push(RoleContractFinding::DependencyShapeUnsupported {
                        reason: format!("invalid {}: {error}", config_path.display()),
                    });
                    continue;
                }
            };

            for role in config.roles.keys() {
                if role_filter.is_some_and(|filter| filter != role.as_str()) {
                    continue;
                }
                matched_declared_role = true;
                match validate_declared_role_package(
                    config_path,
                    role,
                    PackageValidationMode::Passive,
                ) {
                    RolePackageValidation::Supported(package_evidence) => {
                        let resolution =
                            resolve_declared_role_package_contract(config_path, &package_evidence);
                        evidence.push(package_evidence);
                        collect_contract(role, resolution, &mut contracts, &mut errors);
                    }
                    RolePackageValidation::Unsupported(finding) => errors.push(finding),
                }
            }
        }
    }

    if let Some(role) = role_filter
        && role != CanisterRole::WASM_STORE.as_str()
        && !matched_declared_role
    {
        errors.push(RoleContractFinding::RoleUnknown {
            role: CanisterRole::owned(role.to_string()),
        });
    }

    if role_filter.is_none() || role_filter == Some(CanisterRole::WASM_STORE.as_str()) {
        match existing_built_in_wasm_store_manifest(project_root, &evidence) {
            Some(manifest_path) => match validate_built_in_wasm_store_package(
                &manifest_path,
                PackageValidationMode::Passive,
            ) {
                RolePackageValidation::Supported(package_evidence) => collect_contract(
                    &CanisterRole::WASM_STORE,
                    resolve_built_in_wasm_store_contract(&package_evidence),
                    &mut contracts,
                    &mut errors,
                ),
                RolePackageValidation::Unsupported(finding) => errors.push(finding),
            },
            None => errors.push(RoleContractFinding::BuiltInPackageUnavailable {
                role: canic_core::role_contract::BuiltInRoleKind::WasmStore,
            }),
        }
    }

    if !errors.is_empty() {
        return StateManifestResolution::Rejected { errors };
    }

    let contracts = contracts.into_values().collect::<Vec<_>>();
    match materialize_state_manifest(&contracts) {
        Ok(manifest) => StateManifestResolution::Resolved {
            manifest,
            contracts,
        },
        Err(errors) => StateManifestResolution::Rejected { errors },
    }
}

fn collect_contract(
    role: &CanisterRole,
    resolution: RoleContractResolution,
    contracts: &mut BTreeMap<String, ResolvedRoleContract>,
    errors: &mut Vec<RoleContractFinding>,
) {
    match resolution {
        RoleContractResolution::Rejected {
            errors: resolution_errors,
        } => errors.extend(resolution_errors),
        RoleContractResolution::Resolved { contract } => {
            let role_name = role.as_str().to_string();
            if contracts
                .get(&role_name)
                .is_some_and(|existing| existing != &contract)
            {
                errors.push(RoleContractFinding::PackageAmbiguous { role: role.clone() });
            } else {
                contracts.insert(role_name, contract);
            }
        }
    }
}

fn existing_built_in_wasm_store_manifest(
    project_root: &Path,
    evidence: &[RoleCargoGraphEvidence],
) -> Option<PathBuf> {
    for candidate in [
        project_root.join("crates/canic-wasm-store/Cargo.toml"),
        project_root.join(".icp/local/generated/canic-wasm-store/Cargo.toml"),
    ] {
        if candidate.is_file() {
            return Some(candidate);
        }
    }

    for package in evidence {
        let canic_root = package.canic_manifest_path.parent()?;
        let sibling_root = canic_root.parent()?;
        for candidate in [
            sibling_root.join("canic-wasm-store/Cargo.toml"),
            sibling_root
                .join(format!("canic-wasm-store-{}", package.canic_version))
                .join("Cargo.toml"),
        ] {
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}
