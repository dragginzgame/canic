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
