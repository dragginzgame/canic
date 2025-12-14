use crate::types::{Cycles, TC};
use crate::{
    config::schema::{ConfigSchemaError, Validate},
    ids::CanisterRole,
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

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SubnetConfig {
    #[serde(default)]
    pub canisters: BTreeMap<CanisterRole, CanisterConfig>,

    #[serde(default)]
    pub auto_create: BTreeSet<CanisterRole>,

    #[serde(default)]
    pub subnet_directory: BTreeSet<CanisterRole>,

    #[serde(default)]
    pub reserve: CanisterReserve,
}

impl SubnetConfig {
    /// Returns the directory canisters for this subnet.
    #[must_use]
    pub fn directory_canisters(&self) -> Vec<CanisterRole> {
        self.subnet_directory.iter().cloned().collect()
    }

    /// Get a canister configuration by type.
    #[must_use]
    pub fn get_canister(&self, ty: &CanisterRole) -> Option<CanisterConfig> {
        self.canisters.get(ty).cloned()
    }
}

impl Validate for SubnetConfig {
    fn validate(&self) -> Result<(), ConfigSchemaError> {
        // --- 1. Validate directory entries ---
        for canister_ty in &self.subnet_directory {
            if !self.canisters.contains_key(canister_ty) {
                return Err(ConfigSchemaError::ValidationError(format!(
                    "subnet directory canister '{canister_ty}' is not defined in subnet",
                )));
            }
        }

        // --- 2. Validate auto-create entries ---
        for canister_ty in &self.auto_create {
            if !self.canisters.contains_key(canister_ty) {
                return Err(ConfigSchemaError::ValidationError(format!(
                    "auto-create canister '{canister_ty}' is not defined in subnet",
                )));
            }
        }

        // --- 3. Validate canister configurations ---
        for (parent_ty, cfg) in &self.canisters {
            // Sharding pools
            if let Some(sharding) = &cfg.sharding {
                for (pool_name, pool) in &sharding.pools {
                    if !self.canisters.contains_key(&pool.canister_type) {
                        return Err(ConfigSchemaError::ValidationError(format!(
                            "canister '{parent_ty}' sharding pool '{pool_name}' references unknown canister type '{ty}'",
                            ty = pool.canister_type
                        )));
                    }

                    if pool.policy.capacity == 0 {
                        return Err(ConfigSchemaError::ValidationError(format!(
                            "canister '{parent_ty}' sharding pool '{pool_name}' has zero capacity; must be > 0",
                        )));
                    }

                    if pool.policy.max_shards == 0 {
                        return Err(ConfigSchemaError::ValidationError(format!(
                            "canister '{parent_ty}' sharding pool '{pool_name}' has max_shards of 0; must be > 0",
                        )));
                    }
                }
            }

            // Scaling pools
            if let Some(scaling) = &cfg.scaling {
                for (pool_name, pool) in &scaling.pools {
                    if !self.canisters.contains_key(&pool.canister_type) {
                        return Err(ConfigSchemaError::ValidationError(format!(
                            "canister '{parent_ty}' scaling pool '{pool_name}' references unknown canister type '{ty}'",
                            ty = pool.canister_type
                        )));
                    }

                    if pool.policy.max_workers != 0
                        && pool.policy.max_workers < pool.policy.min_workers
                    {
                        return Err(ConfigSchemaError::ValidationError(format!(
                            "canister '{parent_ty}' scaling pool '{pool_name}' has max_workers < min_workers (min {}, max {})",
                            pool.policy.min_workers, pool.policy.max_workers
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

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CanisterReserve {
    pub minimum_size: u8,
}

///
/// CanisterConfig
///

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
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

#[derive(Clone, Debug, Deserialize, Serialize)]
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

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ScalingConfig {
    #[serde(default)]
    pub pools: BTreeMap<String, ScalePool>,
}

///
/// ScalePool
/// One stateless worker group (e.g. "oracle").
///

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ScalePool {
    pub canister_type: CanisterRole,

    #[serde(default)]
    pub policy: ScalePoolPolicy,
}

///
/// ScalePoolPolicy
///

#[derive(Clone, Debug, Deserialize, Serialize)]
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

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ShardingConfig {
    #[serde(default)]
    pub pools: BTreeMap<String, ShardPool>,
}

///
/// ShardPool
///

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ShardPool {
    pub canister_type: CanisterRole,

    #[serde(default)]
    pub policy: ShardPoolPolicy,
}

///
/// ShardPoolPolicy
///

#[derive(Clone, Debug, Deserialize, Serialize)]
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
    use std::collections::{BTreeMap, BTreeSet};

    #[test]
    fn auto_create_entries_must_exist_in_subnet() {
        let mut auto_create = BTreeSet::new();
        auto_create.insert(CanisterRole::from("missing_auto_canister"));

        let subnet = SubnetConfig {
            auto_create,
            ..Default::default()
        };

        subnet
            .validate()
            .expect_err("expected missing auto-create type to fail");
    }

    #[test]
    fn sharding_pool_references_must_exist_in_subnet() {
        let managing_role: CanisterRole = "shard_hub".into();
        let mut canisters = BTreeMap::new();

        let mut sharding = ShardingConfig::default();
        sharding.pools.insert(
            "primary".into(),
            ShardPool {
                canister_type: CanisterRole::from("missing_shard_worker"),
                policy: ShardPoolPolicy::default(),
            },
        );

        let manager_cfg = CanisterConfig {
            sharding: Some(sharding),
            ..Default::default()
        };

        canisters.insert(managing_role, manager_cfg);

        let subnet = SubnetConfig {
            canisters,
            ..Default::default()
        };

        subnet
            .validate()
            .expect_err("expected missing worker type to fail");
    }

    #[test]
    fn sharding_pool_policy_requires_positive_capacity_and_shards() {
        let managing_role: CanisterRole = "shard_hub".into();
        let mut canisters = BTreeMap::new();

        let mut sharding = ShardingConfig::default();
        sharding.pools.insert(
            "primary".into(),
            ShardPool {
                canister_type: managing_role.clone(),
                policy: ShardPoolPolicy {
                    capacity: 0,
                    max_shards: 0,
                },
            },
        );

        canisters.insert(
            managing_role,
            CanisterConfig {
                sharding: Some(sharding),
                ..Default::default()
            },
        );

        let subnet = SubnetConfig {
            canisters,
            ..Default::default()
        };

        subnet
            .validate()
            .expect_err("expected invalid sharding policy to fail");
    }

    #[test]
    fn scaling_pool_policy_requires_max_ge_min_when_bounded() {
        let mut canisters = BTreeMap::new();
        let mut pools = BTreeMap::new();
        pools.insert(
            "worker".into(),
            ScalePool {
                canister_type: CanisterRole::from("worker"),
                policy: ScalePoolPolicy {
                    min_workers: 5,
                    max_workers: 3,
                },
            },
        );

        canisters.insert(CanisterRole::from("worker"), CanisterConfig::default());

        let manager_cfg = CanisterConfig {
            scaling: Some(ScalingConfig { pools }),
            ..Default::default()
        };

        canisters.insert(CanisterRole::from("manager"), manager_cfg);

        let subnet = SubnetConfig {
            canisters,
            ..Default::default()
        };

        subnet
            .validate()
            .expect_err("expected invalid scaling policy to fail");
    }
}
