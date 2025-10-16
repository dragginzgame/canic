use crate::{
    Error,
    config::model::ConfigModelError,
    types::{CanisterType, Cycles, TC},
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

mod defaults {
    use super::Cycles;

    pub fn initial_cycles() -> Cycles {
        Cycles::new(5_000_000_000_000)
    }
}

///
/// SubnetConfig
///

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SubnetConfig {
    #[serde(default)]
    pub canisters: BTreeMap<CanisterType, CanisterConfig>,

    #[serde(default)]
    pub auto_create: BTreeSet<CanisterType>,

    #[serde(default)]
    pub subnet_directory: BTreeSet<CanisterType>,

    #[serde(default)]
    pub reserve: CanisterReserve,
}

impl SubnetConfig {
    /// Returns the directory canisters for this subnet.
    #[must_use]
    pub fn directory_canisters(&self) -> Vec<CanisterType> {
        self.subnet_directory.iter().cloned().collect()
    }

    /// Get a canister configuration by type.
    pub fn try_get_canister(&self, ty: &CanisterType) -> Result<CanisterConfig, Error> {
        self.canisters
            .get(ty)
            .cloned()
            .ok_or_else(|| ConfigModelError::CanisterNotFound(ty.clone()).into())
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
/// CanisterConfig
///

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanisterConfig {
    #[serde(
        default = "defaults::initial_cycles",
        deserialize_with = "Cycles::from_config"
    )]
    pub initial_cycles: Cycles,

    #[serde(default)]
    pub topup: Option<CanisterTopup>,

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
