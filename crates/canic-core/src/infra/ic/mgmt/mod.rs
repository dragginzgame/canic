//! Module: infra::ic::mgmt
//!
//! Responsibility: group raw IC management canister adapters.
//! Does not own: deployment workflow, lifecycle policy, or endpoint DTO shaping.
//! Boundary: ops calls this namespace for approved management canister effects.

mod cycles;
mod lifecycle;
mod randomness;
mod snapshots;
mod status_settings;
mod types;

pub use types::{
    InfraCanisterInstallMode, InfraCanisterSettings, InfraCanisterSnapshot,
    InfraCanisterStatusResult, InfraCanisterStatusType, InfraDefiniteCanisterSettings,
    InfraEnvironmentVariable, InfraLogVisibility, InfraMemoryMetrics, InfraQueryStats,
    InfraUpdateSettingsArgs, InfraUpgradeFlags, InfraWasmMemoryPersistence,
};

use thiserror::Error as ThisError;

///
/// MgmtInfraError
///
/// Management canister adapter failure.
/// Owned by management infra and returned to IC infra callers.
///

#[derive(Debug, ThisError)]
pub enum MgmtInfraError {
    #[error("raw_rand returned {len} bytes")]
    RawRandInvalidLength { len: usize },
}

///
/// MgmtInfra
///
/// Raw management canister adapter facade.
/// Owned by IC infra and extended by management adapter leaves.
///

pub struct MgmtInfra;
