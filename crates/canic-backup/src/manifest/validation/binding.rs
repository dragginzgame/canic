//! Module: manifest::validation::binding
//!
//! Responsibility: validate cross-section bindings in backup manifests.
//! Does not own: manifest data definitions, scalar validation, or persistence.
//! Boundary: compares consistency and verification sections against deployment state.

use crate::manifest::{
    BackupUnit, BackupUnitKind, ConsistencySection, DeploymentMember, DeploymentSection,
    ManifestValidationError, VerificationCheck, VerificationPlan,
};

use std::collections::{BTreeMap, BTreeSet};

pub(super) fn validate_consistency_against_deployment(
    consistency: &ConsistencySection,
    deployment: &DeploymentSection,
) -> Result<(), ManifestValidationError> {
    let deployment_roles = deployment
        .members
        .iter()
        .map(|member| member.role.as_str())
        .collect::<BTreeSet<_>>();
    let mut covered_roles = BTreeSet::new();

    for unit in &consistency.backup_units {
        for role in &unit.roles {
            if !deployment_roles.contains(role.as_str()) {
                return Err(ManifestValidationError::UnknownBackupUnitRole {
                    unit_id: unit.unit_id.clone(),
                    role: role.clone(),
                });
            }
            covered_roles.insert(role.as_str());
        }

        validate_backup_unit_topology(unit, deployment)?;
    }

    for role in &deployment_roles {
        if !covered_roles.contains(role) {
            return Err(ManifestValidationError::BackupUnitCoverageMissingRole {
                role: (*role).to_string(),
            });
        }
    }

    Ok(())
}

pub(super) fn validate_verification_against_deployment(
    verification: &VerificationPlan,
    deployment: &DeploymentSection,
) -> Result<(), ManifestValidationError> {
    let deployment_roles = deployment
        .members
        .iter()
        .map(|member| member.role.as_str())
        .collect::<BTreeSet<_>>();

    for check in &verification.deployment_checks {
        validate_verification_check_roles(check, &deployment_roles)?;
    }

    for member in &deployment.members {
        for check in &member.verification_checks {
            validate_verification_check_roles(check, &deployment_roles)?;
        }
    }

    let mut member_check_roles = BTreeSet::new();
    for member in &verification.member_checks {
        if !deployment_roles.contains(member.role.as_str()) {
            return Err(ManifestValidationError::UnknownVerificationRole {
                role: member.role.clone(),
            });
        }
        if !member_check_roles.insert(member.role.as_str()) {
            return Err(ManifestValidationError::DuplicateMemberVerificationRole(
                member.role.clone(),
            ));
        }
        for check in &member.checks {
            validate_verification_check_roles(check, &deployment_roles)?;
        }
    }

    Ok(())
}

fn validate_verification_check_roles(
    check: &VerificationCheck,
    deployment_roles: &BTreeSet<&str>,
) -> Result<(), ManifestValidationError> {
    for role in &check.roles {
        if !deployment_roles.contains(role.as_str()) {
            return Err(ManifestValidationError::UnknownVerificationRole { role: role.clone() });
        }
    }

    Ok(())
}

fn validate_backup_unit_topology(
    unit: &BackupUnit,
    deployment: &DeploymentSection,
) -> Result<(), ManifestValidationError> {
    match &unit.kind {
        BackupUnitKind::Subtree => validate_subtree_unit(unit, deployment),
        BackupUnitKind::Single => Ok(()),
    }
}

fn validate_subtree_unit(
    unit: &BackupUnit,
    deployment: &DeploymentSection,
) -> Result<(), ManifestValidationError> {
    let unit_roles = unit
        .roles
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let members_by_id = deployment
        .members
        .iter()
        .map(|member| (member.canister_id.as_str(), member))
        .collect::<BTreeMap<_, _>>();
    let unit_member_ids = deployment
        .members
        .iter()
        .filter(|member| unit_roles.contains(member.role.as_str()))
        .map(|member| member.canister_id.as_str())
        .collect::<BTreeSet<_>>();

    let root_count = deployment
        .members
        .iter()
        .filter(|member| unit_member_ids.contains(member.canister_id.as_str()))
        .filter(|member| {
            member
                .parent_canister_id
                .as_deref()
                .is_none_or(|parent| !unit_member_ids.contains(parent))
        })
        .count();
    if root_count != 1 {
        return Err(ManifestValidationError::SubtreeBackupUnitNotConnected {
            unit_id: unit.unit_id.clone(),
        });
    }

    for member in &deployment.members {
        if unit_member_ids.contains(member.canister_id.as_str()) {
            continue;
        }

        if let Some(parent) = first_unit_ancestor(member, &members_by_id, &unit_member_ids) {
            return Err(
                ManifestValidationError::SubtreeBackupUnitMissingDescendant {
                    unit_id: unit.unit_id.clone(),
                    parent: parent.to_string(),
                    descendant: member.canister_id.clone(),
                },
            );
        }
    }

    Ok(())
}

fn first_unit_ancestor<'a>(
    member: &'a DeploymentMember,
    members_by_id: &BTreeMap<&'a str, &'a DeploymentMember>,
    unit_member_ids: &BTreeSet<&'a str>,
) -> Option<&'a str> {
    let mut visited = BTreeSet::new();
    let mut parent = member.parent_canister_id.as_deref();
    while let Some(parent_id) = parent {
        if unit_member_ids.contains(parent_id) {
            return Some(parent_id);
        }
        if !visited.insert(parent_id) {
            return None;
        }
        parent = members_by_id
            .get(parent_id)
            .and_then(|ancestor| ancestor.parent_canister_id.as_deref());
    }

    None
}
