use crate::manifest::{
    FleetBackupManifest, FleetMember, IdentityMode, ManifestValidationError, SourceSnapshot,
    VerificationCheck,
};
use candid::Principal;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    str::FromStr,
};
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
    pub phases: Vec<RestorePhase>,
}

impl RestorePlan {
    /// Return all planned members in execution order.
    #[must_use]
    pub fn ordered_members(&self) -> Vec<&RestorePlanMember> {
        self.phases
            .iter()
            .flat_map(|phase| phase.members.iter())
            .collect()
    }
}

///
/// RestoreStatus
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RestoreStatus {
    pub status_version: u16,
    pub backup_id: String,
    pub source_environment: String,
    pub source_root_canister: String,
    pub topology_hash: String,
    pub ready: bool,
    pub readiness_reasons: Vec<String>,
    pub verification_required: bool,
    pub member_count: usize,
    pub phase_count: usize,
    pub planned_snapshot_loads: usize,
    pub planned_code_reinstalls: usize,
    pub planned_verification_checks: usize,
    pub phases: Vec<RestoreStatusPhase>,
}

impl RestoreStatus {
    /// Build the initial no-mutation restore status from a computed plan.
    #[must_use]
    pub fn from_plan(plan: &RestorePlan) -> Self {
        Self {
            status_version: 1,
            backup_id: plan.backup_id.clone(),
            source_environment: plan.source_environment.clone(),
            source_root_canister: plan.source_root_canister.clone(),
            topology_hash: plan.topology_hash.clone(),
            ready: plan.readiness_summary.ready,
            readiness_reasons: plan.readiness_summary.reasons.clone(),
            verification_required: plan.verification_summary.verification_required,
            member_count: plan.member_count,
            phase_count: plan.ordering_summary.phase_count,
            planned_snapshot_loads: plan.operation_summary.planned_snapshot_loads,
            planned_code_reinstalls: plan.operation_summary.planned_code_reinstalls,
            planned_verification_checks: plan.operation_summary.planned_verification_checks,
            phases: plan
                .phases
                .iter()
                .map(RestoreStatusPhase::from_plan_phase)
                .collect(),
        }
    }
}

///
/// RestoreStatusPhase
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RestoreStatusPhase {
    pub restore_group: u16,
    pub members: Vec<RestoreStatusMember>,
}

impl RestoreStatusPhase {
    // Build one status phase from one planned restore phase.
    fn from_plan_phase(phase: &RestorePhase) -> Self {
        Self {
            restore_group: phase.restore_group,
            members: phase
                .members
                .iter()
                .map(RestoreStatusMember::from_plan_member)
                .collect(),
        }
    }
}

///
/// RestoreStatusMember
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RestoreStatusMember {
    pub source_canister: String,
    pub target_canister: String,
    pub role: String,
    pub restore_group: u16,
    pub phase_order: usize,
    pub snapshot_id: String,
    pub artifact_path: String,
    pub state: RestoreMemberState,
}

impl RestoreStatusMember {
    // Build one member status row from one planned restore member.
    fn from_plan_member(member: &RestorePlanMember) -> Self {
        Self {
            source_canister: member.source_canister.clone(),
            target_canister: member.target_canister.clone(),
            role: member.role.clone(),
            restore_group: member.restore_group,
            phase_order: member.phase_order,
            snapshot_id: member.source_snapshot.snapshot_id.clone(),
            artifact_path: member.source_snapshot.artifact_path.clone(),
            state: RestoreMemberState::Planned,
        }
    }
}

///
/// RestoreMemberState
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RestoreMemberState {
    Planned,
}

///
/// RestoreApplyDryRun
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RestoreApplyDryRun {
    pub dry_run_version: u16,
    pub backup_id: String,
    pub ready: bool,
    pub readiness_reasons: Vec<String>,
    pub member_count: usize,
    pub phase_count: usize,
    pub status_supplied: bool,
    pub planned_snapshot_loads: usize,
    pub planned_code_reinstalls: usize,
    pub planned_verification_checks: usize,
    pub rendered_operations: usize,
    pub phases: Vec<RestoreApplyDryRunPhase>,
}

impl RestoreApplyDryRun {
    /// Build a no-mutation apply dry-run after validating optional status identity.
    pub fn try_from_plan(
        plan: &RestorePlan,
        status: Option<&RestoreStatus>,
    ) -> Result<Self, RestoreApplyDryRunError> {
        if let Some(status) = status {
            validate_restore_status_matches_plan(plan, status)?;
        }

        Ok(Self::from_validated_plan(plan, status))
    }

    // Build a no-mutation apply dry-run after any supplied status is validated.
    fn from_validated_plan(plan: &RestorePlan, status: Option<&RestoreStatus>) -> Self {
        let mut next_sequence = 0;
        let phases = plan
            .phases
            .iter()
            .map(|phase| RestoreApplyDryRunPhase::from_plan_phase(phase, &mut next_sequence))
            .collect::<Vec<_>>();
        let rendered_operations = phases
            .iter()
            .map(|phase| phase.operations.len())
            .sum::<usize>();

        Self {
            dry_run_version: 1,
            backup_id: plan.backup_id.clone(),
            ready: status.map_or(plan.readiness_summary.ready, |status| status.ready),
            readiness_reasons: status.map_or_else(
                || plan.readiness_summary.reasons.clone(),
                |status| status.readiness_reasons.clone(),
            ),
            member_count: plan.member_count,
            phase_count: plan.ordering_summary.phase_count,
            status_supplied: status.is_some(),
            planned_snapshot_loads: plan.operation_summary.planned_snapshot_loads,
            planned_code_reinstalls: plan.operation_summary.planned_code_reinstalls,
            planned_verification_checks: plan.operation_summary.planned_verification_checks,
            rendered_operations,
            phases,
        }
    }
}

// Validate that a supplied restore status belongs to the restore plan.
fn validate_restore_status_matches_plan(
    plan: &RestorePlan,
    status: &RestoreStatus,
) -> Result<(), RestoreApplyDryRunError> {
    validate_status_string_field("backup_id", &plan.backup_id, &status.backup_id)?;
    validate_status_string_field(
        "source_environment",
        &plan.source_environment,
        &status.source_environment,
    )?;
    validate_status_string_field(
        "source_root_canister",
        &plan.source_root_canister,
        &status.source_root_canister,
    )?;
    validate_status_string_field("topology_hash", &plan.topology_hash, &status.topology_hash)?;
    validate_status_usize_field("member_count", plan.member_count, status.member_count)?;
    validate_status_usize_field(
        "phase_count",
        plan.ordering_summary.phase_count,
        status.phase_count,
    )?;
    Ok(())
}

// Validate one string field shared by restore plan and status.
fn validate_status_string_field(
    field: &'static str,
    plan: &str,
    status: &str,
) -> Result<(), RestoreApplyDryRunError> {
    if plan == status {
        return Ok(());
    }

    Err(RestoreApplyDryRunError::StatusPlanMismatch {
        field,
        plan: plan.to_string(),
        status: status.to_string(),
    })
}

// Validate one numeric field shared by restore plan and status.
const fn validate_status_usize_field(
    field: &'static str,
    plan: usize,
    status: usize,
) -> Result<(), RestoreApplyDryRunError> {
    if plan == status {
        return Ok(());
    }

    Err(RestoreApplyDryRunError::StatusPlanCountMismatch {
        field,
        plan,
        status,
    })
}

///
/// RestoreApplyDryRunPhase
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RestoreApplyDryRunPhase {
    pub restore_group: u16,
    pub operations: Vec<RestoreApplyDryRunOperation>,
}

impl RestoreApplyDryRunPhase {
    // Build one dry-run phase from one restore plan phase.
    fn from_plan_phase(phase: &RestorePhase, next_sequence: &mut usize) -> Self {
        let mut operations = Vec::new();

        for member in &phase.members {
            push_member_operation(
                &mut operations,
                next_sequence,
                RestoreApplyOperationKind::UploadSnapshot,
                member,
                None,
            );
            push_member_operation(
                &mut operations,
                next_sequence,
                RestoreApplyOperationKind::LoadSnapshot,
                member,
                None,
            );
            push_member_operation(
                &mut operations,
                next_sequence,
                RestoreApplyOperationKind::ReinstallCode,
                member,
                None,
            );

            for check in &member.verification_checks {
                push_member_operation(
                    &mut operations,
                    next_sequence,
                    RestoreApplyOperationKind::VerifyMember,
                    member,
                    Some(check),
                );
            }
        }

        Self {
            restore_group: phase.restore_group,
            operations,
        }
    }
}

// Append one member-level dry-run operation using the current phase order.
fn push_member_operation(
    operations: &mut Vec<RestoreApplyDryRunOperation>,
    next_sequence: &mut usize,
    operation: RestoreApplyOperationKind,
    member: &RestorePlanMember,
    check: Option<&VerificationCheck>,
) {
    let sequence = *next_sequence;
    *next_sequence += 1;

    operations.push(RestoreApplyDryRunOperation {
        sequence,
        operation,
        restore_group: member.restore_group,
        phase_order: member.phase_order,
        source_canister: member.source_canister.clone(),
        target_canister: member.target_canister.clone(),
        role: member.role.clone(),
        snapshot_id: Some(member.source_snapshot.snapshot_id.clone()),
        artifact_path: Some(member.source_snapshot.artifact_path.clone()),
        verification_kind: check.map(|check| check.kind.clone()),
        verification_method: check.and_then(|check| check.method.clone()),
    });
}

///
/// RestoreApplyDryRunOperation
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RestoreApplyDryRunOperation {
    pub sequence: usize,
    pub operation: RestoreApplyOperationKind,
    pub restore_group: u16,
    pub phase_order: usize,
    pub source_canister: String,
    pub target_canister: String,
    pub role: String,
    pub snapshot_id: Option<String>,
    pub artifact_path: Option<String>,
    pub verification_kind: Option<String>,
    pub verification_method: Option<String>,
}

///
/// RestoreApplyOperationKind
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RestoreApplyOperationKind {
    UploadSnapshot,
    LoadSnapshot,
    ReinstallCode,
    VerifyMember,
}

///
/// RestoreApplyDryRunError
///

#[derive(Debug, ThisError)]
pub enum RestoreApplyDryRunError {
    #[error("restore status field {field} does not match plan: plan={plan}, status={status}")]
    StatusPlanMismatch {
        field: &'static str,
        plan: String,
        status: String,
    },

    #[error("restore status field {field} does not match plan: plan={plan}, status={status}")]
    StatusPlanCountMismatch {
        field: &'static str,
        plan: usize,
        status: usize,
    },
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
    pub planned_snapshot_loads: usize,
    pub planned_code_reinstalls: usize,
    pub planned_verification_checks: usize,
    pub planned_phases: usize,
}

///
/// RestoreOrderingSummary
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RestoreOrderingSummary {
    pub phase_count: usize,
    pub dependency_free_members: usize,
    pub in_group_parent_edges: usize,
    pub cross_group_parent_edges: usize,
}

///
/// RestorePhase
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RestorePhase {
    pub restore_group: u16,
    pub members: Vec<RestorePlanMember>,
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
    pub phase_order: usize,
    pub restore_group: u16,
    pub identity_mode: IdentityMode,
    pub verification_class: String,
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
    ParentInSameGroup,
    ParentInEarlierGroup,
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
        validate_restore_group_dependencies(&members)?;
        let phases = group_and_order_members(members)?;
        let ordering_summary = restore_ordering_summary(&phases);
        let operation_summary =
            restore_operation_summary(manifest.fleet.members.len(), &verification_summary, &phases);

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
            phases,
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

    #[error("restore group {0} contains a parent cycle or unresolved dependency")]
    RestoreOrderCycle(u16),

    #[error(
        "restore plan places parent {parent_source_canister} in group {parent_restore_group} after child {child_source_canister} in group {child_restore_group}"
    )]
    ParentRestoreGroupAfterChild {
        child_source_canister: String,
        parent_source_canister: String,
        child_restore_group: u16,
        parent_restore_group: u16,
    },
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
            phase_order: 0,
            restore_group: member.restore_group,
            identity_mode: member.identity_mode.clone(),
            verification_class: member.verification_class.clone(),
            verification_checks: member.verification_checks.clone(),
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

// Summarize identity and mapping decisions before grouping restore phases.
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

// Summarize snapshot provenance completeness before grouping restore phases.
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

// Summarize whether restore planning has the metadata required for automation.
fn restore_readiness_summary(
    snapshot: &RestoreSnapshotSummary,
    verification: &RestoreVerificationSummary,
) -> RestoreReadinessSummary {
    let mut reasons = Vec::new();

    if !snapshot.all_members_have_module_hash {
        reasons.push("missing-module-hash".to_string());
    }
    if !snapshot.all_members_have_wasm_hash {
        reasons.push("missing-wasm-hash".to_string());
    }
    if !snapshot.all_members_have_code_version {
        reasons.push("missing-code-version".to_string());
    }
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
    let role_check_counts = manifest
        .verification
        .member_checks
        .iter()
        .map(|group| (group.role.as_str(), group.checks.len()))
        .collect::<BTreeMap<_, _>>();
    let inline_member_checks = members
        .iter()
        .map(|member| member.verification_checks.len())
        .sum::<usize>();
    let role_member_checks = members
        .iter()
        .map(|member| {
            role_check_counts
                .get(member.role.as_str())
                .copied()
                .unwrap_or(0)
        })
        .sum::<usize>();
    let member_checks = inline_member_checks + role_member_checks;
    let members_with_checks = members
        .iter()
        .filter(|member| {
            !member.verification_checks.is_empty()
                || role_check_counts.contains_key(member.role.as_str())
        })
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
    phases: &[RestorePhase],
) -> RestoreOperationSummary {
    RestoreOperationSummary {
        planned_snapshot_loads: member_count,
        planned_code_reinstalls: member_count,
        planned_verification_checks: verification_summary.total_checks,
        planned_phases: phases.len(),
    }
}

// Reject group assignments that would restore a child before its parent.
fn validate_restore_group_dependencies(
    members: &[RestorePlanMember],
) -> Result<(), RestorePlanError> {
    let groups_by_source = members
        .iter()
        .map(|member| (member.source_canister.as_str(), member.restore_group))
        .collect::<BTreeMap<_, _>>();

    for member in members {
        let Some(parent) = &member.parent_source_canister else {
            continue;
        };
        let Some(parent_group) = groups_by_source.get(parent.as_str()) else {
            continue;
        };

        if *parent_group > member.restore_group {
            return Err(RestorePlanError::ParentRestoreGroupAfterChild {
                child_source_canister: member.source_canister.clone(),
                parent_source_canister: parent.clone(),
                child_restore_group: member.restore_group,
                parent_restore_group: *parent_group,
            });
        }
    }

    Ok(())
}

// Group members and apply parent-before-child ordering inside each group.
fn group_and_order_members(
    members: Vec<RestorePlanMember>,
) -> Result<Vec<RestorePhase>, RestorePlanError> {
    let mut groups = BTreeMap::<u16, Vec<RestorePlanMember>>::new();
    for member in members {
        groups.entry(member.restore_group).or_default().push(member);
    }

    groups
        .into_iter()
        .map(|(restore_group, members)| {
            let members = order_group(restore_group, members)?;
            Ok(RestorePhase {
                restore_group,
                members,
            })
        })
        .collect()
}

// Topologically order one group using manifest parent relationships.
fn order_group(
    restore_group: u16,
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
            return Err(RestorePlanError::RestoreOrderCycle(restore_group));
        };

        let mut member = remaining.remove(index);
        member.phase_order = ordered.len();
        member.ordering_dependency = ordering_dependency(&member, &group_sources);
        emitted.insert(member.source_canister.clone());
        ordered.push(member);
    }

    Ok(ordered)
}

// Describe the topology dependency that controlled a member's restore ordering.
fn ordering_dependency(
    member: &RestorePlanMember,
    group_sources: &BTreeSet<String>,
) -> Option<RestoreOrderingDependency> {
    let parent_source = member.parent_source_canister.as_ref()?;
    let parent_target = member.parent_target_canister.as_ref()?;
    let relationship = if group_sources.contains(parent_source) {
        RestoreOrderingRelationship::ParentInSameGroup
    } else {
        RestoreOrderingRelationship::ParentInEarlierGroup
    };

    Some(RestoreOrderingDependency {
        source_canister: parent_source.clone(),
        target_canister: parent_target.clone(),
        relationship,
    })
}

// Summarize the dependency ordering metadata exposed in the restore plan.
fn restore_ordering_summary(phases: &[RestorePhase]) -> RestoreOrderingSummary {
    let mut summary = RestoreOrderingSummary {
        phase_count: phases.len(),
        dependency_free_members: 0,
        in_group_parent_edges: 0,
        cross_group_parent_edges: 0,
    };

    for member in phases.iter().flat_map(|phase| phase.members.iter()) {
        match &member.ordering_dependency {
            Some(dependency)
                if dependency.relationship == RestoreOrderingRelationship::ParentInSameGroup =>
            {
                summary.in_group_parent_edges += 1;
            }
            Some(dependency)
                if dependency.relationship == RestoreOrderingRelationship::ParentInEarlierGroup =>
            {
                summary.cross_group_parent_edges += 1;
            }
            Some(_) => {}
            None => summary.dependency_free_members += 1,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::{
        BackupUnit, BackupUnitKind, ConsistencyMode, ConsistencySection, FleetSection,
        MemberVerificationChecks, SourceMetadata, SourceSnapshot, ToolMetadata, VerificationCheck,
        VerificationPlan,
    };

    const ROOT: &str = "aaaaa-aa";
    const CHILD: &str = "renrk-eyaaa-aaaaa-aaada-cai";
    const CHILD_TWO: &str = "r7inp-6aaaa-aaaaa-aaabq-cai";
    const TARGET: &str = "rno2w-sqaaa-aaaaa-aaacq-cai";
    const HASH: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    // Build one valid manifest with a parent and child in the same restore group.
    fn valid_manifest(identity_mode: IdentityMode) -> FleetBackupManifest {
        FleetBackupManifest {
            manifest_version: 1,
            backup_id: "fbk_test_001".to_string(),
            created_at: "2026-04-10T12:00:00Z".to_string(),
            tool: ToolMetadata {
                name: "canic".to_string(),
                version: "v1".to_string(),
            },
            source: SourceMetadata {
                environment: "local".to_string(),
                root_canister: ROOT.to_string(),
            },
            consistency: ConsistencySection {
                mode: ConsistencyMode::CrashConsistent,
                backup_units: vec![BackupUnit {
                    unit_id: "whole-fleet".to_string(),
                    kind: BackupUnitKind::WholeFleet,
                    roles: vec!["root".to_string(), "app".to_string()],
                    consistency_reason: None,
                    dependency_closure: Vec::new(),
                    topology_validation: "subtree-closed".to_string(),
                    quiescence_strategy: None,
                }],
            },
            fleet: FleetSection {
                topology_hash_algorithm: "sha256".to_string(),
                topology_hash_input: "sorted(pid,parent_pid,role,module_hash)".to_string(),
                discovery_topology_hash: HASH.to_string(),
                pre_snapshot_topology_hash: HASH.to_string(),
                topology_hash: HASH.to_string(),
                members: vec![
                    fleet_member("app", CHILD, Some(ROOT), identity_mode, 1),
                    fleet_member("root", ROOT, None, IdentityMode::Fixed, 1),
                ],
            },
            verification: VerificationPlan {
                fleet_checks: Vec::new(),
                member_checks: Vec::new(),
            },
        }
    }

    // Build one manifest member for restore planning tests.
    fn fleet_member(
        role: &str,
        canister_id: &str,
        parent_canister_id: Option<&str>,
        identity_mode: IdentityMode,
        restore_group: u16,
    ) -> FleetMember {
        FleetMember {
            role: role.to_string(),
            canister_id: canister_id.to_string(),
            parent_canister_id: parent_canister_id.map(str::to_string),
            subnet_canister_id: None,
            controller_hint: Some(ROOT.to_string()),
            identity_mode,
            restore_group,
            verification_class: "basic".to_string(),
            verification_checks: vec![VerificationCheck {
                kind: "call".to_string(),
                method: Some("canic_ready".to_string()),
                roles: Vec::new(),
            }],
            source_snapshot: SourceSnapshot {
                snapshot_id: format!("snap-{role}"),
                module_hash: Some(HASH.to_string()),
                wasm_hash: Some(HASH.to_string()),
                code_version: Some("v0.30.0".to_string()),
                artifact_path: format!("artifacts/{role}"),
                checksum_algorithm: "sha256".to_string(),
                checksum: Some(HASH.to_string()),
            },
        }
    }

    // Ensure in-place restore planning sorts parent before child.
    #[test]
    fn in_place_plan_orders_parent_before_child() {
        let manifest = valid_manifest(IdentityMode::Relocatable);

        let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
        let ordered = plan.ordered_members();

        assert_eq!(plan.backup_id, "fbk_test_001");
        assert_eq!(plan.source_environment, "local");
        assert_eq!(plan.source_root_canister, ROOT);
        assert_eq!(plan.topology_hash, HASH);
        assert_eq!(plan.member_count, 2);
        assert_eq!(plan.identity_summary.fixed_members, 1);
        assert_eq!(plan.identity_summary.relocatable_members, 1);
        assert_eq!(plan.identity_summary.in_place_members, 2);
        assert_eq!(plan.identity_summary.mapped_members, 0);
        assert_eq!(plan.identity_summary.remapped_members, 0);
        assert!(plan.verification_summary.verification_required);
        assert!(plan.verification_summary.all_members_have_checks);
        assert!(plan.readiness_summary.ready);
        assert!(plan.readiness_summary.reasons.is_empty());
        assert_eq!(plan.verification_summary.fleet_checks, 0);
        assert_eq!(plan.verification_summary.member_check_groups, 0);
        assert_eq!(plan.verification_summary.member_checks, 2);
        assert_eq!(plan.verification_summary.members_with_checks, 2);
        assert_eq!(plan.verification_summary.total_checks, 2);
        assert_eq!(plan.ordering_summary.phase_count, 1);
        assert_eq!(plan.ordering_summary.dependency_free_members, 1);
        assert_eq!(plan.ordering_summary.in_group_parent_edges, 1);
        assert_eq!(plan.ordering_summary.cross_group_parent_edges, 0);
        assert_eq!(ordered[0].phase_order, 0);
        assert_eq!(ordered[1].phase_order, 1);
        assert_eq!(ordered[0].source_canister, ROOT);
        assert_eq!(ordered[1].source_canister, CHILD);
        assert_eq!(
            ordered[1].ordering_dependency,
            Some(RestoreOrderingDependency {
                source_canister: ROOT.to_string(),
                target_canister: ROOT.to_string(),
                relationship: RestoreOrderingRelationship::ParentInSameGroup,
            })
        );
    }

    // Ensure cross-group parent dependencies are exposed when the parent phase is earlier.
    #[test]
    fn plan_reports_parent_dependency_from_earlier_group() {
        let mut manifest = valid_manifest(IdentityMode::Relocatable);
        manifest.fleet.members[0].restore_group = 2;
        manifest.fleet.members[1].restore_group = 1;

        let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
        let ordered = plan.ordered_members();

        assert_eq!(plan.phases.len(), 2);
        assert_eq!(plan.ordering_summary.phase_count, 2);
        assert_eq!(plan.ordering_summary.dependency_free_members, 1);
        assert_eq!(plan.ordering_summary.in_group_parent_edges, 0);
        assert_eq!(plan.ordering_summary.cross_group_parent_edges, 1);
        assert_eq!(ordered[0].source_canister, ROOT);
        assert_eq!(ordered[1].source_canister, CHILD);
        assert_eq!(
            ordered[1].ordering_dependency,
            Some(RestoreOrderingDependency {
                source_canister: ROOT.to_string(),
                target_canister: ROOT.to_string(),
                relationship: RestoreOrderingRelationship::ParentInEarlierGroup,
            })
        );
    }

    // Ensure restore planning fails when groups would restore a child before its parent.
    #[test]
    fn plan_rejects_parent_in_later_restore_group() {
        let mut manifest = valid_manifest(IdentityMode::Relocatable);
        manifest.fleet.members[0].restore_group = 1;
        manifest.fleet.members[1].restore_group = 2;

        let err = RestorePlanner::plan(&manifest, None)
            .expect_err("parent-after-child group ordering should fail");

        assert!(matches!(
            err,
            RestorePlanError::ParentRestoreGroupAfterChild { .. }
        ));
    }

    // Ensure fixed identities cannot be remapped.
    #[test]
    fn fixed_identity_member_cannot_be_remapped() {
        let manifest = valid_manifest(IdentityMode::Fixed);
        let mapping = RestoreMapping {
            members: vec![
                RestoreMappingEntry {
                    source_canister: ROOT.to_string(),
                    target_canister: ROOT.to_string(),
                },
                RestoreMappingEntry {
                    source_canister: CHILD.to_string(),
                    target_canister: TARGET.to_string(),
                },
            ],
        };

        let err = RestorePlanner::plan(&manifest, Some(&mapping))
            .expect_err("fixed member remap should fail");

        assert!(matches!(err, RestorePlanError::FixedIdentityRemap { .. }));
    }

    // Ensure relocatable identities may be mapped when all members are covered.
    #[test]
    fn relocatable_member_can_be_mapped() {
        let manifest = valid_manifest(IdentityMode::Relocatable);
        let mapping = RestoreMapping {
            members: vec![
                RestoreMappingEntry {
                    source_canister: ROOT.to_string(),
                    target_canister: ROOT.to_string(),
                },
                RestoreMappingEntry {
                    source_canister: CHILD.to_string(),
                    target_canister: TARGET.to_string(),
                },
            ],
        };

        let plan = RestorePlanner::plan(&manifest, Some(&mapping)).expect("plan should build");
        let child = plan
            .ordered_members()
            .into_iter()
            .find(|member| member.source_canister == CHILD)
            .expect("child member should be planned");

        assert_eq!(plan.identity_summary.fixed_members, 1);
        assert_eq!(plan.identity_summary.relocatable_members, 1);
        assert_eq!(plan.identity_summary.in_place_members, 1);
        assert_eq!(plan.identity_summary.mapped_members, 2);
        assert_eq!(plan.identity_summary.remapped_members, 1);
        assert_eq!(child.target_canister, TARGET);
        assert_eq!(child.parent_target_canister, Some(ROOT.to_string()));
    }

    // Ensure restore plans carry enough metadata for operator preflight.
    #[test]
    fn plan_members_include_snapshot_and_verification_metadata() {
        let manifest = valid_manifest(IdentityMode::Relocatable);

        let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
        let root = plan
            .ordered_members()
            .into_iter()
            .find(|member| member.source_canister == ROOT)
            .expect("root member should be planned");

        assert_eq!(root.identity_mode, IdentityMode::Fixed);
        assert_eq!(root.verification_class, "basic");
        assert_eq!(root.verification_checks[0].kind, "call");
        assert_eq!(root.source_snapshot.snapshot_id, "snap-root");
        assert_eq!(root.source_snapshot.artifact_path, "artifacts/root");
    }

    // Ensure restore plans make mapping mode explicit.
    #[test]
    fn plan_includes_mapping_summary() {
        let manifest = valid_manifest(IdentityMode::Relocatable);
        let in_place = RestorePlanner::plan(&manifest, None).expect("plan should build");

        assert!(!in_place.identity_summary.mapping_supplied);
        assert!(!in_place.identity_summary.all_sources_mapped);
        assert_eq!(in_place.identity_summary.mapped_members, 0);

        let mapping = RestoreMapping {
            members: vec![
                RestoreMappingEntry {
                    source_canister: ROOT.to_string(),
                    target_canister: ROOT.to_string(),
                },
                RestoreMappingEntry {
                    source_canister: CHILD.to_string(),
                    target_canister: TARGET.to_string(),
                },
            ],
        };
        let mapped = RestorePlanner::plan(&manifest, Some(&mapping)).expect("plan should build");

        assert!(mapped.identity_summary.mapping_supplied);
        assert!(mapped.identity_summary.all_sources_mapped);
        assert_eq!(mapped.identity_summary.mapped_members, 2);
        assert_eq!(mapped.identity_summary.remapped_members, 1);
    }

    // Ensure restore plans summarize snapshot provenance completeness.
    #[test]
    fn plan_includes_snapshot_summary() {
        let mut manifest = valid_manifest(IdentityMode::Relocatable);
        manifest.fleet.members[1].source_snapshot.module_hash = None;
        manifest.fleet.members[1].source_snapshot.wasm_hash = None;
        manifest.fleet.members[1].source_snapshot.checksum = None;

        let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");

        assert!(!plan.snapshot_summary.all_members_have_module_hash);
        assert!(!plan.snapshot_summary.all_members_have_wasm_hash);
        assert!(plan.snapshot_summary.all_members_have_code_version);
        assert!(!plan.snapshot_summary.all_members_have_checksum);
        assert_eq!(plan.snapshot_summary.members_with_module_hash, 1);
        assert_eq!(plan.snapshot_summary.members_with_wasm_hash, 1);
        assert_eq!(plan.snapshot_summary.members_with_code_version, 2);
        assert_eq!(plan.snapshot_summary.members_with_checksum, 1);
        assert!(!plan.readiness_summary.ready);
        assert_eq!(
            plan.readiness_summary.reasons,
            [
                "missing-module-hash",
                "missing-wasm-hash",
                "missing-snapshot-checksum"
            ]
        );
    }

    // Ensure restore plans summarize manifest-level verification work.
    #[test]
    fn plan_includes_verification_summary() {
        let mut manifest = valid_manifest(IdentityMode::Relocatable);
        manifest.verification.fleet_checks.push(VerificationCheck {
            kind: "fleet-ready".to_string(),
            method: None,
            roles: Vec::new(),
        });
        manifest
            .verification
            .member_checks
            .push(MemberVerificationChecks {
                role: "app".to_string(),
                checks: vec![VerificationCheck {
                    kind: "app-ready".to_string(),
                    method: Some("ready".to_string()),
                    roles: Vec::new(),
                }],
            });

        let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");

        assert!(plan.verification_summary.verification_required);
        assert!(plan.verification_summary.all_members_have_checks);
        assert_eq!(plan.verification_summary.fleet_checks, 1);
        assert_eq!(plan.verification_summary.member_check_groups, 1);
        assert_eq!(plan.verification_summary.member_checks, 3);
        assert_eq!(plan.verification_summary.members_with_checks, 2);
        assert_eq!(plan.verification_summary.total_checks, 4);
    }

    // Ensure restore plans summarize the concrete operation counts automation will schedule.
    #[test]
    fn plan_includes_operation_summary() {
        let manifest = valid_manifest(IdentityMode::Relocatable);

        let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");

        assert_eq!(plan.operation_summary.planned_snapshot_loads, 2);
        assert_eq!(plan.operation_summary.planned_code_reinstalls, 2);
        assert_eq!(plan.operation_summary.planned_verification_checks, 2);
        assert_eq!(plan.operation_summary.planned_phases, 1);
    }

    // Ensure initial restore status mirrors the no-mutation restore plan.
    #[test]
    fn restore_status_starts_all_members_as_planned() {
        let manifest = valid_manifest(IdentityMode::Relocatable);

        let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
        let status = RestoreStatus::from_plan(&plan);

        assert_eq!(status.status_version, 1);
        assert_eq!(status.backup_id.as_str(), plan.backup_id.as_str());
        assert_eq!(
            status.source_environment.as_str(),
            plan.source_environment.as_str()
        );
        assert_eq!(
            status.source_root_canister.as_str(),
            plan.source_root_canister.as_str()
        );
        assert_eq!(status.topology_hash.as_str(), plan.topology_hash.as_str());
        assert!(status.ready);
        assert!(status.readiness_reasons.is_empty());
        assert!(status.verification_required);
        assert_eq!(status.member_count, 2);
        assert_eq!(status.phase_count, 1);
        assert_eq!(status.planned_snapshot_loads, 2);
        assert_eq!(status.planned_code_reinstalls, 2);
        assert_eq!(status.planned_verification_checks, 2);
        assert_eq!(status.phases.len(), 1);
        assert_eq!(status.phases[0].restore_group, 1);
        assert_eq!(status.phases[0].members.len(), 2);
        assert_eq!(
            status.phases[0].members[0].state,
            RestoreMemberState::Planned
        );
        assert_eq!(status.phases[0].members[0].source_canister, ROOT);
        assert_eq!(status.phases[0].members[0].target_canister, ROOT);
        assert_eq!(status.phases[0].members[0].snapshot_id, "snap-root");
        assert_eq!(status.phases[0].members[0].artifact_path, "artifacts/root");
        assert_eq!(
            status.phases[0].members[1].state,
            RestoreMemberState::Planned
        );
        assert_eq!(status.phases[0].members[1].source_canister, CHILD);
    }

    // Ensure apply dry-runs render ordered operations without mutating targets.
    #[test]
    fn apply_dry_run_renders_ordered_member_operations() {
        let manifest = valid_manifest(IdentityMode::Relocatable);

        let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
        let status = RestoreStatus::from_plan(&plan);
        let dry_run =
            RestoreApplyDryRun::try_from_plan(&plan, Some(&status)).expect("dry-run should build");

        assert_eq!(dry_run.dry_run_version, 1);
        assert_eq!(dry_run.backup_id.as_str(), "fbk_test_001");
        assert!(dry_run.ready);
        assert!(dry_run.status_supplied);
        assert_eq!(dry_run.member_count, 2);
        assert_eq!(dry_run.phase_count, 1);
        assert_eq!(dry_run.planned_snapshot_loads, 2);
        assert_eq!(dry_run.planned_code_reinstalls, 2);
        assert_eq!(dry_run.planned_verification_checks, 2);
        assert_eq!(dry_run.rendered_operations, 8);
        assert_eq!(dry_run.phases.len(), 1);

        let operations = &dry_run.phases[0].operations;
        assert_eq!(operations[0].sequence, 0);
        assert_eq!(
            operations[0].operation,
            RestoreApplyOperationKind::UploadSnapshot
        );
        assert_eq!(operations[0].source_canister, ROOT);
        assert_eq!(operations[0].target_canister, ROOT);
        assert_eq!(operations[0].snapshot_id, Some("snap-root".to_string()));
        assert_eq!(
            operations[0].artifact_path,
            Some("artifacts/root".to_string())
        );
        assert_eq!(
            operations[1].operation,
            RestoreApplyOperationKind::LoadSnapshot
        );
        assert_eq!(
            operations[2].operation,
            RestoreApplyOperationKind::ReinstallCode
        );
        assert_eq!(
            operations[3].operation,
            RestoreApplyOperationKind::VerifyMember
        );
        assert_eq!(operations[3].verification_kind, Some("call".to_string()));
        assert_eq!(
            operations[3].verification_method,
            Some("canic_ready".to_string())
        );
        assert_eq!(operations[4].source_canister, CHILD);
        assert_eq!(
            operations[7].operation,
            RestoreApplyOperationKind::VerifyMember
        );
    }

    // Ensure apply dry-run operation sequences remain unique across phases.
    #[test]
    fn apply_dry_run_sequences_operations_across_phases() {
        let mut manifest = valid_manifest(IdentityMode::Relocatable);
        manifest.fleet.members[0].restore_group = 2;

        let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
        let dry_run = RestoreApplyDryRun::try_from_plan(&plan, None).expect("dry-run should build");

        assert_eq!(dry_run.phases.len(), 2);
        assert_eq!(dry_run.rendered_operations, 8);
        assert_eq!(dry_run.phases[0].operations[0].sequence, 0);
        assert_eq!(dry_run.phases[0].operations[3].sequence, 3);
        assert_eq!(dry_run.phases[1].operations[0].sequence, 4);
        assert_eq!(dry_run.phases[1].operations[3].sequence, 7);
    }

    // Ensure apply dry-runs reject status files that do not match the plan.
    #[test]
    fn apply_dry_run_rejects_mismatched_status() {
        let manifest = valid_manifest(IdentityMode::Relocatable);

        let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
        let mut status = RestoreStatus::from_plan(&plan);
        status.backup_id = "other-backup".to_string();

        let err = RestoreApplyDryRun::try_from_plan(&plan, Some(&status))
            .expect_err("mismatched status should fail");

        assert!(matches!(
            err,
            RestoreApplyDryRunError::StatusPlanMismatch {
                field: "backup_id",
                ..
            }
        ));
    }

    // Ensure role-level verification checks are counted once per matching member.
    #[test]
    fn plan_expands_role_verification_checks_per_matching_member() {
        let mut manifest = valid_manifest(IdentityMode::Relocatable);
        manifest.fleet.members.push(fleet_member(
            "app",
            CHILD_TWO,
            Some(ROOT),
            IdentityMode::Relocatable,
            1,
        ));
        manifest
            .verification
            .member_checks
            .push(MemberVerificationChecks {
                role: "app".to_string(),
                checks: vec![VerificationCheck {
                    kind: "app-ready".to_string(),
                    method: Some("ready".to_string()),
                    roles: Vec::new(),
                }],
            });

        let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");

        assert_eq!(plan.verification_summary.fleet_checks, 0);
        assert_eq!(plan.verification_summary.member_check_groups, 1);
        assert_eq!(plan.verification_summary.member_checks, 5);
        assert_eq!(plan.verification_summary.members_with_checks, 3);
        assert_eq!(plan.verification_summary.total_checks, 5);
    }

    // Ensure mapped restores must cover every source member.
    #[test]
    fn mapped_restore_requires_complete_mapping() {
        let manifest = valid_manifest(IdentityMode::Relocatable);
        let mapping = RestoreMapping {
            members: vec![RestoreMappingEntry {
                source_canister: ROOT.to_string(),
                target_canister: ROOT.to_string(),
            }],
        };

        let err = RestorePlanner::plan(&manifest, Some(&mapping))
            .expect_err("incomplete mapping should fail");

        assert!(matches!(err, RestorePlanError::MissingMappingSource(_)));
    }

    // Ensure mappings cannot silently include canisters outside the manifest.
    #[test]
    fn mapped_restore_rejects_unknown_mapping_sources() {
        let manifest = valid_manifest(IdentityMode::Relocatable);
        let unknown = "rdmx6-jaaaa-aaaaa-aaadq-cai";
        let mapping = RestoreMapping {
            members: vec![
                RestoreMappingEntry {
                    source_canister: ROOT.to_string(),
                    target_canister: ROOT.to_string(),
                },
                RestoreMappingEntry {
                    source_canister: CHILD.to_string(),
                    target_canister: TARGET.to_string(),
                },
                RestoreMappingEntry {
                    source_canister: unknown.to_string(),
                    target_canister: unknown.to_string(),
                },
            ],
        };

        let err = RestorePlanner::plan(&manifest, Some(&mapping))
            .expect_err("unknown mapping source should fail");

        assert!(matches!(err, RestorePlanError::UnknownMappingSource(_)));
    }

    // Ensure duplicate target mappings fail before a plan is produced.
    #[test]
    fn duplicate_mapping_targets_fail_validation() {
        let manifest = valid_manifest(IdentityMode::Relocatable);
        let mapping = RestoreMapping {
            members: vec![
                RestoreMappingEntry {
                    source_canister: ROOT.to_string(),
                    target_canister: ROOT.to_string(),
                },
                RestoreMappingEntry {
                    source_canister: CHILD.to_string(),
                    target_canister: ROOT.to_string(),
                },
            ],
        };

        let err = RestorePlanner::plan(&manifest, Some(&mapping))
            .expect_err("duplicate targets should fail");

        assert!(matches!(err, RestorePlanError::DuplicateMappingTarget(_)));
    }
}
