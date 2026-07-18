//! Module: restore::plan::types
//!
//! Responsibility: define serialized restore plan and mapping data shapes.
//! Does not own: restore planning decisions, artifact validation, or execution.
//! Boundary: data contracts shared by restore planning, apply dry-runs, and runners.

use crate::manifest::{IdentityMode, SourceSnapshot, VerificationCheck};

use serde::{Deserialize, Serialize};

///
/// RestoreMapping
///
/// Optional operator mapping from source canisters to restore targets.
/// Owned by restore planning and accepted by restore plan construction.
///

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RestoreMapping {
    pub members: Vec<RestoreMappingEntry>,
}

impl RestoreMapping {
    /// Resolve the target canister for one source member.
    pub(super) fn target_for(&self, source_canister: &str) -> Option<&str> {
        self.members
            .iter()
            .find(|entry| entry.source_canister == source_canister)
            .map(|entry| entry.target_canister.as_str())
    }
}

///
/// RestoreMappingEntry
///
/// One source-to-target canister mapping row.
/// Owned by restore planning and embedded in `RestoreMapping`.
///

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RestoreMappingEntry {
    pub source_canister: String,
    pub target_canister: String,
}

///
/// RestorePlan
///
/// No-mutation restore plan derived from one backup manifest.
/// Owned by restore planning and consumed by apply dry-run and runner workflows.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
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
    pub deployment_verification_checks: Vec<VerificationCheck>,
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
/// Read-only summary of in-place and remapped restore identities.
/// Owned by restore planning and embedded in `RestorePlan`.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
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
/// Read-only summary of snapshot metadata completeness.
/// Owned by restore planning and embedded in `RestorePlan`.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RestoreSnapshotSummary {
    pub all_members_have_module_hash: bool,
    pub all_members_have_code_version: bool,
    pub all_members_have_checksum: bool,
    pub members_with_module_hash: usize,
    pub members_with_code_version: usize,
    pub members_with_checksum: usize,
}

///
/// RestoreVerificationSummary
///
/// Read-only summary of restore verification work.
/// Owned by restore planning and embedded in `RestorePlan`.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RestoreVerificationSummary {
    pub verification_required: bool,
    pub all_members_have_checks: bool,
    pub deployment_checks: usize,
    pub member_check_groups: usize,
    pub member_checks: usize,
    pub members_with_checks: usize,
    pub total_checks: usize,
}

///
/// RestoreReadinessSummary
///
/// Read-only restore readiness projection with blocking reasons.
/// Owned by restore planning and embedded in `RestorePlan`.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RestoreReadinessSummary {
    pub ready: bool,
    pub reasons: Vec<String>,
}

///
/// RestoreOperationSummary
///
/// Read-only summary of concrete restore operations to schedule.
/// Owned by restore planning and embedded in `RestorePlan`.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RestoreOperationSummary {
    #[serde(default)]
    pub planned_canister_stops: usize,
    #[serde(default)]
    pub planned_canister_starts: usize,
    pub planned_snapshot_uploads: usize,
    pub planned_snapshot_loads: usize,
    pub planned_verification_checks: usize,
    pub planned_operations: usize,
}

///
/// RestoreOrderingSummary
///
/// Read-only summary of restore member ordering dependencies.
/// Owned by restore planning and embedded in `RestorePlan`.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RestoreOrderingSummary {
    pub ordered_members: usize,
    pub dependency_free_members: usize,
    pub parent_edges: usize,
}

///
/// RestorePlanMember
///
/// Planned restore row for one manifest member.
/// Owned by restore planning and consumed by apply dry-run and runner workflows.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
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
/// Parent-before-child dependency attached to one restore member.
/// Owned by restore planning and embedded in `RestorePlanMember`.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RestoreOrderingDependency {
    pub source_canister: String,
    pub target_canister: String,
    pub relationship: RestoreOrderingRelationship,
}

///
/// RestoreOrderingRelationship
///
/// Supported restore member ordering relationship.
/// Owned by restore planning and serialized in ordering dependency rows.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RestoreOrderingRelationship {
    ParentBeforeChild,
}
