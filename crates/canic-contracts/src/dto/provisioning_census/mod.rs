//! Native provisioning discovery, distinct from canonical retained journal encodings.
use crate::{
    dto::root::{RootProvisioningReleaseKey, RootProvisioningReleasePhase},
    values::provisioning_failure::{ProvisioningFailureStage, ProvisioningRetryCategory},
};
use ic_platform::Principal;

/// Complete RootProvisioningReleaseResponse transport evidence.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct Page {
    pub root: Principal,
    pub active_provisioning: Option<[u8; 32]>,
    pub active_directory_synchronization: Option<[u8; 32]>,
    pub entry: Option<Entry>,
    pub next_after: Option<RootProvisioningReleaseKey>,
}

/// Complete RootProvisioningReleaseEntry transport evidence.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct Entry {
    pub key: RootProvisioningReleaseKey,
    pub plan_hash: [u8; 32],
    pub phase: RootProvisioningReleasePhase,
    pub delivery_in_flight: Option<Principal>,
    pub last_failure: Option<Failure>,
}

/// Complete RootComponentProvisioningFailure transport evidence.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct Failure {
    pub dependency: Option<DependencyFailure>,
    pub stage: ProvisioningFailureStage,
    pub target: Principal,
    pub operation_id: [u8; 32],
    pub diagnostic_code: u16,
    pub retry_category: ProvisioningRetryCategory,
    pub failed_at_ns: u64,
    pub consecutive_failures: u32,
    pub retry_at_ns: Option<u64>,
}

/// Complete ProvisioningDependencyFailure transport evidence.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct DependencyFailure {
    pub target: Principal,
    pub method: String,
    pub step: String,
    pub rejection_code: Option<u32>,
    pub rejection_detail: Option<String>,
}
