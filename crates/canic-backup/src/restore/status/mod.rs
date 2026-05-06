use super::{RestorePhase, RestorePlan, RestorePlanMember};
use serde::{Deserialize, Serialize};

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
    #[serde(default)]
    pub planned_snapshot_uploads: usize,
    pub planned_snapshot_loads: usize,
    pub planned_code_reinstalls: usize,
    pub planned_verification_checks: usize,
    #[serde(default)]
    pub planned_operations: usize,
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
            planned_snapshot_uploads: plan
                .operation_summary
                .effective_planned_snapshot_uploads(plan.member_count),
            planned_snapshot_loads: plan.operation_summary.planned_snapshot_loads,
            planned_code_reinstalls: plan.operation_summary.planned_code_reinstalls,
            planned_verification_checks: plan.operation_summary.planned_verification_checks,
            planned_operations: plan
                .operation_summary
                .effective_planned_operations(plan.member_count),
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
