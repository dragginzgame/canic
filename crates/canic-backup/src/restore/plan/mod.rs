use crate::manifest::{
    FleetBackupManifest, FleetMember, IdentityMode, ManifestValidationError, SourceSnapshot,
    VerificationCheck, VerificationPlan,
};
use candid::Principal;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, str::FromStr};
use thiserror::Error as ThisError;

///
/// RestoreMapping
///

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct RestoreMapping {
    pub members: Vec<RestoreMappingEntry>,
}

impl RestoreMapping {
    /// Resolve the target canister for one source member.
    fn target_for(&self, source_canister: &str) -> Option<&str> {
        self.members
            .iter()
            .find(|entry| entry.source_canister == source_canister)
            .map(|entry| entry.target_canister.as_str())
    }
}

///
/// RestoreMappingEntry
///

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RestoreMappingEntry {
    pub source_canister: String,
    pub target_canister: String,
}

///
/// RestorePlan
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RestorePlan {
    pub backup_id: String,
    pub source_environment: String,
    pub source_root_canister: String,
    pub topology_hash: String,
    pub member_count: usize,
    pub identity_summary: RestoreIdentitySummary,
    pub snapshot_summary: RestoreSnapshotSummary,
    pub verification_summary: RestoreVerificationSummary,
    pub readiness_summary: RestoreReadinessSummary,
    pub operation_summary: RestoreOperationSummary,
    pub ordering_summary: RestoreOrderingSummary,
    #[serde(default)]
    pub fleet_verification_checks: Vec<VerificationCheck>,
    pub members: Vec<RestorePlanMember>,
}

impl RestorePlan {
    /// Return all planned members in execution order.
    #[must_use]
    pub fn ordered_members(&self) -> Vec<&RestorePlanMember> {
        self.members.iter().collect()
    }
}

///
/// RestoreIdentitySummary
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RestoreIdentitySummary {
    pub mapping_supplied: bool,
    pub all_sources_mapped: bool,
    pub fixed_members: usize,
    pub relocatable_members: usize,
    pub in_place_members: usize,
    pub mapped_members: usize,
    pub remapped_members: usize,
}

///
/// RestoreSnapshotSummary
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "restore summaries intentionally expose machine-readable readiness flags"
)]
pub struct RestoreSnapshotSummary {
    pub all_members_have_module_hash: bool,
    pub all_members_have_wasm_hash: bool,
    pub all_members_have_code_version: bool,
    pub all_members_have_checksum: bool,
    pub members_with_module_hash: usize,
    pub members_with_wasm_hash: usize,
    pub members_with_code_version: usize,
    pub members_with_checksum: usize,
}

///
/// RestoreVerificationSummary
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RestoreVerificationSummary {
    pub verification_required: bool,
    pub all_members_have_checks: bool,
    pub fleet_checks: usize,
    pub member_check_groups: usize,
    pub member_checks: usize,
    pub members_with_checks: usize,
    pub total_checks: usize,
}

///
/// RestoreReadinessSummary
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RestoreReadinessSummary {
    pub ready: bool,
    pub reasons: Vec<String>,
}

///
/// RestoreOperationSummary
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RestoreOperationSummary {
    pub planned_snapshot_uploads: usize,
    pub planned_snapshot_loads: usize,
    pub planned_verification_checks: usize,
    pub planned_operations: usize,
}

///
/// RestoreOrderingSummary
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RestoreOrderingSummary {
    pub ordered_members: usize,
    pub dependency_free_members: usize,
    pub parent_edges: usize,
}

///
/// RestorePlanMember
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RestorePlanMember {
    pub source_canister: String,
    pub target_canister: String,
    pub role: String,
    pub parent_source_canister: Option<String>,
    pub parent_target_canister: Option<String>,
    pub ordering_dependency: Option<RestoreOrderingDependency>,
    pub member_order: usize,
    pub identity_mode: IdentityMode,
    pub verification_checks: Vec<VerificationCheck>,
    pub source_snapshot: SourceSnapshot,
}

///
/// RestoreOrderingDependency
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RestoreOrderingDependency {
    pub source_canister: String,
    pub target_canister: String,
    pub relationship: RestoreOrderingRelationship,
}

///
/// RestoreOrderingRelationship
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RestoreOrderingRelationship {
    ParentBeforeChild,
}

///
/// RestorePlanner
///

pub struct RestorePlanner;

impl RestorePlanner {
    /// Build a no-mutation restore plan from the manifest and optional target mapping.
    pub fn plan(
        manifest: &FleetBackupManifest,
        mapping: Option<&RestoreMapping>,
    ) -> Result<RestorePlan, RestorePlanError> {
        manifest.validate()?;
        if let Some(mapping) = mapping {
            validate_mapping(mapping)?;
            validate_mapping_sources(manifest, mapping)?;
        }

        let members = resolve_members(manifest, mapping)?;
        let identity_summary = restore_identity_summary(&members, mapping.is_some());
        let snapshot_summary = restore_snapshot_summary(&members);
        let verification_summary = restore_verification_summary(manifest, &members);
        let readiness_summary = restore_readiness_summary(&snapshot_summary, &verification_summary);
        let members = order_members(members)?;
        let ordering_summary = restore_ordering_summary(&members);
        let operation_summary =
            restore_operation_summary(manifest.fleet.members.len(), &verification_summary);

        Ok(RestorePlan {
            backup_id: manifest.backup_id.clone(),
            source_environment: manifest.source.environment.clone(),
            source_root_canister: manifest.source.root_canister.clone(),
            topology_hash: manifest.fleet.topology_hash.clone(),
            member_count: manifest.fleet.members.len(),
            identity_summary,
            snapshot_summary,
            verification_summary,
            readiness_summary,
            operation_summary,
            ordering_summary,
            fleet_verification_checks: manifest.verification.fleet_checks.clone(),
            members,
        })
    }
}

///
/// RestorePlanError
///

#[derive(Debug, ThisError)]
pub enum RestorePlanError {
    #[error(transparent)]
    InvalidManifest(#[from] ManifestValidationError),

    #[error("field {field} must be a valid principal: {value}")]
    InvalidPrincipal { field: &'static str, value: String },

    #[error("mapping contains duplicate source canister {0}")]
    DuplicateMappingSource(String),

    #[error("mapping contains duplicate target canister {0}")]
    DuplicateMappingTarget(String),

    #[error("mapping references unknown source canister {0}")]
    UnknownMappingSource(String),

    #[error("mapping is missing source canister {0}")]
    MissingMappingSource(String),

    #[error("fixed-identity member {source_canister} cannot be mapped to {target_canister}")]
    FixedIdentityRemap {
        source_canister: String,
        target_canister: String,
    },

    #[error("restore plan contains duplicate target canister {0}")]
    DuplicatePlanTarget(String),

    #[error("restore plan contains a parent cycle or unresolved dependency")]
    RestoreOrderCycle,
}

// Validate a user-supplied restore mapping before applying it to the manifest.
fn validate_mapping(mapping: &RestoreMapping) -> Result<(), RestorePlanError> {
    let mut sources = BTreeSet::new();
    let mut targets = BTreeSet::new();

    for entry in &mapping.members {
        validate_principal("mapping.members[].source_canister", &entry.source_canister)?;
        validate_principal("mapping.members[].target_canister", &entry.target_canister)?;

        if !sources.insert(entry.source_canister.clone()) {
            return Err(RestorePlanError::DuplicateMappingSource(
                entry.source_canister.clone(),
            ));
        }

        if !targets.insert(entry.target_canister.clone()) {
            return Err(RestorePlanError::DuplicateMappingTarget(
                entry.target_canister.clone(),
            ));
        }
    }

    Ok(())
}

// Ensure mappings only reference members declared in the manifest.
fn validate_mapping_sources(
    manifest: &FleetBackupManifest,
    mapping: &RestoreMapping,
) -> Result<(), RestorePlanError> {
    let sources = manifest
        .fleet
        .members
        .iter()
        .map(|member| member.canister_id.as_str())
        .collect::<BTreeSet<_>>();

    for entry in &mapping.members {
        if !sources.contains(entry.source_canister.as_str()) {
            return Err(RestorePlanError::UnknownMappingSource(
                entry.source_canister.clone(),
            ));
        }
    }

    Ok(())
}

// Resolve source manifest members into target restore members.
fn resolve_members(
    manifest: &FleetBackupManifest,
    mapping: Option<&RestoreMapping>,
) -> Result<Vec<RestorePlanMember>, RestorePlanError> {
    let mut plan_members = Vec::with_capacity(manifest.fleet.members.len());
    let mut targets = BTreeSet::new();
    let mut source_to_target = std::collections::BTreeMap::new();

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

// Resolve all concrete verification checks that apply to one restore member role.
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

// Return whether a verification check's role filter includes one member role.
fn verification_check_applies_to_role(check: &VerificationCheck, role: &str) -> bool {
    check.roles.is_empty() || check.roles.iter().any(|check_role| check_role == role)
}

// Resolve one member's target canister, enforcing identity continuity.
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

// Summarize identity and mapping decisions before ordering restore members.
fn restore_identity_summary(
    members: &[RestorePlanMember],
    mapping_supplied: bool,
) -> RestoreIdentitySummary {
    let mut summary = RestoreIdentitySummary {
        mapping_supplied,
        all_sources_mapped: false,
        fixed_members: 0,
        relocatable_members: 0,
        in_place_members: 0,
        mapped_members: 0,
        remapped_members: 0,
    };

    for member in members {
        match member.identity_mode {
            IdentityMode::Fixed => summary.fixed_members += 1,
            IdentityMode::Relocatable => summary.relocatable_members += 1,
        }

        if member.source_canister == member.target_canister {
            summary.in_place_members += 1;
        } else {
            summary.remapped_members += 1;
        }
        if mapping_supplied {
            summary.mapped_members += 1;
        }
    }

    summary.all_sources_mapped = mapping_supplied && summary.mapped_members == members.len();

    summary
}

// Summarize snapshot provenance completeness before ordering restore members.
fn restore_snapshot_summary(members: &[RestorePlanMember]) -> RestoreSnapshotSummary {
    let members_with_module_hash = members
        .iter()
        .filter(|member| member.source_snapshot.module_hash.is_some())
        .count();
    let members_with_wasm_hash = members
        .iter()
        .filter(|member| member.source_snapshot.wasm_hash.is_some())
        .count();
    let members_with_code_version = members
        .iter()
        .filter(|member| member.source_snapshot.code_version.is_some())
        .count();
    let members_with_checksum = members
        .iter()
        .filter(|member| member.source_snapshot.checksum.is_some())
        .count();

    RestoreSnapshotSummary {
        all_members_have_module_hash: members_with_module_hash == members.len(),
        all_members_have_wasm_hash: members_with_wasm_hash == members.len(),
        all_members_have_code_version: members_with_code_version == members.len(),
        all_members_have_checksum: members_with_checksum == members.len(),
        members_with_module_hash,
        members_with_wasm_hash,
        members_with_code_version,
        members_with_checksum,
    }
}

// Summarize whether restore planning has the minimum metadata required to execute.
fn restore_readiness_summary(
    snapshot: &RestoreSnapshotSummary,
    verification: &RestoreVerificationSummary,
) -> RestoreReadinessSummary {
    let mut reasons = Vec::new();

    if !snapshot.all_members_have_checksum {
        reasons.push("missing-snapshot-checksum".to_string());
    }
    if !verification.all_members_have_checks {
        reasons.push("missing-verification-checks".to_string());
    }

    RestoreReadinessSummary {
        ready: reasons.is_empty(),
        reasons,
    }
}

// Summarize restore verification work declared by the manifest and members.
fn restore_verification_summary(
    manifest: &FleetBackupManifest,
    members: &[RestorePlanMember],
) -> RestoreVerificationSummary {
    let fleet_checks = manifest.verification.fleet_checks.len();
    let member_check_groups = manifest.verification.member_checks.len();
    let member_checks = members
        .iter()
        .map(|member| member.verification_checks.len())
        .sum::<usize>();
    let members_with_checks = members
        .iter()
        .filter(|member| !member.verification_checks.is_empty())
        .count();

    RestoreVerificationSummary {
        verification_required: true,
        all_members_have_checks: members_with_checks == members.len(),
        fleet_checks,
        member_check_groups,
        member_checks,
        members_with_checks,
        total_checks: fleet_checks + member_checks,
    }
}

// Summarize the concrete restore operations implied by a no-mutation plan.
const fn restore_operation_summary(
    member_count: usize,
    verification_summary: &RestoreVerificationSummary,
) -> RestoreOperationSummary {
    RestoreOperationSummary {
        planned_snapshot_uploads: member_count,
        planned_snapshot_loads: member_count,
        planned_verification_checks: verification_summary.total_checks,
        planned_operations: member_count + member_count + verification_summary.total_checks,
    }
}

// Topologically order members using manifest parent relationships.
fn order_members(
    members: Vec<RestorePlanMember>,
) -> Result<Vec<RestorePlanMember>, RestorePlanError> {
    let mut remaining = members;
    let group_sources = remaining
        .iter()
        .map(|member| member.source_canister.clone())
        .collect::<BTreeSet<_>>();
    let mut emitted = BTreeSet::new();
    let mut ordered = Vec::with_capacity(remaining.len());

    while !remaining.is_empty() {
        let Some(index) = remaining
            .iter()
            .position(|member| parent_satisfied(member, &group_sources, &emitted))
        else {
            return Err(RestorePlanError::RestoreOrderCycle);
        };

        let mut member = remaining.remove(index);
        member.member_order = ordered.len();
        member.ordering_dependency = ordering_dependency(&member);
        emitted.insert(member.source_canister.clone());
        ordered.push(member);
    }

    Ok(ordered)
}

// Describe the topology dependency that controlled a member's restore ordering.
fn ordering_dependency(member: &RestorePlanMember) -> Option<RestoreOrderingDependency> {
    let parent_source = member.parent_source_canister.as_ref()?;
    let parent_target = member.parent_target_canister.as_ref()?;
    let relationship = RestoreOrderingRelationship::ParentBeforeChild;

    Some(RestoreOrderingDependency {
        source_canister: parent_source.clone(),
        target_canister: parent_target.clone(),
        relationship,
    })
}

// Summarize the dependency ordering metadata exposed in the restore plan.
fn restore_ordering_summary(members: &[RestorePlanMember]) -> RestoreOrderingSummary {
    let mut summary = RestoreOrderingSummary {
        ordered_members: members.len(),
        dependency_free_members: 0,
        parent_edges: 0,
    };

    for member in members {
        if member.ordering_dependency.is_some() {
            summary.parent_edges += 1;
        } else {
            summary.dependency_free_members += 1;
        }
    }

    summary
}

// Determine whether a member's in-group parent has already been emitted.
fn parent_satisfied(
    member: &RestorePlanMember,
    group_sources: &BTreeSet<String>,
    emitted: &BTreeSet<String>,
) -> bool {
    match &member.parent_source_canister {
        Some(parent) if group_sources.contains(parent) => emitted.contains(parent),
        _ => true,
    }
}

// Validate textual principal fields used in mappings.
fn validate_principal(field: &'static str, value: &str) -> Result<(), RestorePlanError> {
    Principal::from_str(value)
        .map(|_| ())
        .map_err(|_| RestorePlanError::InvalidPrincipal {
            field,
            value: value.to_string(),
        })
}
