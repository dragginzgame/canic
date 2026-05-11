use crate::manifest::IdentityMode;
use serde::{Deserialize, Serialize};

///
/// BackupPlan
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum BackupScopeKind {
    Member,
    Subtree,
    NonRootFleet,
    MaintenanceRoot,
}

///
/// BackupTarget
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BackupOperation {
    pub operation_id: String,
    pub order: u32,
    pub kind: BackupOperationKind,
    pub target_canister_id: Option<String>,
}

///
/// BackupOperationKind
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
/// BackupReceipt
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BackupReceipt {
    pub plan_id: String,
    pub operation_id: String,
    pub status: BackupReceiptStatus,
    pub target_canister_id: Option<String>,
    pub snapshot_id: Option<String>,
    pub message: Option<String>,
}

///
/// BackupReceiptStatus
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum BackupReceiptStatus {
    Completed,
    Failed,
    Skipped,
}

///
/// BackupExecutionPreflightReceipts
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
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
