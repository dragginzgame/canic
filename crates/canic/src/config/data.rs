use crate::{
    Error,
    config::ConfigError,
    types::{CanisterType, Cycles, TC},
};
use candid::Principal;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error as ThisError;

///
/// ConfigDataError
///

#[derive(Debug, ThisError)]
pub enum ConfigDataError {
    #[error("invalid principal: {0} ({1})")]
    InvalidPrincipal(String, usize),

    #[error("canister not found: {0}")]
    CanisterNotFound(CanisterType),
}

mod defaults {
    use super::Cycles;
    pub fn initial_cycles() -> Cycles {
        Cycles::new(5_000_000_000_000)
    }
}

///
/// ConfigData
///

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigData {
    #[serde(default)]
    pub canisters: BTreeMap<CanisterType, Canister>,

    // controllers
    // a vec because we just append it to the controller arguments
    #[serde(default)]
    pub controllers: Vec<Principal>,

    #[serde(default)]
    pub reserve: CanisterReserve,

    #[serde(default)]
    pub standards: Option<Standards>,

    #[serde(default)]
    pub whitelist: Option<WhiteList>,
}

impl ConfigData {
    pub(super) fn validate(&self) -> Result<(), ConfigDataError> {
        if let Some(list) = &self.whitelist {
            for (i, s) in list.principals.iter().enumerate() {
                // Reject if invalid principal format
                if Principal::from_text(s).is_err() {
                    return Err(ConfigDataError::InvalidPrincipal(s.to_string(), i));
                }
            }
        }

        Ok(())
    }

    /// Lookup a canister config by type name (string).
    pub fn try_get_canister(&self, ty: &CanisterType) -> Result<Canister, Error> {
        self.canisters.get(ty).cloned().ok_or_else(|| {
            ConfigError::ConfigDataError(ConfigDataError::CanisterNotFound(ty.clone())).into()
        })
    }

    /// Return true if the given principal is present in the whitelist.
    #[must_use]
    pub fn is_whitelisted(&self, principal: &Principal) -> bool {
        self.whitelist
            .as_ref()
            .is_none_or(|w| w.principals.contains(&principal.to_string()))
    }

    /// Return whether ICRC-21 standard support is enabled.
    #[must_use]
    pub fn icrc21_enabled(&self) -> bool {
        self.standards.as_ref().is_some_and(|s| s.icrc21)
    }
}

///
/// Canister
///

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Canister {
    #[serde(default)]
    pub auto_create: bool,

    #[serde(default)]
    pub delegation: bool,

    #[serde(
        default = "defaults::initial_cycles",
        deserialize_with = "Cycles::from_config"
    )]
    pub initial_cycles: Cycles,

    #[serde(default)]
    pub topup: Option<CanisterTopup>,

    #[serde(default)]
    pub uses_directory: bool,

    #[serde(default)]
    pub scaling: Option<ScalingConfig>,

    #[serde(default)]
    pub sharding: Option<ShardingConfig>,
}

///
/// CanisterTopup
///
/// auto_topup : default to false
///

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanisterTopup {
    #[serde(deserialize_with = "Cycles::from_config")]
    pub threshold: Cycles,

    #[serde(deserialize_with = "Cycles::from_config")]
    pub amount: Cycles,
}

impl Default for CanisterTopup {
    fn default() -> Self {
        Self {
            threshold: Cycles::new(10 * TC),
            amount: Cycles::new(5 * TC),
        }
    }
}

///
/// CanisterReserve
/// defaults to a minimum size of 0
///

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CanisterReserve {
    pub minimum_size: u8,
}

///
/// Whitelist
///

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WhiteList {
    // principals
    // a hashset as we constantly have to do lookups
    // strings because then we can validate and know if there are any bad ones
    #[serde(default)]
    pub principals: BTreeSet<String>,
}

///
/// Standards
///

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Standards {
    #[serde(default)]
    pub icrc21: bool,

    #[serde(default)]
    pub icrc103: bool,
}

///
/// ScalingConfig
/// (stateless, scaling)
///
/// * Organizes canisters into **worker groups** (e.g. "oracle").
/// * Workers are interchangeable and handle transient tasks (no tenant assignment).
/// * Scaling is about throughput, not capacity.
/// * Hence: `WorkerManager → pools → WorkerSpec → WorkerPolicy`.
///

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScalingConfig {
    #[serde(default)]
    pub pools: BTreeMap<String, ScalePool>,
}

///
/// ScalePool
/// One stateless worker group (e.g. "oracle").
///

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScalePool {
    pub canister_type: CanisterType,

    #[serde(default)]
    pub policy: ScalePoolPolicy,
}

///
/// ScalePoolPolicy
///

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct ScalePoolPolicy {
    /// Minimum number of worker canisters to keep alive
    pub min_workers: u32,

    /// Maximum number of worker canisters to allow
    pub max_workers: u32,

    /// When average load % exceeds this, spawn a new worker
    pub scale_up_threshold_pct: u32,

    /// When average load % drops below this, retire a worker
    pub scale_down_threshold_pct: u32,
}

impl Default for ScalePoolPolicy {
    fn default() -> Self {
        Self {
            min_workers: 1,
            max_workers: 32,
            scale_up_threshold_pct: 75,
            scale_down_threshold_pct: 25,
        }
    }
}

///
/// ShardingConfig
/// (stateful, partitioned)
///
/// * Organizes canisters into named **pools**.
/// * Each pool manages a set of **shards**, and each shard owns a partition of state.
/// * Tenants are assigned to shards and stay there.
/// * Hence: `ShardManager → pools → ShardPoolSpec → ShardPoolPolicy`.
///

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShardingConfig {
    #[serde(default)]
    pub pools: BTreeMap<String, ShardPool>,
}

///
/// ShardPool
///

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShardPool {
    pub canister_type: CanisterType,
    #[serde(default)]
    pub policy: ShardPoolPolicy,
}

///
/// ShardPoolPolicy
///

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct ShardPoolPolicy {
    pub initial_capacity: u32,
    pub max_shards: u32,
    pub growth_threshold_pct: u32,
}

impl Default for ShardPoolPolicy {
    fn default() -> Self {
        Self {
            initial_capacity: 100,
            max_shards: 64,
            growth_threshold_pct: 80,
        }
    }
}
