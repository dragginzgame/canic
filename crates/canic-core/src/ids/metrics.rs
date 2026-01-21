use candid::CandidType;
use serde::{Deserialize, Serialize};

///
/// AccessMetricKind
/// Enumerates the access predicate kind that rejected the call.
/// Access metrics are emitted only on denial.
/// Custom predicates report AccessMetricKind::Custom.
/// Predicate names are recorded separately alongside the kind.
///

#[derive(
    CandidType, Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize,
)]
#[remain::sorted]
pub enum AccessMetricKind {
    Auth,
    Custom,
    Env,
    Guard,
    Rule,
}

impl AccessMetricKind {
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
