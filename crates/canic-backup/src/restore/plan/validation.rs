//! Module: restore::plan::validation
//!
//! Responsibility: validate persisted restore plans and their derived projections.
//! Does not own: plan construction, artifact validation, or restore execution.
//! Boundary: persisted plan readers and apply preparation fail closed through typed causes.

use super::{
    RestorePlan, RestorePlanError, RestorePlanMember, order_members, restore_identity_summary,
    restore_operation_summary, restore_ordering_summary, restore_readiness_summary,
    restore_snapshot_summary, restore_verification_summary,
};
use crate::manifest::IdentityMode;
use candid::Principal;
use std::{
    collections::{BTreeMap, BTreeSet},
    str::FromStr,
};

const SUPPORTED_RESTORE_PLAN_VERSION: u16 = 1;
const SHA256_ALGORITHM: &str = "sha256";

impl RestorePlan {
    /// Validate one persisted restore plan and every derived projection.
    pub fn validate(&self) -> Result<(), RestorePlanError> {
        if self.plan_version != SUPPORTED_RESTORE_PLAN_VERSION {
            return Err(RestorePlanError::UnsupportedVersion(self.plan_version));
        }
        validate_nonempty("backup_id", &self.backup_id)?;
        validate_nonempty("source_environment", &self.source_environment)?;
        validate_principal("source_root_canister", &self.source_root_canister)?;
        validate_hash("topology_hash", &self.topology_hash)?;
        if self.member_count != self.members.len() {
            return Err(RestorePlanError::MemberCountMismatch {
                expected: self.members.len(),
                actual: self.member_count,
            });
        }
        if self.members.is_empty() {
            return Err(RestorePlanError::EmptyField("members"));
        }

        validate_members(self)?;
        validate_projections(self)
    }
}

fn validate_members(plan: &RestorePlan) -> Result<(), RestorePlanError> {
    let mut sources = BTreeSet::new();
    let mut targets = BTreeSet::new();

    for member in &plan.members {
        validate_principal("members[].source_canister", &member.source_canister)?;
        validate_principal("members[].target_canister", &member.target_canister)?;
        validate_nonempty("members[].role", &member.role)?;
        if !sources.insert(member.source_canister.clone()) {
            return Err(RestorePlanError::DuplicatePlanSource(
                member.source_canister.clone(),
            ));
        }
        if !targets.insert(member.target_canister.clone()) {
            return Err(RestorePlanError::DuplicatePlanTarget(
                member.target_canister.clone(),
            ));
        }
        if member.identity_mode == IdentityMode::Fixed
            && member.source_canister != member.target_canister
        {
            return Err(RestorePlanError::FixedIdentityRemap {
                source_canister: member.source_canister.clone(),
                target_canister: member.target_canister.clone(),
            });
        }
        validate_member_snapshot(member)?;
    }

    let source_targets = plan
        .members
        .iter()
        .map(|member| {
            (
                member.source_canister.as_str(),
                member.target_canister.as_str(),
            )
        })
        .collect::<BTreeMap<_, _>>();

    for member in &plan.members {
        if let Some(parent_source) = &member.parent_source_canister {
            validate_principal("members[].parent_source_canister", parent_source)?;
            match source_targets.get(parent_source.as_str()) {
                Some(expected_target)
                    if member.parent_target_canister.as_deref() == Some(*expected_target) => {}
                None if member.parent_target_canister.is_none() => {}
                _ => {
                    return Err(RestorePlanError::ProjectionMismatch(
                        "members[].parent_target_canister",
                    ));
                }
            }
        } else if member.parent_target_canister.is_some() {
            return Err(RestorePlanError::ProjectionMismatch(
                "members[].parent_target_canister",
            ));
        }
        if let Some(parent_target) = &member.parent_target_canister {
            validate_principal("members[].parent_target_canister", parent_target)?;
        }
    }

    for check in &plan.deployment_verification_checks {
        validate_nonempty("deployment_verification_checks[].kind", &check.kind)?;
    }

    let canonical = order_members(plan.members.clone())?;
    if canonical != plan.members {
        return Err(RestorePlanError::ProjectionMismatch("members"));
    }
    Ok(())
}

fn validate_member_snapshot(member: &RestorePlanMember) -> Result<(), RestorePlanError> {
    validate_nonempty(
        "members[].source_snapshot.snapshot_id",
        &member.source_snapshot.snapshot_id,
    )?;
    validate_nonempty(
        "members[].source_snapshot.artifact_path",
        &member.source_snapshot.artifact_path,
    )?;
    if member.source_snapshot.checksum_algorithm != SHA256_ALGORITHM {
        return Err(RestorePlanError::ProjectionMismatch(
            "members[].source_snapshot.checksum_algorithm",
        ));
    }
    if let Some(checksum) = &member.source_snapshot.checksum {
        validate_hash("members[].source_snapshot.checksum", checksum)?;
    }
    for check in &member.verification_checks {
        validate_nonempty("members[].verification_checks[].kind", &check.kind)?;
    }
    Ok(())
}

fn validate_projections(plan: &RestorePlan) -> Result<(), RestorePlanError> {
    let identity = restore_identity_summary(&plan.members, plan.identity_summary.mapping_supplied);
    require_projection("identity_summary", &plan.identity_summary, &identity)?;
    let snapshot = restore_snapshot_summary(&plan.members);
    require_projection("snapshot_summary", &plan.snapshot_summary, &snapshot)?;
    let verification =
        restore_verification_summary(&plan.deployment_verification_checks, &plan.members);
    require_projection(
        "verification_summary",
        &plan.verification_summary,
        &verification,
    )?;
    let readiness = restore_readiness_summary(&snapshot, &verification);
    require_projection("readiness_summary", &plan.readiness_summary, &readiness)?;
    let operations = restore_operation_summary(plan.members.len(), &verification);
    require_projection("operation_summary", &plan.operation_summary, &operations)?;
    let ordering = restore_ordering_summary(&plan.members);
    require_projection("ordering_summary", &plan.ordering_summary, &ordering)
}

fn require_projection<T: PartialEq>(
    field: &'static str,
    actual: &T,
    expected: &T,
) -> Result<(), RestorePlanError> {
    if actual == expected {
        Ok(())
    } else {
        Err(RestorePlanError::ProjectionMismatch(field))
    }
}

fn validate_nonempty(field: &'static str, value: &str) -> Result<(), RestorePlanError> {
    if value.trim().is_empty() {
        Err(RestorePlanError::EmptyField(field))
    } else {
        Ok(())
    }
}

fn validate_principal(field: &'static str, value: &str) -> Result<(), RestorePlanError> {
    validate_nonempty(field, value)?;
    Principal::from_str(value)
        .map(|_| ())
        .map_err(|_| RestorePlanError::InvalidPrincipal {
            field,
            value: value.to_string(),
        })
}

fn validate_hash(field: &'static str, value: &str) -> Result<(), RestorePlanError> {
    if value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err(RestorePlanError::InvalidHash {
            field,
            value: value.to_string(),
        })
    }
}
