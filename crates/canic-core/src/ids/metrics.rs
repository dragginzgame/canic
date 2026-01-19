use candid::CandidType;
use serde::{Deserialize, Serialize};

///
/// AccessMetricKind
/// Enumerates the access-control stage that rejected the call.
/// Access metrics are emitted only on denial.
///

#[derive(
    CandidType, Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize,
)]
#[remain::sorted]
pub enum AccessMetricKind {
    Auth,
    Env,
    Guard,
    Rule,
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
