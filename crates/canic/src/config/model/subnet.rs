use crate::{
    Error,
    config::model::{ConfigModelError, Validate},
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

impl Validate for SubnetConfig {
    fn validate(&self) -> Result<(), ConfigModelError> {
        // --- 1. Validate directory entries ---
        for canister_ty in &self.subnet_directory {
            if !self.canisters.contains_key(canister_ty) {
                return Err(ConfigModelError::ValidationError(format!(
                    "subnet directory canister '{canister_ty}' is not defined in subnet",
                )));
            }
        }

        // --- 2. Validate canister configurations ---
        for (parent_ty, cfg) in &self.canisters {
            // Sharding pools
            if let Some(sharding) = &cfg.sharding {
                for (pool_name, pool) in &sharding.pools {
                    if !self.canisters.contains_key(&pool.canister_type) {
                        return Err(ConfigModelError::ValidationError(format!(
                            "canister '{parent_ty}' sharding pool '{pool_name}' references unknown canister type '{ty}'",
                            ty = pool.canister_type
                        )));
                    }
                }
            }

            // Scaling pools
            if let Some(scaling) = &cfg.scaling {
                for (pool_name, pool) in &scaling.pools {
                    if !self.canisters.contains_key(&pool.canister_type) {
                        return Err(ConfigModelError::ValidationError(format!(
                            "canister '{parent_ty}' scaling pool '{pool_name}' references unknown canister type '{ty}'",
                            ty = pool.canister_type
                        )));
                    }
                }
            }
        }

        Ok(())
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

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanisterTopup {
    #[serde(default, deserialize_with = "Cycles::from_config")]
    pub threshold: Cycles,

    #[serde(default, deserialize_with = "Cycles::from_config")]
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
}

impl Default for ScalePoolPolicy {
    fn default() -> Self {
        Self {
            min_workers: 1,
            max_workers: 32,
        }
    }
}

///
/// ShardingConfig
/// (stateful, partitioned)
///
/// * Organizes canisters into named **pools**.
/// * Each pool manages a set of **shards**, and each shard owns a partition of state.
/// * Tenants are assigned to shards via HRW and stay there.
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
    pub capacity: u32,
    pub max_shards: u32,
}

impl Default for ShardPoolPolicy {
    fn default() -> Self {
        Self {
            capacity: 1_000,
            max_shards: 4,
        }
    }
}

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn sharding_pool_references_must_exist_in_subnet() {
        let managing_ty: CanisterType = "shard_hub".into();
        let mut canisters = BTreeMap::new();

        let mut sharding = ShardingConfig::default();
        sharding.pools.insert(
            "primary".into(),
            ShardPool {
                canister_type: CanisterType::from("missing_shard_worker"),
                policy: ShardPoolPolicy::default(),
            },
        );

        let manager_cfg = CanisterConfig {
            sharding: Some(sharding),
            ..Default::default()
        };

        canisters.insert(managing_ty, manager_cfg);

        let subnet = SubnetConfig {
            canisters,
            ..Default::default()
        };

        let err = subnet
            .validate()
            .expect_err("expected missing worker type to fail");
        match err {
            ConfigModelError::ValidationError(msg) => {
                assert!(
                    msg.contains("missing_shard_worker"),
                    "error should include missing canister type, got: {msg}"
                );
            }
            other => panic!("unexpected error variant: {other:?}"),
        }
    }
}
