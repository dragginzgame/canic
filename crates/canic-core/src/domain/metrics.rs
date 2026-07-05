//! Module: domain::metrics
//!
//! Responsibility: define pure metric-family selector values shared by runtime
//! metric projection, workflow queries, and endpoint DTOs.
//! Does not own: metric row DTO structs, metric recording, or CLI metrics
//! transport models.
//! Boundary: DTOs re-export these values to preserve the public API path while
//! internal code imports them from the domain owner.

use candid::CandidType;
use serde::Deserialize;

///
/// MetricsKind
///
/// Metric tier selector.
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize)]
#[remain::sorted]
pub enum MetricsKind {
    Core,
    Placement,
    Platform,
    Runtime,
    Security,
    Storage,
}

///
/// CanisterOpsMetricOperation
///
/// Canister operation metric dimension used by public metrics projection.
///

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[remain::sorted]
pub enum CanisterOpsMetricOperation {
    Create,
    Delete,
    Install,
    Reinstall,
    Restore,
    Snapshot,
    Upgrade,
}

impl CanisterOpsMetricOperation {
    /// Return the stable public metrics label for this operation.
    #[must_use]
    pub const fn metric_label(self) -> &'static str {
        match self {
            Self::Create => "create",
            Self::Delete => "delete",
            Self::Install => "install",
            Self::Reinstall => "reinstall",
            Self::Restore => "restore",
            Self::Snapshot => "snapshot",
            Self::Upgrade => "upgrade",
        }
    }
}

///
/// CanisterOpsMetricOutcome
///
/// Canister operation outcome dimension used by public metrics projection.
///

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[remain::sorted]
pub enum CanisterOpsMetricOutcome {
    Completed,
    Failed,
    Skipped,
    Started,
}

impl CanisterOpsMetricOutcome {
    /// Return the stable public metrics label for this outcome.
    #[must_use]
    pub const fn metric_label(self) -> &'static str {
        match self {
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Skipped => "skipped",
            Self::Started => "started",
        }
    }
}

///
/// CanisterOpsMetricReason
///
/// Bounded canister operation reason dimension used by public metrics projection.
///

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[remain::sorted]
pub enum CanisterOpsMetricReason {
    AlreadyExists,
    Cycles,
    InvalidState,
    ManagementCall,
    MissingWasm,
    NewAllocation,
    NotFound,
    Ok,
    PolicyDenied,
    PoolReuse,
    PoolTopup,
    StatePropagation,
    Topology,
    TopologyPropagation,
    Unknown,
}

impl CanisterOpsMetricReason {
    /// Return the stable public metrics label for this reason.
    #[must_use]
    pub const fn metric_label(self) -> &'static str {
        match self {
            Self::AlreadyExists => "already_exists",
            Self::NewAllocation => "new_allocation",
            Self::Cycles => "cycles",
            Self::InvalidState => "invalid_state",
            Self::ManagementCall => "management_call",
            Self::MissingWasm => "missing_wasm",
            Self::NotFound => "not_found",
            Self::Ok => "ok",
            Self::PolicyDenied => "policy_denied",
            Self::PoolReuse => "pool_reuse",
            Self::PoolTopup => "pool_topup",
            Self::StatePropagation => "state_propagation",
            Self::Topology => "topology",
            Self::TopologyPropagation => "topology_propagation",
            Self::Unknown => "unknown",
        }
    }
}

///
/// ManagementCallMetricOperation
///
/// Management canister operation dimension used by runtime metrics recording.
///

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[remain::sorted]
pub enum ManagementCallMetricOperation {
    CanisterStatus,
    ClearChunkStore,
    CreateCanister,
    DeleteCanister,
    DepositCycles,
    EcdsaPublicKey,
    GetCycles,
    InstallChunkedCode,
    InstallCode,
    LoadCanisterSnapshot,
    RawRand,
    SignWithEcdsa,
    StopCanister,
    StoredChunks,
    TakeCanisterSnapshot,
    UninstallCode,
    UpdateSettings,
    UploadChunk,
}

///
/// ManagementCallMetricOutcome
///
/// Management canister outcome dimension used by runtime metrics recording.
///

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[remain::sorted]
pub enum ManagementCallMetricOutcome {
    Completed,
    Failed,
    Started,
}

///
/// ManagementCallMetricReason
///
/// Bounded management canister reason dimension used by runtime metrics recording.
///

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[remain::sorted]
pub enum ManagementCallMetricReason {
    Infra,
    Ok,
}

///
/// PlatformCallMetricSurface
///
/// Platform call surface dimension used by public metrics projection.
///

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[remain::sorted]
pub enum PlatformCallMetricSurface {
    Generic,
    Http,
    Ledger,
    Management,
}

impl PlatformCallMetricSurface {
    /// Return the stable public metrics label for this surface.
    #[must_use]
    pub const fn metric_label(self) -> &'static str {
        match self {
            Self::Generic => "generic",
            Self::Http => "http",
            Self::Ledger => "ledger",
            Self::Management => "management",
        }
    }
}

///
/// PlatformCallMetricMode
///
/// Platform call mode dimension used by public metrics projection.
///

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[remain::sorted]
pub enum PlatformCallMetricMode {
    BoundedWait,
    UnboundedWait,
    Update,
}

impl PlatformCallMetricMode {
    /// Return the stable public metrics label for this mode.
    #[must_use]
    pub const fn metric_label(self) -> &'static str {
        match self {
            Self::BoundedWait => "bounded_wait",
            Self::UnboundedWait => "unbounded_wait",
            Self::Update => "update",
        }
    }
}

///
/// PlatformCallMetricOutcome
///
/// Platform call outcome dimension used by public metrics projection.
///

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[remain::sorted]
pub enum PlatformCallMetricOutcome {
    Completed,
    Failed,
    Started,
}

impl PlatformCallMetricOutcome {
    /// Return the stable public metrics label for this outcome.
    #[must_use]
    pub const fn metric_label(self) -> &'static str {
        match self {
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Started => "started",
        }
    }
}

///
/// PlatformCallMetricReason
///
/// Bounded platform call reason dimension used by public metrics projection.
///

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[remain::sorted]
pub enum PlatformCallMetricReason {
    CandidDecode,
    CandidEncode,
    HttpStatus,
    Infra,
    LedgerRejected,
    Ok,
}

impl PlatformCallMetricReason {
    /// Return the stable public metrics label for this reason.
    #[must_use]
    pub const fn metric_label(self) -> &'static str {
        match self {
            Self::CandidDecode => "candid_decode",
            Self::CandidEncode => "candid_encode",
            Self::HttpStatus => "http_status",
            Self::Infra => "infra",
            Self::LedgerRejected => "ledger_rejected",
            Self::Ok => "ok",
        }
    }
}
