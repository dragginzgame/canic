use crate::discovery::DiscoveryError;
use thiserror::Error as ThisError;

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
