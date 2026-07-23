//! Module: canic_cli::medic::role_contract
//!
//! Responsibility: diagnose configured role-package and runtime-feature contracts.
//! Does not own: Cargo package resolution, role-contract policy, or report rendering.
//! Boundary: maps fleet declarations and resolved package evidence into Medic checks.

use crate::medic::{
    display_medic_path,
    package::{
        canic_dependency_feature_snippet, canic_package_metadata, role_package_manifest_path,
    },
    report::{MedicCategory, MedicCheck, MedicSource},
};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use canic_core::{
    bootstrap::compiled::ConfigModel,
    ids::CanisterRole,
    role_contract::{
        ResolvedRoleContract, RoleContractFinding, RoleContractResolution, RoleFeatureRequirement,
        required_features_for_role,
    },
};
use canic_host::{
    release_set::{AppConfigSnapshot, ConfiguredRoleLifecycle},
    role_contract::{
        PackageValidationMode, RoleCargoGraphEvidence, RolePackageValidation, finding_detail,
        materialize_state_manifest, resolve_declared_role_package_contract,
        validate_declared_role_package,
    },
};

pub(super) fn project_config_quality_checks(root: &Path, configs: &[PathBuf]) -> Vec<MedicCheck> {
    configs
        .iter()
        .flat_map(|config| app_config_quality_checks(root, config))
        .collect()
}

fn app_config_quality_checks(root: &Path, config: &Path) -> Vec<MedicCheck> {
    let config_display = display_medic_path(root, config);
    let snapshot = match AppConfigSnapshot::load(config) {
        Ok(snapshot) => snapshot,
        Err(err) => {
            return vec![MedicCheck::fail(
                MedicCategory::ProjectConfig,
                "app_config_missing",
                config_display,
                err.to_string(),
                "repair the fleet config before running deployment checks",
                MedicSource::AppConfig,
            )];
        }
    };
    let fleet = snapshot.app_id().to_string();
    let roles = snapshot.role_lifecycle();
    let required_features_by_role = required_canic_features_by_role(snapshot.model(), &roles);
    roles
        .iter()
        .flat_map(|role| {
            let mut checks = vec![check_role_package_metadata(root, config, role, &fleet)];
            let role_id = CanisterRole::owned(role.role.clone());
            match validate_declared_role_package(
                config,
                snapshot.model(),
                &role_id,
                PackageValidationMode::Passive,
            ) {
                RolePackageValidation::Supported(evidence) => {
                    checks.extend(check_role_contract_resolution(
                        root,
                        snapshot.model(),
                        role,
                        &evidence,
                        required_features_by_role
                            .get(&role.role)
                            .map(Vec::as_slice)
                            .unwrap_or_default(),
                    ));
                }
                RolePackageValidation::Unsupported(finding) => {
                    if let Some(check) = check_role_package_contract(role, &finding) {
                        checks.push(check);
                    }
                }
            }
            if !role.attached {
                checks.push(check_declared_role_not_deployable(root, config, role));
            }
            checks
        })
        .collect()
}

fn required_canic_features_by_role(
    config: &ConfigModel,
    roles: &[ConfiguredRoleLifecycle],
) -> BTreeMap<String, Vec<RoleFeatureRequirement>> {
    roles
        .iter()
        .map(|role| {
            let role_id = CanisterRole::owned(role.role.clone());
            (
                role.role.clone(),
                required_features_for_role(config, &role_id).unwrap_or_else(|finding| {
                    panic!("configured role contract rejected: {finding:?}")
                }),
            )
        })
        .filter(|(_, requirements)| !requirements.is_empty())
        .collect()
}

fn check_role_package_metadata(
    root: &Path,
    config: &Path,
    role: &ConfiguredRoleLifecycle,
    fleet: &str,
) -> MedicCheck {
    let manifest = role_package_manifest_path(config, &role.package);
    match canic_package_metadata(&manifest) {
        Ok(metadata) if metadata.fleet == fleet && metadata.role == role.role => MedicCheck::pass(
            MedicCategory::ProjectConfig,
            "role_package_metadata_present",
            role.display.clone(),
            format!(
                "{} declares [package.metadata.canic] fleet={} role={}",
                display_medic_path(root, &manifest),
                metadata.fleet,
                metadata.role
            ),
            "none",
            MedicSource::AppConfig,
        ),
        Ok(metadata) => MedicCheck::fail(
            MedicCategory::ProjectConfig,
            "role_package_metadata_missing",
            role.display.clone(),
            format!(
                "{} declares [package.metadata.canic] fleet={} role={}, expected fleet={} role={}",
                display_medic_path(root, &manifest),
                metadata.fleet,
                metadata.role,
                fleet,
                role.role
            ),
            "update package metadata or repair the fleet role declaration",
            MedicSource::AppConfig,
        ),
        Err(err) => MedicCheck::fail(
            MedicCategory::ProjectConfig,
            "role_package_metadata_missing",
            role.display.clone(),
            err,
            "add matching [package.metadata.canic] fleet and role metadata",
            MedicSource::AppConfig,
        ),
    }
}

fn check_role_contract_resolution(
    root: &Path,
    config: &ConfigModel,
    role: &ConfiguredRoleLifecycle,
    evidence: &RoleCargoGraphEvidence,
    requirements: &[RoleFeatureRequirement],
) -> Vec<MedicCheck> {
    match resolve_declared_role_package_contract(config, evidence) {
        RoleContractResolution::Resolved { contract } => {
            check_resolved_role_contract(root, role, evidence, requirements, &contract)
        }
        RoleContractResolution::Rejected { errors } => errors
            .iter()
            .filter_map(|finding| {
                check_role_resolution_finding(root, role, evidence, requirements, finding)
            })
            .collect(),
    }
}

fn check_resolved_role_contract(
    root: &Path,
    role: &ConfiguredRoleLifecycle,
    evidence: &RoleCargoGraphEvidence,
    requirements: &[RoleFeatureRequirement],
    contract: &ResolvedRoleContract,
) -> Vec<MedicCheck> {
    let mut checks = requirements
        .iter()
        .filter(|requirement| contract.required_features.contains(&requirement.feature))
        .map(|requirement| {
            MedicCheck::pass(
                MedicCategory::ProjectConfig,
                "role_required_canic_feature_present",
                role.display.clone(),
                format!(
                    "{} resolves required canic feature `{}` for {}",
                    display_medic_path(root, &evidence.role_manifest_path),
                    requirement.feature.cargo_name(),
                    requirement.config_key
                ),
                "none",
                MedicSource::AppConfig,
            )
        })
        .collect::<Vec<_>>();
    if let Err(errors) = materialize_state_manifest(std::slice::from_ref(contract)) {
        checks.extend(
            errors
                .iter()
                .filter_map(|finding| check_role_package_contract(role, finding)),
        );
    }
    checks
}

fn check_role_resolution_finding(
    root: &Path,
    role: &ConfiguredRoleLifecycle,
    evidence: &RoleCargoGraphEvidence,
    requirements: &[RoleFeatureRequirement],
    finding: &RoleContractFinding,
) -> Option<MedicCheck> {
    if let RoleContractFinding::RequiredFeatureMissing { feature, .. } = finding {
        let requirement = requirements
            .iter()
            .find(|requirement| requirement.feature == *feature)?;
        return Some(MedicCheck::fail(
            MedicCategory::ProjectConfig,
            finding.code(),
            role.display.clone(),
            format!(
                "{} requires canic feature `{}` because {} is enabled ({})",
                role.display,
                requirement.feature.cargo_name(),
                requirement.config_key,
                requirement.reason
            ),
            missing_canic_feature_next_action(
                root,
                &evidence.role_manifest_path,
                requirement.feature.cargo_name(),
            ),
            MedicSource::AppConfig,
        ));
    }
    check_role_package_contract(role, finding)
}

fn check_role_package_contract(
    role: &ConfiguredRoleLifecycle,
    finding: &RoleContractFinding,
) -> Option<MedicCheck> {
    if matches!(
        finding,
        RoleContractFinding::PackageMissing { .. }
            | RoleContractFinding::PackageMetadataMismatch { .. }
            | RoleContractFinding::PackageAmbiguous { .. }
    ) {
        return None;
    }

    let next = match finding {
        RoleContractFinding::AllocationDescriptorDuplicate { .. }
        | RoleContractFinding::AllocationDescriptorIdMismatch { .. }
        | RoleContractFinding::AllocationDescriptorMissing { .. } => {
            "repair the Canic state descriptor registry and rerun canic medic project"
        }
        _ => {
            "use one direct, unconditional, non-optional normal Canic dependency with no package feature forwarding"
        }
    };
    Some(MedicCheck::fail(
        MedicCategory::ProjectConfig,
        finding.code(),
        role.display.clone(),
        finding_detail(finding),
        next,
        MedicSource::AppConfig,
    ))
}

fn missing_canic_feature_next_action(root: &Path, manifest: &Path, feature: &str) -> String {
    format!(
        "edit runtime [dependencies].canic in {} (not [build-dependencies]) and add `{feature}`; for workspace inheritance, edit inherited [workspace.dependencies].canic features; example:\n{}",
        display_medic_path(root, manifest),
        canic_dependency_feature_snippet([feature])
    )
}

fn check_declared_role_not_deployable(
    root: &Path,
    config: &Path,
    role: &ConfiguredRoleLifecycle,
) -> MedicCheck {
    MedicCheck::warn(
        MedicCategory::ProjectConfig,
        "declared_role_not_deployable",
        role.display.clone(),
        format!(
            "role is declared in {} but is not attached to topology",
            display_medic_path(root, config)
        ),
        format!(
            "run canic fleet role attach {} {} --subnet <subnet>, or remove the declaration",
            role.fleet, role.role
        ),
        MedicSource::AppConfig,
    )
}
