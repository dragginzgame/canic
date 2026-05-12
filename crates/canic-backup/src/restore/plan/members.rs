use super::{RestoreMapping, RestorePlanError, RestorePlanMember};
use crate::manifest::{
    FleetBackupManifest, FleetMember, IdentityMode, VerificationCheck, VerificationPlan,
};
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn resolve_members(
    manifest: &FleetBackupManifest,
    mapping: Option<&RestoreMapping>,
) -> Result<Vec<RestorePlanMember>, RestorePlanError> {
    let mut plan_members = Vec::with_capacity(manifest.fleet.members.len());
    let mut targets = BTreeSet::new();
    let mut source_to_target = BTreeMap::new();

    for member in &manifest.fleet.members {
        let target = resolve_target(member, mapping)?;
        if !targets.insert(target.clone()) {
            return Err(RestorePlanError::DuplicatePlanTarget(target));
        }

        source_to_target.insert(member.canister_id.clone(), target.clone());
        plan_members.push(RestorePlanMember {
            source_canister: member.canister_id.clone(),
            target_canister: target,
            role: member.role.clone(),
            parent_source_canister: member.parent_canister_id.clone(),
            parent_target_canister: None,
            ordering_dependency: None,
            member_order: 0,
            identity_mode: member.identity_mode.clone(),
            verification_checks: concrete_member_verification_checks(
                member,
                &manifest.verification,
            ),
            source_snapshot: member.source_snapshot.clone(),
        });
    }

    for member in &mut plan_members {
        member.parent_target_canister = member
            .parent_source_canister
            .as_ref()
            .and_then(|parent| source_to_target.get(parent))
            .cloned();
    }

    Ok(plan_members)
}

fn concrete_member_verification_checks(
    member: &FleetMember,
    verification: &VerificationPlan,
) -> Vec<VerificationCheck> {
    let mut checks = member
        .verification_checks
        .iter()
        .filter(|check| verification_check_applies_to_role(check, &member.role))
        .cloned()
        .collect::<Vec<_>>();

    for group in &verification.member_checks {
        if group.role != member.role {
            continue;
        }

        checks.extend(
            group
                .checks
                .iter()
                .filter(|check| verification_check_applies_to_role(check, &member.role))
                .cloned(),
        );
    }

    checks
}

fn verification_check_applies_to_role(check: &VerificationCheck, role: &str) -> bool {
    check.roles.is_empty() || check.roles.iter().any(|check_role| check_role == role)
}

fn resolve_target(
    member: &FleetMember,
    mapping: Option<&RestoreMapping>,
) -> Result<String, RestorePlanError> {
    let target = match mapping {
        Some(mapping) => mapping
            .target_for(&member.canister_id)
            .ok_or_else(|| RestorePlanError::MissingMappingSource(member.canister_id.clone()))?
            .to_string(),
        None => member.canister_id.clone(),
    };

    if matches!(member.identity_mode, IdentityMode::Fixed) && target != member.canister_id {
        return Err(RestorePlanError::FixedIdentityRemap {
            source_canister: member.canister_id.clone(),
            target_canister: target,
        });
    }

    Ok(target)
}
