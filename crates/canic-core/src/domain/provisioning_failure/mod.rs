//! Module: domain::provisioning_failure
//!
//! Responsibility: name bounded provisioning failures and decide active retry delays.
//! Does not own: timers, storage, observations, or effect reconciliation.
//! Boundary: only proved authority failures suspend immediately; unknown failures back off.

use candid::CandidType;
use serde::{Deserialize, Serialize};

/// Exact owner stage of the latest provisioning failure.
#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ProvisioningFailureStage {
    Provisioning,
    CoordinatorStatus,
    ComponentOrigin,
    ComponentAllocation,
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

/// Whether another active attempt is justified by the originating failure.
#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ProvisioningRetryCategory {
    Backoff,
    ReviewRequired,
}

/// Maximum delay for transient attempts that retain uncertain-effect reconciliation.
pub const MAX_PROVISIONING_BACKOFF_SECONDS: u64 = 60;

/// Return a bounded exponential delay, or suspend active attempts for review.
#[must_use]
pub fn retry_delay_seconds(category: ProvisioningRetryCategory, failures: u32) -> Option<u64> {
    if matches!(category, ProvisioningRetryCategory::ReviewRequired) {
        None
    } else {
        Some((1_u64 << failures.saturating_sub(1).min(6)).min(MAX_PROVISIONING_BACKOFF_SECONDS))
    }
}
