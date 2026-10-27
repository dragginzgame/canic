//! Native diagnostics preserve complete fixture and authority evidence without changing storage.
use crate::{
    dto::{
        component_directory::ManagedCanisterBinding,
        fixture_provisioning::{FixtureImportError, FixtureImportFailure},
        runtime::{
            CanisterTimerStatus, ReadinessStatus, RecentFailure, RuntimeAuthStatusSummary,
            RuntimeBuildInfo, RuntimeCheck, RuntimeDiagnostic, RuntimeFeatureStatus,
            RuntimeReceiptCapacityStatus, RuntimeStateSummary, RuntimeStatus,
            RuntimeVisibilityEntry,
        },
    },
    ids::{BuildNetwork, ReleaseBuildId},
};
use ic_platform::Principal;
use serde::{Deserialize, Serialize};

/// Complete CanicRuntimeStatus as a native transport projection.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct CanicRuntimeStatus {
    pub schema_version: u32,
    pub observed_at_ns: u64,
    pub canister_id: Principal,
    pub role: Option<String>,
    pub root: Option<Principal>,
    pub build_network: Option<BuildNetwork>,
    pub build: RuntimeBuildInfo,
    pub features: Vec<RuntimeFeatureStatus>,
    pub topology: Option<RuntimeTopologyStatus>,
    pub timers: Vec<CanisterTimerStatus>,
    pub timer_inventory: RuntimeCheck,
    pub state: Option<RuntimeStateSummary>,
    pub auth: Option<RuntimeAuthStatusSummary>,
    pub receipt_capacity: Option<RuntimeReceiptCapacityStatus>,
    pub recent_failures: Vec<RecentFailure>,
    pub visibility: Vec<RuntimeVisibilityEntry>,
    pub readiness: CanicReadinessStatus,
    pub status: RuntimeStatus,
}

/// Complete CanicReadinessStatus as a native transport projection.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct CanicReadinessStatus {
    pub fixture: Result<FixtureProvisioningStatus, FixtureImportError>,
    pub schema_version: u32,
    pub role: Option<String>,
    pub status: ReadinessStatus,
    pub observed_at_ns: u64,
    pub checks: Vec<RuntimeCheck>,
    pub blockers: Vec<RuntimeDiagnostic>,
    pub warnings: Vec<RuntimeDiagnostic>,
}

/// Complete RuntimeTopologyStatus as a native transport projection.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct RuntimeTopologyStatus {
    pub root: Option<Principal>,
    pub parent: Option<Principal>,
    pub subnet: Option<Principal>,
    pub source: String,
}

/// Complete FixtureTargetBinding as a native transport projection.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct FixtureTargetBinding {
    pub target: ManagedCanisterBinding,
    pub installation: [u8; 32],
    pub release_build_id: ReleaseBuildId,
    pub content_id: [u8; 32],
}

/// Complete FixtureImportReceipt as a native transport projection.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct FixtureImportReceipt {
    pub binding: FixtureTargetBinding,
    pub completion_summary: [u8; 32],
}

/// Complete FixtureImportProgress as a native transport projection.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct FixtureImportProgress {
    pub binding: FixtureTargetBinding,
    pub next_chunk: u32,
    pub receipt: Option<Box<FixtureImportReceipt>>,
}

/// Exact application import state, including installation-bound completion evidence.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub enum FixtureProvisioningStatus {
    NotRequired,
    AwaitingImporter,
    Pending(Option<Box<FixtureImportProgress>>),
    Complete(Box<FixtureImportReceipt>),
    Failed(FixtureImportFailure),
}
