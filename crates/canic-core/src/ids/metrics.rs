//! Module: ids::metrics
//!
//! Responsibility: metric classification identifiers.
//! Does not own: metric storage, aggregation, or emission.
//! Boundary: exposes stable categories used by access and system metrics.

use serde::{Deserialize, Serialize};

///
/// AccessMetricKind
///
/// Enumerates the access predicate kind that rejected the call.
/// Access metrics are emitted only on denial.
/// Custom predicates report AccessMetricKind::Custom.
/// Predicate names are recorded separately alongside the kind.
/// Owned by ids and consumed by access metrics adapters.
///

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[remain::sorted]
pub enum AccessMetricKind {
    Auth,
    Custom,
    Env,
    Guard,
    Rule,
}

impl AccessMetricKind {
    /// Return the stable metric label for this access metric kind.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Auth => "auth",
            Self::Custom => "custom",
            Self::Env => "env",
            Self::Guard => "guard",
            Self::Rule => "rule",
        }
    }
}

///
/// SystemMetricKind
///
/// Enumerates platform operation families recorded by system metrics.
/// Owned by ids and consumed by runtime metrics adapters.
///

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[remain::sorted]
pub enum SystemMetricKind {
    CanisterCall,
    CanisterStatus,
    CreateCanister,
    DeleteCanister,
    DepositCycles,
    HttpOutcall,
    InstallCode,
    RawRand,
    ReinstallCode,
    TimerScheduled,
    UninstallCode,
    UpdateSettings,
    UpgradeCode,
}
