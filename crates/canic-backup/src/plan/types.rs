//! Module: plan::types
//!
//! Responsibility: define serialized backup plan and preflight contracts.
//! Does not own: registry discovery, validation, execution, or persistence.
//! Boundary: data shapes shared by backup planners, runners, and preflights.

use crate::manifest::IdentityMode;

use serde::{Deserialize, Serialize};

///
/// BackupPlan
///
/// Executable backup plan derived from a selected deployment scope.
/// Owned by backup planning and consumed by execution journals and runners.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BackupPlan {
    pub plan_id: String,
    pub run_id: String,
    pub fleet: String,
    pub network: String,
    pub root_canister_id: String,
    pub selected_subtree_root: Option<String>,
    pub selected_scope_kind: BackupScopeKind,
    pub include_descendants: bool,
    pub root_included: bool,
    pub requires_root_controller: bool,
    pub snapshot_read_authority: SnapshotReadAuthority,
    pub quiescence_policy: QuiescencePolicy,
    pub topology_hash_before_quiesce: String,
    pub targets: Vec<BackupTarget>,
    pub phases: Vec<BackupOperation>,
}

///
/// BackupScopeKind
///
/// Backup selection mode used to derive target and operation scope.
/// Owned by backup planning and serialized into plan and preflight contracts.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum BackupScopeKind {
    Member,
    Subtree,
    NonRootDeployment,
    MaintenanceRoot,
}

///
/// BackupTarget
///
/// One canister selected for backup with authority and restore policy.
/// Owned by backup planning and used by preflight and execution builders.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BackupTarget {
    pub canister_id: String,
    pub role: Option<String>,
    pub parent_canister_id: Option<String>,
    pub depth: u32,
    pub control_authority: ControlAuthority,
    pub snapshot_read_authority: SnapshotReadAuthority,
    pub identity_mode: IdentityMode,
    pub expected_module_hash: Option<String>,
}

///
/// AuthorityEvidence
///
/// Confidence level for an authority decision embedded in a backup plan.
/// Owned by backup planning and refined by execution preflight receipts.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum AuthorityEvidence {
    Proven,
    Declared,
    Unknown,
}

///
/// ControlAuthority
///
/// Control authority decision for one selected backup target.
/// Owned by backup planning and validated before mutation execution.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ControlAuthority {
    pub source: ControlAuthoritySource,
    pub evidence: AuthorityEvidence,
}

impl ControlAuthority {
    #[must_use]
    pub const fn unknown() -> Self {
        Self {
            source: ControlAuthoritySource::Unknown,
            evidence: AuthorityEvidence::Unknown,
        }
    }

    #[must_use]
    pub const fn root_controller(evidence: AuthorityEvidence) -> Self {
        Self {
            source: ControlAuthoritySource::RootController,
            evidence,
        }
    }

    #[must_use]
    pub const fn operator_controller(evidence: AuthorityEvidence) -> Self {
        Self {
            source: ControlAuthoritySource::OperatorController,
            evidence,
        }
    }

    #[must_use]
    pub fn alternate_controller(
        controller: impl Into<String>,
        reason: impl Into<String>,
        evidence: AuthorityEvidence,
    ) -> Self {
        Self {
            source: ControlAuthoritySource::AlternateController {
                controller: controller.into(),
                reason: reason.into(),
            },
            evidence,
        }
    }

    #[must_use]
    pub fn is_proven(&self) -> bool {
        self.evidence == AuthorityEvidence::Proven && self.source != ControlAuthoritySource::Unknown
    }

    #[must_use]
    pub fn is_proven_root_controller(&self) -> bool {
        self.evidence == AuthorityEvidence::Proven
            && self.source == ControlAuthoritySource::RootController
    }
}

///
/// ControlAuthoritySource
///
/// Source of a control authority decision for one backup target.
/// Owned by backup planning and interpreted by preflight validation.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "kind")]
pub enum ControlAuthoritySource {
    Unknown,
    RootController,
    OperatorController,
    AlternateController { controller: String, reason: String },
}

///
/// SnapshotReadAuthority
///
/// Snapshot read authority decision for one selected backup target.
/// Owned by backup planning and validated before snapshot download.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SnapshotReadAuthority {
    pub source: SnapshotReadAuthoritySource,
    pub evidence: AuthorityEvidence,
}

impl SnapshotReadAuthority {
    #[must_use]
    pub const fn unknown() -> Self {
        Self {
            source: SnapshotReadAuthoritySource::Unknown,
            evidence: AuthorityEvidence::Unknown,
        }
    }

    #[must_use]
    pub const fn operator_controller(evidence: AuthorityEvidence) -> Self {
        Self {
            source: SnapshotReadAuthoritySource::OperatorController,
            evidence,
        }
    }

    #[must_use]
    pub const fn snapshot_visibility(evidence: AuthorityEvidence) -> Self {
        Self {
            source: SnapshotReadAuthoritySource::SnapshotVisibility,
            evidence,
        }
    }

    #[must_use]
    pub const fn root_configured_read(evidence: AuthorityEvidence) -> Self {
        Self {
            source: SnapshotReadAuthoritySource::RootConfiguredRead,
            evidence,
        }
    }

    #[must_use]
    pub const fn root_mediated_transfer(evidence: AuthorityEvidence) -> Self {
        Self {
            source: SnapshotReadAuthoritySource::RootMediatedTransfer,
            evidence,
        }
    }

    #[must_use]
    pub fn is_proven(&self) -> bool {
        self.evidence == AuthorityEvidence::Proven
            && self.source != SnapshotReadAuthoritySource::Unknown
    }
}

///
/// SnapshotReadAuthoritySource
///
/// Source of a snapshot-read authority decision for one backup target.
/// Owned by backup planning and interpreted by preflight validation.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SnapshotReadAuthoritySource {
    Unknown,
    OperatorController,
    SnapshotVisibility,
    RootConfiguredRead,
    RootMediatedTransfer,
}

///
/// QuiescencePolicy
///
/// Consistency policy requested before backup mutation begins.
/// Owned by backup planning and checked by execution preflight receipts.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum QuiescencePolicy {
    CrashConsistent,
    RootCoordinated,
    AppQuiesced,
}

///
/// BackupOperation
///
/// Ordered operation generated for one backup plan.
/// Owned by backup planning and converted into execution journal operations.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BackupOperation {
    pub operation_id: String,
    pub order: u32,
    pub kind: BackupOperationKind,
    pub target_canister_id: Option<String>,
}

///
/// BackupOperationKind
///
/// Operation class used by backup execution and preflight validation.
/// Owned by backup planning and interpreted by execution journals.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum BackupOperationKind {
    ValidateTopology,
    ValidateControlAuthority,
    ValidateSnapshotReadAuthority,
    ValidateQuiescencePolicy,
    Stop,
    CreateSnapshot,
    Start,
    DownloadSnapshot,
    VerifyArtifact,
    FinalizeManifest,
}

///
/// BackupExecutionPreflightReceipts
///
/// Complete preflight receipt bundle required before backup mutation.
/// Owned by backup planning and consumed by execution journals.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BackupExecutionPreflightReceipts {
    pub plan_id: String,
    pub preflight_id: String,
    pub validated_at: String,
    pub expires_at: String,
    pub topology: TopologyPreflightReceipt,
    pub control_authority: Vec<ControlAuthorityReceipt>,
    pub snapshot_read_authority: Vec<SnapshotReadAuthorityReceipt>,
    pub quiescence: QuiescencePreflightReceipt,
}

///
/// ControlAuthorityReceipt
///
/// Control authority preflight result for one selected backup target.
/// Owned by backup planning and applied before execution can mutate targets.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ControlAuthorityReceipt {
    pub plan_id: String,
    pub preflight_id: String,
    pub target_canister_id: String,
    pub authority: ControlAuthority,
    pub proof_source: AuthorityProofSource,
    pub validated_at: String,
    pub expires_at: String,
    pub message: Option<String>,
}

///
/// SnapshotReadAuthorityReceipt
///
/// Snapshot-read authority preflight result for one selected backup target.
/// Owned by backup planning and applied before snapshot download can run.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SnapshotReadAuthorityReceipt {
    pub plan_id: String,
    pub preflight_id: String,
    pub target_canister_id: String,
    pub authority: SnapshotReadAuthority,
    pub proof_source: AuthorityProofSource,
    pub validated_at: String,
    pub expires_at: String,
    pub message: Option<String>,
}

///
/// AuthorityProofSource
///
/// Evidence source used by an authority preflight receipt.
/// Owned by backup planning and serialized with authority receipt contracts.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum AuthorityProofSource {
    RootCoordination,
    ManagementStatus,
    SnapshotReadCheck,
    Declaration,
    Unknown,
}

///
/// ControlAuthorityPreflightRequest
///
/// Request shape for proving control authority over selected targets.
/// Owned by backup planning and sent to execution preflight providers.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ControlAuthorityPreflightRequest {
    pub plan_id: String,
    pub run_id: String,
    pub fleet: String,
    pub network: String,
    pub root_canister_id: String,
    pub requires_root_controller: bool,
    pub targets: Vec<ControlAuthorityPreflightTarget>,
}

///
/// ControlAuthorityPreflightTarget
///
/// Target row included in a control-authority preflight request.
/// Owned by backup planning and projected from selected backup targets.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ControlAuthorityPreflightTarget {
    pub canister_id: String,
    pub role: Option<String>,
    pub parent_canister_id: Option<String>,
    pub declared_authority: ControlAuthority,
}

impl From<&BackupTarget> for ControlAuthorityPreflightTarget {
    fn from(target: &BackupTarget) -> Self {
        Self {
            canister_id: target.canister_id.clone(),
            role: target.role.clone(),
            parent_canister_id: target.parent_canister_id.clone(),
            declared_authority: target.control_authority.clone(),
        }
    }
}

///
/// SnapshotReadAuthorityPreflightRequest
///
/// Request shape for proving snapshot read authority over selected targets.
/// Owned by backup planning and sent to execution preflight providers.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SnapshotReadAuthorityPreflightRequest {
    pub plan_id: String,
    pub run_id: String,
    pub fleet: String,
    pub network: String,
    pub root_canister_id: String,
    pub targets: Vec<SnapshotReadAuthorityPreflightTarget>,
}

///
/// SnapshotReadAuthorityPreflightTarget
///
/// Target row included in a snapshot-read preflight request.
/// Owned by backup planning and projected from selected backup targets.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SnapshotReadAuthorityPreflightTarget {
    pub canister_id: String,
    pub role: Option<String>,
    pub parent_canister_id: Option<String>,
    pub declared_authority: SnapshotReadAuthority,
}

impl From<&BackupTarget> for SnapshotReadAuthorityPreflightTarget {
    fn from(target: &BackupTarget) -> Self {
        Self {
            canister_id: target.canister_id.clone(),
            role: target.role.clone(),
            parent_canister_id: target.parent_canister_id.clone(),
            declared_authority: target.snapshot_read_authority.clone(),
        }
    }
}

///
/// TopologyPreflightRequest
///
/// Request shape for confirming selected topology before mutation.
/// Owned by backup planning and sent to execution preflight providers.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TopologyPreflightRequest {
    pub plan_id: String,
    pub run_id: String,
    pub fleet: String,
    pub network: String,
    pub root_canister_id: String,
    pub selected_subtree_root: Option<String>,
    pub selected_scope_kind: BackupScopeKind,
    pub topology_hash_before_quiesce: String,
    pub targets: Vec<TopologyPreflightTarget>,
}

///
/// TopologyPreflightTarget
///
/// Target row included in a topology preflight request and receipt.
/// Owned by backup planning and projected from selected backup targets.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TopologyPreflightTarget {
    pub canister_id: String,
    pub parent_canister_id: Option<String>,
    pub depth: u32,
}

impl From<&BackupTarget> for TopologyPreflightTarget {
    fn from(target: &BackupTarget) -> Self {
        Self {
            canister_id: target.canister_id.clone(),
            parent_canister_id: target.parent_canister_id.clone(),
            depth: target.depth,
        }
    }
}

///
/// TopologyPreflightReceipt
///
/// Topology preflight result accepted before backup mutation begins.
/// Owned by backup planning and checked against the selected plan.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TopologyPreflightReceipt {
    pub plan_id: String,
    pub preflight_id: String,
    pub topology_hash_before_quiesce: String,
    pub topology_hash_at_preflight: String,
    pub targets: Vec<TopologyPreflightTarget>,
    pub validated_at: String,
    pub expires_at: String,
    pub message: Option<String>,
}

///
/// QuiescencePreflightRequest
///
/// Request shape for confirming the selected quiescence policy.
/// Owned by backup planning and sent to execution preflight providers.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct QuiescencePreflightRequest {
    pub plan_id: String,
    pub run_id: String,
    pub fleet: String,
    pub network: String,
    pub root_canister_id: String,
    pub selected_subtree_root: Option<String>,
    pub quiescence_policy: QuiescencePolicy,
    pub targets: Vec<QuiescencePreflightTarget>,
}

///
/// QuiescencePreflightTarget
///
/// Target row included in a quiescence preflight request and receipt.
/// Owned by backup planning and projected from selected backup targets.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct QuiescencePreflightTarget {
    pub canister_id: String,
    pub role: Option<String>,
    pub parent_canister_id: Option<String>,
}

impl From<&BackupTarget> for QuiescencePreflightTarget {
    fn from(target: &BackupTarget) -> Self {
        Self {
            canister_id: target.canister_id.clone(),
            role: target.role.clone(),
            parent_canister_id: target.parent_canister_id.clone(),
        }
    }
}

///
/// QuiescencePreflightReceipt
///
/// Quiescence preflight result accepted before backup mutation begins.
/// Owned by backup planning and checked against the selected plan.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct QuiescencePreflightReceipt {
    pub plan_id: String,
    pub preflight_id: String,
    pub quiescence_policy: QuiescencePolicy,
    pub accepted: bool,
    pub targets: Vec<QuiescencePreflightTarget>,
    pub validated_at: String,
    pub expires_at: String,
    pub message: Option<String>,
}
