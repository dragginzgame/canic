//! Module: role_contract
//!
//! Responsibility: collect exact local Cargo package evidence for core role policy.
//! Does not own: feature implications, allocation policy, descriptors, or rendering.
//! Boundary: exposes only supported package evidence or one blocking finding.

mod descriptor;
mod package;

pub use descriptor::{
    StateDescriptorRegistry, materialize_state_manifest, validate_state_descriptor_registry,
};

pub(crate) use package::validate_declared_role_package_from_config;
pub use package::{
    PackageValidationMode, RoleCargoGraphEvidence, RolePackageValidation,
    declared_role_manifest_path, validate_built_in_wasm_store_package,
    validate_declared_role_package, validate_internal_test_wasm_packages,
};

use canic_core::{
    bootstrap::parse_config_model,
    role_contract::{
        BuiltInRoleKind, RoleContractFinding, RoleContractInput, RoleContractResolution,
        RoleContractSource, resolve_role_contract,
    },
};
use std::{fs, path::Path};

#[must_use]
pub fn resolve_declared_role_contract(
    config_path: &Path,
    role: &canic_core::ids::CanisterRole,
    mode: PackageValidationMode,
) -> RoleContractResolution {
    match validate_declared_role_package(config_path, role, mode) {
        RolePackageValidation::Supported(evidence) => {
            resolve_declared_role_package_contract(config_path, &evidence)
        }
        RolePackageValidation::Unsupported(finding) => RoleContractResolution::Rejected {
            errors: vec![finding],
        },
    }
}

#[must_use]
pub fn resolve_declared_role_package_contract(
    config_path: &Path,
    evidence: &RoleCargoGraphEvidence,
) -> RoleContractResolution {
    let Ok(config_source) = fs::read_to_string(config_path) else {
        return RoleContractResolution::Rejected {
            errors: vec![RoleContractFinding::DependencyShapeUnsupported {
                reason: "failed to read role configuration".to_string(),
            }],
        };
    };
    let Ok(config) = parse_config_model(&config_source) else {
        return RoleContractResolution::Rejected {
            errors: vec![RoleContractFinding::DependencyShapeUnsupported {
                reason: "invalid role configuration".to_string(),
            }],
        };
    };

    resolve_declared_role_package_contract_from_config(&config, evidence)
}

#[must_use]
pub(crate) fn resolve_declared_role_package_contract_from_config(
    config: &canic_core::bootstrap::compiled::ConfigModel,
    evidence: &RoleCargoGraphEvidence,
) -> RoleContractResolution {
    resolve_role_contract(RoleContractInput {
        source: RoleContractSource::Declared {
            config,
            role: &evidence.role,
        },
        declared_features: evidence.direct_features.clone(),
        default_features_enabled: evidence.default_features_enabled,
    })
}

#[must_use]
pub fn resolve_built_in_wasm_store_contract(
    evidence: &RoleCargoGraphEvidence,
) -> RoleContractResolution {
    resolve_role_contract(RoleContractInput {
        source: RoleContractSource::BuiltIn(BuiltInRoleKind::WasmStore),
        declared_features: evidence.direct_features.clone(),
        default_features_enabled: evidence.default_features_enabled,
    })
}

#[must_use]
pub fn finding_detail(finding: &RoleContractFinding) -> String {
    match finding {
        RoleContractFinding::AllocationDescriptorDuplicate { key } => {
            format!("allocation {key:?} has more than one state descriptor")
        }
        RoleContractFinding::AllocationDescriptorIdMismatch {
            key,
            expected,
            actual,
        } => format!(
            "allocation {key:?} descriptor IDs {:?} do not match canonical IDs {:?}",
            actual.iter().map(|id| id.get()).collect::<Vec<_>>(),
            expected.iter().map(|id| id.get()).collect::<Vec<_>>()
        ),
        RoleContractFinding::AllocationDescriptorMissing { key } => {
            format!("allocation {key:?} has no state descriptor")
        }
        RoleContractFinding::BuiltInPackageUnavailable { role } => {
            format!("built-in package evidence is unavailable for {role:?}")
        }
        RoleContractFinding::CanicVersionMismatch { expected, actual } => {
            format!("resolved Canic version {actual}, expected {expected}")
        }
        RoleContractFinding::CargoCatalogDrift { reason }
        | RoleContractFinding::CatalogInvalid { reason }
        | RoleContractFinding::DependencyShapeUnsupported { reason } => reason.clone(),
        RoleContractFinding::MemoryIdCollision {
            memory_id,
            first,
            second,
        } => format!(
            "memory ID {} is claimed by {first:?} and {second:?}",
            memory_id.get()
        ),
        RoleContractFinding::MultipleCanicPackages { packages } => format!(
            "the wasm runtime graph reaches multiple Canic packages: {}",
            packages.join(", ")
        ),
        RoleContractFinding::PackageAmbiguous { role } => {
            format!("multiple Cargo packages resolve for role {role}")
        }
        RoleContractFinding::PackageMetadataMismatch {
            expected_fleet,
            expected_role,
            actual_fleet,
            actual_role,
        } => format!(
            "package metadata declares fleet={} role={}, expected fleet={expected_fleet} role={expected_role}",
            actual_fleet.as_deref().unwrap_or("<missing>"),
            actual_role.as_deref().unwrap_or("<missing>")
        ),
        RoleContractFinding::PackageMissing { role } => {
            format!("Cargo package for role {role} is missing")
        }
        RoleContractFinding::RequiredFeatureMissing {
            capability,
            feature,
        } => format!(
            "capability {capability:?} requires Canic feature `{}`",
            feature.cargo_name()
        ),
        RoleContractFinding::RoleUnknown { role } => {
            format!("role {role} is not declared")
        }
        RoleContractFinding::RuntimeCanicDependencyMissing { role } => {
            format!("role {role} has no direct normal runtime dependency on package `canic`")
        }
    }
}
