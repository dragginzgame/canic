//! Passive protocol classifications; runtime decisions remain with Core.

use candid::CandidType;
use serde::{Deserialize, Serialize};

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
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
pub enum ProvisioningRetryCategory {
    Backoff,
    ReviewRequired,
}
