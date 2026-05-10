mod build;

pub use build::{BackupPlanBuildInput, build_backup_plan, resolve_backup_selector};

use crate::{discovery::DiscoveryError, manifest::IdentityMode};
use candid::Principal;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, collections::BTreeSet, str::FromStr};
use thiserror::Error as ThisError;

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

impl BackupPlan {
    /// Validate the backup plan as a dry-run/planning artifact.
    pub fn validate(&self) -> Result<(), BackupPlanError> {
        validate_nonempty("plan_id", &self.plan_id)?;
        validate_nonempty("run_id", &self.run_id)?;
        validate_nonempty("fleet", &self.fleet)?;
        validate_nonempty("network", &self.network)?;
        validate_principal("root_canister_id", &self.root_canister_id)?;
        validate_optional_principal(
            "selected_subtree_root",
            self.selected_subtree_root.as_deref(),
        )?;
        validate_nonempty(
            "topology_hash_before_quiesce",
            &self.topology_hash_before_quiesce,
        )?;
        validate_root_scope(self)?;
        validate_targets(self)?;
        validate_selected_scope(self)?;
        validate_phase_order(&self.phases)
    }

    /// Validate the backup plan before any live mutation can run.
    pub fn validate_for_execution(&self) -> Result<(), BackupPlanError> {
        self.validate()?;

        for target in &self.targets {
            if !target.control_authority.is_proven() {
                return Err(BackupPlanError::UnprovenControlAuthority(
                    target.canister_id.clone(),
                ));
            }
            if !target.snapshot_read_authority.is_proven() {
                return Err(BackupPlanError::UnprovenTargetSnapshotReadAuthority(
                    target.canister_id.clone(),
                ));
            }
            if self.requires_root_controller
                && target.canister_id != self.root_canister_id
                && !target.control_authority.is_proven_root_controller()
            {
                return Err(BackupPlanError::MissingRootController(
                    target.canister_id.clone(),
                ));
            }
        }

        Ok(())
    }

    /// Validate execution-only preflight receipts before mutation starts.
    pub fn validate_execution_preflight_receipts(
        &self,
        topology_receipt: &TopologyPreflightReceipt,
        quiescence_receipt: &QuiescencePreflightReceipt,
        preflight_id: &str,
        as_of: &str,
    ) -> Result<(), BackupPlanError> {
        self.validate_for_execution()?;
        validate_preflight_id(preflight_id)?;
        validate_preflight_timestamp("preflight.as_of", as_of)?;
        validate_topology_preflight_receipt(self, topology_receipt, preflight_id, as_of)?;
        validate_quiescence_preflight_receipt(self, quiescence_receipt, preflight_id, as_of)
    }

    /// Apply and validate the full execution preflight receipt bundle.
    pub fn apply_execution_preflight_receipts(
        &mut self,
        receipts: &BackupExecutionPreflightReceipts,
        as_of: &str,
    ) -> Result<(), BackupPlanError> {
        validate_execution_preflight_bundle(self, receipts, as_of)?;
        self.apply_authority_preflight_receipts(
            &receipts.preflight_id,
            &receipts.control_authority,
            &receipts.snapshot_read_authority,
            as_of,
        )?;
        self.validate_execution_preflight_receipts(
            &receipts.topology,
            &receipts.quiescence,
            &receipts.preflight_id,
            as_of,
        )
    }

    /// Apply proven authority receipts produced by execution preflights.
    pub fn apply_authority_preflight_receipts(
        &mut self,
        preflight_id: &str,
        control_receipts: &[ControlAuthorityReceipt],
        snapshot_read_receipts: &[SnapshotReadAuthorityReceipt],
        as_of: &str,
    ) -> Result<(), BackupPlanError> {
        self.apply_control_authority_receipts(preflight_id, control_receipts, as_of)?;
        self.apply_snapshot_read_authority_receipts(preflight_id, snapshot_read_receipts, as_of)
    }

    /// Apply proven control authority receipts for every selected target.
    pub fn apply_control_authority_receipts(
        &mut self,
        preflight_id: &str,
        receipts: &[ControlAuthorityReceipt],
        as_of: &str,
    ) -> Result<(), BackupPlanError> {
        let mut receipts =
            control_receipt_map(&self.plan_id, preflight_id, as_of, &self.targets, receipts)?;
        let mut updates = Vec::with_capacity(self.targets.len());
        for target in &self.targets {
            let receipt = receipts.remove(&target.canister_id).ok_or_else(|| {
                BackupPlanError::MissingControlAuthorityReceipt(target.canister_id.clone())
            })?;
            if !receipt.authority.is_proven() {
                return Err(BackupPlanError::UnprovenControlAuthority(
                    target.canister_id.clone(),
                ));
            }
            if self.requires_root_controller
                && target.canister_id != self.root_canister_id
                && !receipt.authority.is_proven_root_controller()
            {
                return Err(BackupPlanError::MissingRootController(
                    target.canister_id.clone(),
                ));
            }
            updates.push((target.canister_id.clone(), receipt.authority));
        }

        for (target_id, authority) in updates {
            let target = self
                .targets
                .iter_mut()
                .find(|target| target.canister_id == target_id)
                .expect("validated update target should exist");
            target.control_authority = authority;
        }
        Ok(())
    }

    /// Apply proven snapshot read authority receipts for every selected target.
    pub fn apply_snapshot_read_authority_receipts(
        &mut self,
        preflight_id: &str,
        receipts: &[SnapshotReadAuthorityReceipt],
        as_of: &str,
    ) -> Result<(), BackupPlanError> {
        let mut receipts =
            snapshot_read_receipt_map(&self.plan_id, preflight_id, as_of, &self.targets, receipts)?;
        let mut updates = Vec::with_capacity(self.targets.len());
        for target in &self.targets {
            let receipt = receipts.remove(&target.canister_id).ok_or_else(|| {
                BackupPlanError::MissingSnapshotReadAuthorityReceipt(target.canister_id.clone())
            })?;
            if !receipt.authority.is_proven() {
                return Err(BackupPlanError::UnprovenTargetSnapshotReadAuthority(
                    target.canister_id.clone(),
                ));
            }
            updates.push((target.canister_id.clone(), receipt.authority));
        }

        for (target_id, authority) in updates {
            let target = self
                .targets
                .iter_mut()
                .find(|target| target.canister_id == target_id)
                .expect("validated update target should exist");
            target.snapshot_read_authority = authority;
        }
        Ok(())
    }

    /// Build the typed control-authority preflight request for this plan.
    #[must_use]
    pub fn control_authority_preflight_request(&self) -> ControlAuthorityPreflightRequest {
        ControlAuthorityPreflightRequest {
            plan_id: self.plan_id.clone(),
            run_id: self.run_id.clone(),
            fleet: self.fleet.clone(),
            network: self.network.clone(),
            root_canister_id: self.root_canister_id.clone(),
            requires_root_controller: self.requires_root_controller,
            targets: self
                .targets
                .iter()
                .map(ControlAuthorityPreflightTarget::from)
                .collect(),
        }
    }

    /// Build the typed snapshot-read preflight request for this plan.
    #[must_use]
    pub fn snapshot_read_authority_preflight_request(
        &self,
    ) -> SnapshotReadAuthorityPreflightRequest {
        SnapshotReadAuthorityPreflightRequest {
            plan_id: self.plan_id.clone(),
            run_id: self.run_id.clone(),
            fleet: self.fleet.clone(),
            network: self.network.clone(),
            root_canister_id: self.root_canister_id.clone(),
            targets: self
                .targets
                .iter()
                .map(SnapshotReadAuthorityPreflightTarget::from)
                .collect(),
        }
    }

    /// Build the typed topology preflight request for this plan.
    #[must_use]
    pub fn topology_preflight_request(&self) -> TopologyPreflightRequest {
        TopologyPreflightRequest {
            plan_id: self.plan_id.clone(),
            run_id: self.run_id.clone(),
            fleet: self.fleet.clone(),
            network: self.network.clone(),
            root_canister_id: self.root_canister_id.clone(),
            selected_subtree_root: self.selected_subtree_root.clone(),
            selected_scope_kind: self.selected_scope_kind.clone(),
            topology_hash_before_quiesce: self.topology_hash_before_quiesce.clone(),
            targets: self
                .targets
                .iter()
                .map(TopologyPreflightTarget::from)
                .collect(),
        }
    }

    /// Build the typed quiescence preflight request for this plan.
    #[must_use]
    pub fn quiescence_preflight_request(&self) -> QuiescencePreflightRequest {
        QuiescencePreflightRequest {
            plan_id: self.plan_id.clone(),
            run_id: self.run_id.clone(),
            fleet: self.fleet.clone(),
            network: self.network.clone(),
            root_canister_id: self.root_canister_id.clone(),
            selected_subtree_root: self.selected_subtree_root.clone(),
            quiescence_policy: self.quiescence_policy.clone(),
            targets: self
                .targets
                .iter()
                .map(QuiescencePreflightTarget::from)
                .collect(),
        }
    }
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

///
/// BackupPlanError
///

#[derive(Debug, ThisError)]
pub enum BackupPlanError {
    #[error("field {0} must not be empty")]
    EmptyField(&'static str),

    #[error("field {field} must be a valid principal: {value}")]
    InvalidPrincipal { field: &'static str, value: String },

    #[error("field {field} must be a 64-character hex topology hash: {value}")]
    InvalidTopologyHash { field: &'static str, value: String },

    #[error("field {field} must be a unix timestamp marker: {value}")]
    InvalidTimestamp { field: &'static str, value: String },

    #[error("backup plan has no targets")]
    EmptyTargets,

    #[error("backup plan has no phases")]
    EmptyPhases,

    #[error("duplicate backup target {0}")]
    DuplicateTarget(String),

    #[error("duplicate backup operation id {0}")]
    DuplicateOperationId(String),

    #[error("operation {operation_id} has order {order}, expected {expected}")]
    OperationOrderMismatch {
        operation_id: String,
        order: u32,
        expected: u32,
    },

    #[error("normal backup scope must not include root")]
    RootIncludedWithoutMaintenance,

    #[error("maintenance root scope must include root")]
    MaintenanceRootExcludesRoot,

    #[error("selected scope root {0} is not present in plan targets")]
    SelectedRootNotInTargets(String),

    #[error("non-root-fleet scope must not declare a selected subtree root")]
    NonRootFleetHasSelectedRoot,

    #[error("target {0} has no proven control authority")]
    UnprovenControlAuthority(String),

    #[error("target {0} has no proven snapshot read authority")]
    UnprovenTargetSnapshotReadAuthority(String),

    #[error("target {0} must be controllable by root for this plan")]
    MissingRootController(String),

    #[error("target {0} has no control authority receipt")]
    MissingControlAuthorityReceipt(String),

    #[error("target {0} has no snapshot read authority receipt")]
    MissingSnapshotReadAuthorityReceipt(String),

    #[error("authority receipt targets unknown canister {0}")]
    UnknownAuthorityReceiptTarget(String),

    #[error("duplicate authority receipt for target {0}")]
    DuplicateAuthorityReceipt(String),

    #[error("authority receipt plan id {actual} does not match plan {expected}")]
    AuthorityReceiptPlanMismatch { expected: String, actual: String },

    #[error("authority receipt preflight id {actual} does not match preflight {expected}")]
    AuthorityReceiptPreflightMismatch { expected: String, actual: String },

    #[error("preflight receipt plan id {actual} does not match plan {expected}")]
    PreflightReceiptPlanMismatch { expected: String, actual: String },

    #[error("preflight receipt id {actual} does not match preflight {expected}")]
    PreflightReceiptIdMismatch { expected: String, actual: String },

    #[error(
        "preflight receipt {preflight_id} is not valid yet at {as_of}; validated at {validated_at}"
    )]
    PreflightReceiptNotYetValid {
        preflight_id: String,
        validated_at: String,
        as_of: String,
    },

    #[error("preflight receipt {preflight_id} expired at {expires_at}; checked at {as_of}")]
    PreflightReceiptExpired {
        preflight_id: String,
        expires_at: String,
        as_of: String,
    },

    #[error("preflight receipt {preflight_id} has invalid validity window")]
    PreflightReceiptInvalidWindow { preflight_id: String },

    #[error("topology preflight hash drifted from {expected} to {actual}")]
    TopologyPreflightHashMismatch { expected: String, actual: String },

    #[error("topology preflight targets do not match selected plan targets")]
    TopologyPreflightTargetsMismatch,

    #[error("quiescence preflight policy does not match plan")]
    QuiescencePolicyMismatch,

    #[error("quiescence preflight was not accepted")]
    QuiescencePreflightRejected,

    #[error("quiescence preflight targets do not match selected plan targets")]
    QuiescencePreflightTargetsMismatch,

    #[error("operation {operation_id} targets unknown canister {target_canister_id}")]
    UnknownOperationTarget {
        operation_id: String,
        target_canister_id: String,
    },

    #[error("backup selector {0} did not match a live topology node")]
    UnknownSelector(String),

    #[error("backup selector {selector} matched multiple canisters: {matches:?}")]
    AmbiguousSelector {
        selector: String,
        matches: Vec<String>,
    },

    #[error("required preflight operation {0} is missing")]
    MissingPreflight(&'static str),

    #[error("mutating operation {operation_id} appears before required preflights")]
    MutationBeforePreflight { operation_id: String },

    #[error(transparent)]
    Discovery(#[from] DiscoveryError),
}

fn validate_root_scope(plan: &BackupPlan) -> Result<(), BackupPlanError> {
    if plan.selected_scope_kind == BackupScopeKind::MaintenanceRoot {
        if plan.root_included {
            return Ok(());
        }
        return Err(BackupPlanError::MaintenanceRootExcludesRoot);
    }

    if plan.root_included {
        return Err(BackupPlanError::RootIncludedWithoutMaintenance);
    }

    Ok(())
}

fn validate_targets(plan: &BackupPlan) -> Result<(), BackupPlanError> {
    if plan.targets.is_empty() {
        return Err(BackupPlanError::EmptyTargets);
    }

    let mut target_ids = BTreeSet::new();
    for target in &plan.targets {
        validate_principal("targets[].canister_id", &target.canister_id)?;
        validate_optional_principal(
            "targets[].parent_canister_id",
            target.parent_canister_id.as_deref(),
        )?;
        validate_optional_nonempty("targets[].role", target.role.as_deref())?;
        validate_optional_nonempty(
            "targets[].expected_module_hash",
            target.expected_module_hash.as_deref(),
        )?;
        validate_control_authority(&target.control_authority)?;

        if !target_ids.insert(target.canister_id.clone()) {
            return Err(BackupPlanError::DuplicateTarget(target.canister_id.clone()));
        }
        if !plan.root_included && target.canister_id == plan.root_canister_id {
            return Err(BackupPlanError::RootIncludedWithoutMaintenance);
        }
    }

    validate_operation_targets(&plan.phases, &target_ids)
}

fn validate_control_authority(authority: &ControlAuthority) -> Result<(), BackupPlanError> {
    match &authority.source {
        ControlAuthoritySource::Unknown
        | ControlAuthoritySource::RootController
        | ControlAuthoritySource::OperatorController => Ok(()),
        ControlAuthoritySource::AlternateController { controller, reason } => {
            validate_principal("targets[].control_authority.controller", controller)?;
            validate_nonempty("targets[].control_authority.reason", reason)
        }
    }
}

fn control_receipt_map(
    plan_id: &str,
    preflight_id: &str,
    as_of: &str,
    targets: &[BackupTarget],
    receipts: &[ControlAuthorityReceipt],
) -> Result<BTreeMap<String, ControlAuthorityReceipt>, BackupPlanError> {
    let target_ids = targets
        .iter()
        .map(|target| target.canister_id.as_str())
        .collect::<BTreeSet<_>>();
    let mut receipt_map = BTreeMap::new();

    for receipt in receipts {
        validate_authority_receipt_header(AuthorityReceiptHeaderInput {
            expected_plan_id: plan_id,
            expected_preflight_id: preflight_id,
            as_of,
            target_ids: &target_ids,
            actual_plan_id: &receipt.plan_id,
            actual_preflight_id: &receipt.preflight_id,
            target_canister_id: &receipt.target_canister_id,
            validated_at: &receipt.validated_at,
            expires_at: &receipt.expires_at,
            message: receipt.message.as_deref(),
        })?;
        validate_control_authority(&receipt.authority)?;
        if receipt_map
            .insert(receipt.target_canister_id.clone(), receipt.clone())
            .is_some()
        {
            return Err(BackupPlanError::DuplicateAuthorityReceipt(
                receipt.target_canister_id.clone(),
            ));
        }
    }

    Ok(receipt_map)
}

fn snapshot_read_receipt_map(
    plan_id: &str,
    preflight_id: &str,
    as_of: &str,
    targets: &[BackupTarget],
    receipts: &[SnapshotReadAuthorityReceipt],
) -> Result<BTreeMap<String, SnapshotReadAuthorityReceipt>, BackupPlanError> {
    let target_ids = targets
        .iter()
        .map(|target| target.canister_id.as_str())
        .collect::<BTreeSet<_>>();
    let mut receipt_map = BTreeMap::new();

    for receipt in receipts {
        validate_authority_receipt_header(AuthorityReceiptHeaderInput {
            expected_plan_id: plan_id,
            expected_preflight_id: preflight_id,
            as_of,
            target_ids: &target_ids,
            actual_plan_id: &receipt.plan_id,
            actual_preflight_id: &receipt.preflight_id,
            target_canister_id: &receipt.target_canister_id,
            validated_at: &receipt.validated_at,
            expires_at: &receipt.expires_at,
            message: receipt.message.as_deref(),
        })?;
        if receipt_map
            .insert(receipt.target_canister_id.clone(), receipt.clone())
            .is_some()
        {
            return Err(BackupPlanError::DuplicateAuthorityReceipt(
                receipt.target_canister_id.clone(),
            ));
        }
    }

    Ok(receipt_map)
}

struct AuthorityReceiptHeaderInput<'a> {
    expected_plan_id: &'a str,
    expected_preflight_id: &'a str,
    as_of: &'a str,
    target_ids: &'a BTreeSet<&'a str>,
    actual_plan_id: &'a str,
    actual_preflight_id: &'a str,
    target_canister_id: &'a str,
    validated_at: &'a str,
    expires_at: &'a str,
    message: Option<&'a str>,
}

fn validate_authority_receipt_header(
    input: AuthorityReceiptHeaderInput<'_>,
) -> Result<(), BackupPlanError> {
    validate_nonempty("authority_receipts[].plan_id", input.actual_plan_id)?;
    validate_preflight_id(input.actual_preflight_id)?;
    validate_principal(
        "authority_receipts[].target_canister_id",
        input.target_canister_id,
    )?;
    validate_optional_nonempty("authority_receipts[].message", input.message)?;
    validate_preflight_window(
        input.actual_preflight_id,
        input.validated_at,
        input.expires_at,
        input.as_of,
    )?;

    if input.actual_plan_id != input.expected_plan_id {
        return Err(BackupPlanError::AuthorityReceiptPlanMismatch {
            expected: input.expected_plan_id.to_string(),
            actual: input.actual_plan_id.to_string(),
        });
    }
    if input.actual_preflight_id != input.expected_preflight_id {
        return Err(BackupPlanError::AuthorityReceiptPreflightMismatch {
            expected: input.expected_preflight_id.to_string(),
            actual: input.actual_preflight_id.to_string(),
        });
    }
    if !input.target_ids.contains(input.target_canister_id) {
        return Err(BackupPlanError::UnknownAuthorityReceiptTarget(
            input.target_canister_id.to_string(),
        ));
    }

    Ok(())
}

fn validate_execution_preflight_bundle(
    plan: &BackupPlan,
    receipts: &BackupExecutionPreflightReceipts,
    as_of: &str,
) -> Result<(), BackupPlanError> {
    validate_nonempty("preflight_receipts.plan_id", &receipts.plan_id)?;
    validate_preflight_id(&receipts.preflight_id)?;
    validate_preflight_timestamp("preflight_receipts.as_of", as_of)?;
    validate_preflight_window(
        &receipts.preflight_id,
        &receipts.validated_at,
        &receipts.expires_at,
        as_of,
    )?;

    if receipts.plan_id != plan.plan_id {
        return Err(BackupPlanError::PreflightReceiptPlanMismatch {
            expected: plan.plan_id.clone(),
            actual: receipts.plan_id.clone(),
        });
    }

    Ok(())
}

fn validate_topology_preflight_receipt(
    plan: &BackupPlan,
    receipt: &TopologyPreflightReceipt,
    preflight_id: &str,
    as_of: &str,
) -> Result<(), BackupPlanError> {
    validate_nonempty("topology_receipt.plan_id", &receipt.plan_id)?;
    validate_preflight_id(&receipt.preflight_id)?;
    validate_required_hash(
        "topology_receipt.topology_hash_before_quiesce",
        &receipt.topology_hash_before_quiesce,
    )?;
    validate_required_hash(
        "topology_receipt.topology_hash_at_preflight",
        &receipt.topology_hash_at_preflight,
    )?;
    validate_optional_nonempty("topology_receipt.message", receipt.message.as_deref())?;
    validate_preflight_window(
        &receipt.preflight_id,
        &receipt.validated_at,
        &receipt.expires_at,
        as_of,
    )?;

    if receipt.plan_id != plan.plan_id {
        return Err(BackupPlanError::PreflightReceiptPlanMismatch {
            expected: plan.plan_id.clone(),
            actual: receipt.plan_id.clone(),
        });
    }
    if receipt.preflight_id != preflight_id {
        return Err(BackupPlanError::PreflightReceiptIdMismatch {
            expected: preflight_id.to_string(),
            actual: receipt.preflight_id.clone(),
        });
    }
    if receipt.topology_hash_before_quiesce != plan.topology_hash_before_quiesce {
        return Err(BackupPlanError::TopologyPreflightHashMismatch {
            expected: plan.topology_hash_before_quiesce.clone(),
            actual: receipt.topology_hash_before_quiesce.clone(),
        });
    }
    if receipt.topology_hash_at_preflight != plan.topology_hash_before_quiesce {
        return Err(BackupPlanError::TopologyPreflightHashMismatch {
            expected: plan.topology_hash_before_quiesce.clone(),
            actual: receipt.topology_hash_at_preflight.clone(),
        });
    }
    if receipt.targets != plan.topology_preflight_request().targets {
        return Err(BackupPlanError::TopologyPreflightTargetsMismatch);
    }

    Ok(())
}

fn validate_quiescence_preflight_receipt(
    plan: &BackupPlan,
    receipt: &QuiescencePreflightReceipt,
    preflight_id: &str,
    as_of: &str,
) -> Result<(), BackupPlanError> {
    validate_nonempty("quiescence_receipt.plan_id", &receipt.plan_id)?;
    validate_preflight_id(&receipt.preflight_id)?;
    validate_optional_nonempty("quiescence_receipt.message", receipt.message.as_deref())?;
    validate_preflight_window(
        &receipt.preflight_id,
        &receipt.validated_at,
        &receipt.expires_at,
        as_of,
    )?;

    if receipt.plan_id != plan.plan_id {
        return Err(BackupPlanError::PreflightReceiptPlanMismatch {
            expected: plan.plan_id.clone(),
            actual: receipt.plan_id.clone(),
        });
    }
    if receipt.preflight_id != preflight_id {
        return Err(BackupPlanError::PreflightReceiptIdMismatch {
            expected: preflight_id.to_string(),
            actual: receipt.preflight_id.clone(),
        });
    }
    if receipt.quiescence_policy != plan.quiescence_policy {
        return Err(BackupPlanError::QuiescencePolicyMismatch);
    }
    if !receipt.accepted {
        return Err(BackupPlanError::QuiescencePreflightRejected);
    }
    if receipt.targets != plan.quiescence_preflight_request().targets {
        return Err(BackupPlanError::QuiescencePreflightTargetsMismatch);
    }

    Ok(())
}

fn validate_selected_scope(plan: &BackupPlan) -> Result<(), BackupPlanError> {
    match plan.selected_scope_kind {
        BackupScopeKind::NonRootFleet => {
            if plan.selected_subtree_root.is_some() {
                return Err(BackupPlanError::NonRootFleetHasSelectedRoot);
            }
            Ok(())
        }
        BackupScopeKind::Member | BackupScopeKind::Subtree | BackupScopeKind::MaintenanceRoot => {
            let Some(selected_root) = &plan.selected_subtree_root else {
                return Err(BackupPlanError::EmptyField("selected_subtree_root"));
            };
            if plan
                .targets
                .iter()
                .any(|target| &target.canister_id == selected_root)
            {
                Ok(())
            } else {
                Err(BackupPlanError::SelectedRootNotInTargets(
                    selected_root.clone(),
                ))
            }
        }
    }
}

fn validate_operation_targets(
    phases: &[BackupOperation],
    target_ids: &BTreeSet<String>,
) -> Result<(), BackupPlanError> {
    if phases.is_empty() {
        return Err(BackupPlanError::EmptyPhases);
    }

    let mut operation_ids = BTreeSet::new();
    for (index, phase) in phases.iter().enumerate() {
        validate_nonempty("phases[].operation_id", &phase.operation_id)?;
        let expected = u32::try_from(index).unwrap_or(u32::MAX);
        if phase.order != expected {
            return Err(BackupPlanError::OperationOrderMismatch {
                operation_id: phase.operation_id.clone(),
                order: phase.order,
                expected,
            });
        }
        if !operation_ids.insert(phase.operation_id.clone()) {
            return Err(BackupPlanError::DuplicateOperationId(
                phase.operation_id.clone(),
            ));
        }
        if let Some(target) = &phase.target_canister_id {
            validate_principal("phases[].target_canister_id", target)?;
            if !target_ids.contains(target) {
                return Err(BackupPlanError::UnknownOperationTarget {
                    operation_id: phase.operation_id.clone(),
                    target_canister_id: target.clone(),
                });
            }
        }
    }

    Ok(())
}

fn validate_phase_order(phases: &[BackupOperation]) -> Result<(), BackupPlanError> {
    let topology = preflight_position(phases, BackupOperationKind::ValidateTopology, "topology")?;
    let control = preflight_position(
        phases,
        BackupOperationKind::ValidateControlAuthority,
        "control_authority",
    )?;
    let read = preflight_position(
        phases,
        BackupOperationKind::ValidateSnapshotReadAuthority,
        "snapshot_read_authority",
    )?;
    let quiescence = preflight_position(
        phases,
        BackupOperationKind::ValidateQuiescencePolicy,
        "quiescence_policy",
    )?;
    let preflight_cutoff = [topology, control, read, quiescence]
        .into_iter()
        .max()
        .expect("non-empty preflight positions");

    for (index, phase) in phases.iter().enumerate() {
        if index < preflight_cutoff && phase.kind.is_mutating() {
            return Err(BackupPlanError::MutationBeforePreflight {
                operation_id: phase.operation_id.clone(),
            });
        }
    }

    Ok(())
}

fn preflight_position(
    phases: &[BackupOperation],
    kind: BackupOperationKind,
    label: &'static str,
) -> Result<usize, BackupPlanError> {
    phases
        .iter()
        .position(|phase| phase.kind == kind)
        .ok_or(BackupPlanError::MissingPreflight(label))
}

impl BackupOperationKind {
    const fn is_mutating(&self) -> bool {
        matches!(
            self,
            Self::Stop | Self::CreateSnapshot | Self::Start | Self::DownloadSnapshot
        )
    }
}

fn validate_nonempty(field: &'static str, value: &str) -> Result<(), BackupPlanError> {
    if value.trim().is_empty() {
        Err(BackupPlanError::EmptyField(field))
    } else {
        Ok(())
    }
}

fn validate_optional_nonempty(
    field: &'static str,
    value: Option<&str>,
) -> Result<(), BackupPlanError> {
    match value {
        Some(value) => validate_nonempty(field, value),
        None => Ok(()),
    }
}

fn validate_principal(field: &'static str, value: &str) -> Result<(), BackupPlanError> {
    Principal::from_str(value)
        .map(|_| ())
        .map_err(|_| BackupPlanError::InvalidPrincipal {
            field,
            value: value.to_string(),
        })
}

fn validate_required_hash(field: &'static str, value: &str) -> Result<(), BackupPlanError> {
    validate_nonempty(field, value)?;
    if value.len() == 64 && value.chars().all(|char| char.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err(BackupPlanError::InvalidTopologyHash {
            field,
            value: value.to_string(),
        })
    }
}

fn validate_preflight_id(value: &str) -> Result<(), BackupPlanError> {
    validate_nonempty("preflight_id", value)
}

fn validate_preflight_window(
    preflight_id: &str,
    validated_at: &str,
    expires_at: &str,
    as_of: &str,
) -> Result<(), BackupPlanError> {
    let validated_at_seconds =
        validate_preflight_timestamp("preflight_receipts[].validated_at", validated_at)?;
    let expires_at_seconds =
        validate_preflight_timestamp("preflight_receipts[].expires_at", expires_at)?;
    let as_of_seconds = validate_preflight_timestamp("preflight_receipts.as_of", as_of)?;

    if validated_at_seconds >= expires_at_seconds {
        return Err(BackupPlanError::PreflightReceiptInvalidWindow {
            preflight_id: preflight_id.to_string(),
        });
    }
    if as_of_seconds < validated_at_seconds {
        return Err(BackupPlanError::PreflightReceiptNotYetValid {
            preflight_id: preflight_id.to_string(),
            validated_at: validated_at.to_string(),
            as_of: as_of.to_string(),
        });
    }
    if as_of_seconds >= expires_at_seconds {
        return Err(BackupPlanError::PreflightReceiptExpired {
            preflight_id: preflight_id.to_string(),
            expires_at: expires_at.to_string(),
            as_of: as_of.to_string(),
        });
    }

    Ok(())
}

fn validate_preflight_timestamp(field: &'static str, value: &str) -> Result<u64, BackupPlanError> {
    validate_nonempty(field, value)?;
    value
        .strip_prefix("unix:")
        .and_then(|seconds| seconds.parse::<u64>().ok())
        .ok_or_else(|| BackupPlanError::InvalidTimestamp {
            field,
            value: value.to_string(),
        })
}

fn validate_optional_principal(
    field: &'static str,
    value: Option<&str>,
) -> Result<(), BackupPlanError> {
    match value {
        Some(value) => validate_principal(field, value),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests;
