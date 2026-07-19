//! Module: plan::error
//!
//! Responsibility: report typed backup plan validation failures.
//! Does not own: plan construction, preflight validation, or execution state.
//! Boundary: shared error contract for plan builders and execution preflights.

use crate::discovery::DiscoveryError;

use thiserror::Error as ThisError;

///
/// BackupPlanError
///
/// Typed backup plan construction, validation, or preflight failure.
/// Owned by backup planning and returned before invalid execution can start.
///

#[derive(Debug, ThisError)]
pub enum BackupPlanError {
    #[error("unsupported backup plan version {0}")]
    UnsupportedVersion(u16),

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

    #[error("duplicate backup target {0}")]
    DuplicateTarget(String),

    #[error("backup plan has {actual} operations, expected {expected}")]
    OperationCountMismatch { expected: usize, actual: usize },

    #[error("backup plan operation {index} field {field} does not match its canonical projection")]
    OperationProjectionMismatch { index: usize, field: &'static str },

    #[error("normal backup scope must not include root")]
    RootIncludedWithoutMaintenance,

    #[error("maintenance root scope must include root")]
    MaintenanceRootExcludesRoot,

    #[error("selected scope root {0} is not present in plan targets")]
    SelectedRootNotInTargets(String),

    #[error("non-root-deployment scope must not declare a selected subtree root")]
    NonRootDeploymentHasSelectedRoot,

    #[error("backup target {canister_id} has a parent cycle")]
    TargetParentCycle { canister_id: String },

    #[error(
        "backup target {canister_id} has depth {actual}, expected {expected} below parent {parent_canister_id}"
    )]
    TargetDepthMismatch {
        canister_id: String,
        parent_canister_id: String,
        expected: u64,
        actual: u32,
    },

    #[error("backup target {canister_id} is disconnected from expected root {expected_root}")]
    TargetDisconnected {
        canister_id: String,
        expected_root: String,
    },

    #[error("selected backup root {selected_root} has selected parent {parent_canister_id}")]
    SelectedRootHasInternalParent {
        selected_root: String,
        parent_canister_id: String,
    },

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

    #[error("backup selector {0} did not match a live topology node")]
    UnknownSelector(String),

    #[error("backup selector {selector} matched multiple canisters: {matches:?}")]
    AmbiguousSelector {
        selector: String,
        matches: Vec<String>,
    },

    #[error(transparent)]
    Discovery(#[from] DiscoveryError),
}
