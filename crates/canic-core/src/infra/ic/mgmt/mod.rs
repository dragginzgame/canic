//! Infra-scoped IC management canister helpers.
//!
//! These wrappers provide low-level management canister calls without workflow
//! or policy layering concerns.

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

#[derive(Debug, ThisError)]
pub enum MgmtInfraError {
    #[error("raw_rand returned {len} bytes")]
    RawRandInvalidLength { len: usize },
}

///
/// MgmtInfra
///

pub struct MgmtInfra;
