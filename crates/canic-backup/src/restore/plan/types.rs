use crate::manifest::{IdentityMode, SourceSnapshot, VerificationCheck};
use serde::{Deserialize, Serialize};

///
/// RestoreMapping
///

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
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
