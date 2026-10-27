//! Passive protocol classifications; runtime decisions remain with Core.

use serde::{Deserialize, Serialize};

/// Maximum bytes in one exact dependency method label.
pub const MAX_PROVISIONING_DEPENDENCY_METHOD_BYTES: usize = 128;
/// Maximum bytes in one provisioning substep label.
pub const MAX_PROVISIONING_DEPENDENCY_STEP_BYTES: usize = 64;
/// Maximum UTF-8 bytes retained from a protected platform rejection.
pub const MAX_PROVISIONING_REJECTION_BYTES: usize = 1024;

/// Protected dependency evidence, independent of public diagnostics and spending authority.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
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

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
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

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub enum ProvisioningRetryCategory {
    Backoff,
    ReviewRequired,
}
