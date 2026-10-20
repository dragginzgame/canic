//! Passive protocol classifications; runtime decisions remain with Core.

use ic_platform::candid::contract::CandidType;
use serde::{Deserialize, Serialize};

/// Maximum bytes in one exact dependency method label.
pub const MAX_PROVISIONING_DEPENDENCY_METHOD_BYTES: usize = 128;
/// Maximum bytes in one provisioning substep label.
pub const MAX_PROVISIONING_DEPENDENCY_STEP_BYTES: usize = 64;
/// Maximum UTF-8 bytes retained from a protected platform rejection.
pub const MAX_PROVISIONING_REJECTION_BYTES: usize = 1024;

/// Protected dependency evidence, independent of public diagnostics and spending authority.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[candid_path("ic_platform::candid::contract")]
#[serde(deny_unknown_fields)]
pub struct ProvisioningDependencyFailure {
    pub target: ic_platform::principal::ExternalPrincipal,
    pub method: String,
    pub step: String,
    #[serde(deserialize_with = "crate::serialization::required_option")]
    pub rejection_code: Option<u32>,
    #[serde(deserialize_with = "crate::serialization::required_option")]
    pub rejection_detail: Option<String>,
}

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[candid_path("ic_platform::candid::contract")]
pub enum ProvisioningFailureStage {
    Provisioning,
    CoordinatorStatus,
    ComponentOrigin,
    ComponentAllocation,
    ComponentChildAllocation,
    ComponentRuntime,
    ComponentMembership,
    ComponentCommit,
    RootPreparation,
    RootActivation,
    StoreCatalog,
    StoreStatus,
    StoreIdentity,
    StoreCredential,
    StoreActivation,
}

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[candid_path("ic_platform::candid::contract")]
pub enum ProvisioningRetryCategory {
    Backoff,
    ReviewRequired,
}
