use crate::{
    config::schema::{ConfigSchemaError, NAME_MAX_BYTES, Validate},
    ids::CanisterRole,
    types::{Cycles, TC},
};
use candid::Principal;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

mod defaults {
    use super::Cycles;

    pub const fn initial_cycles() -> Cycles {
        Cycles::new(5_000_000_000_000)
    }
}

fn validate_role_len(role: &CanisterRole, context: &str) -> Result<(), ConfigSchemaError> {
    if role.as_ref().len() > NAME_MAX_BYTES {
        return Err(ConfigSchemaError::ValidationError(format!(
            "{context} '{role}' exceeds {NAME_MAX_BYTES} bytes",
        )));
    }

    Ok(())
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
    pub pool: CanisterPool,
}

impl SubnetConfig {
    /// Returns the directory canisters for this subnet.
    #[must_use]
    pub fn directory_canisters(&self) -> Vec<CanisterRole> {
        self.subnet_directory.iter().cloned().collect()
    }

    /// Get a canister configuration by role.
    #[must_use]
    pub fn get_canister(&self, role: &CanisterRole) -> Option<CanisterConfig> {
        self.canisters.get(role).cloned()
    }
}

impl Validate for SubnetConfig {
    fn validate(&self) -> Result<(), ConfigSchemaError> {
        // --- 1. Validate directory entries ---
        for canister_role in &self.subnet_directory {
            validate_role_len(canister_role, "subnet directory canister")?;
            let canister_cfg = self.canisters.get(canister_role).ok_or_else(|| {
                ConfigSchemaError::ValidationError(format!(
                    "subnet directory canister '{canister_role}' is not defined in subnet",
                ))
            })?;

            if canister_cfg.cardinality != CanisterCardinality::Single {
                return Err(ConfigSchemaError::ValidationError(format!(
                    "subnet directory canister '{canister_role}' must have cardinality = \"single\"",
                )));
            }
        }

        // --- 2. Validate auto-create entries ---
        for canister_role in &self.auto_create {
            validate_role_len(canister_role, "auto-create canister")?;
            if !self.canisters.contains_key(canister_role) {
                return Err(ConfigSchemaError::ValidationError(format!(
                    "auto-create canister '{canister_role}' is not defined in subnet",
                )));
            }
        }

        // --- 3. Validate canister configurations ---
        for (parent_role, cfg) in &self.canisters {
            validate_role_len(parent_role, "canister")?;
            if cfg.randomness.enabled && cfg.randomness.reseed_interval_secs == 0 {
                return Err(ConfigSchemaError::ValidationError(format!(
                    "canister '{parent_role}' randomness reseed_interval_secs must be > 0",
                )));
            }

            // Sharding pools
            if let Some(sharding) = &cfg.sharding {
                for (pool_name, pool) in &sharding.pools {
                    if pool_name.len() > NAME_MAX_BYTES {
                        return Err(ConfigSchemaError::ValidationError(format!(
                            "canister '{parent_role}' sharding pool '{pool_name}' name exceeds {NAME_MAX_BYTES} bytes",
                        )));
                    }

                    if pool.canister_role.as_ref().len() > NAME_MAX_BYTES {
                        return Err(ConfigSchemaError::ValidationError(format!(
                            "canister '{parent_role}' sharding pool '{pool_name}' canister role '{role}' exceeds {NAME_MAX_BYTES} bytes",
                            role = pool.canister_role
                        )));
                    }

                    if !self.canisters.contains_key(&pool.canister_role) {
                        return Err(ConfigSchemaError::ValidationError(format!(
                            "canister '{parent_role}' sharding pool '{pool_name}' references unknown canister role '{role}'",
                            role = pool.canister_role
                        )));
                    }

                    if pool.policy.capacity == 0 {
                        return Err(ConfigSchemaError::ValidationError(format!(
                            "canister '{parent_role}' sharding pool '{pool_name}' has zero capacity; must be > 0",
                        )));
                    }

                    if pool.policy.max_shards == 0 {
                        return Err(ConfigSchemaError::ValidationError(format!(
                            "canister '{parent_role}' sharding pool '{pool_name}' has max_shards of 0; must be > 0",
                        )));
                    }
                }
            }

            // Scaling pools
            if let Some(scaling) = &cfg.scaling {
                for (pool_name, pool) in &scaling.pools {
                    if pool_name.len() > NAME_MAX_BYTES {
                        return Err(ConfigSchemaError::ValidationError(format!(
                            "canister '{parent_role}' scaling pool '{pool_name}' name exceeds {NAME_MAX_BYTES} bytes",
                        )));
                    }

                    if pool.canister_role.as_ref().len() > NAME_MAX_BYTES {
                        return Err(ConfigSchemaError::ValidationError(format!(
                            "canister '{parent_role}' scaling pool '{pool_name}' canister role '{role}' exceeds {NAME_MAX_BYTES} bytes",
                            role = pool.canister_role
                        )));
                    }

                    if !self.canisters.contains_key(&pool.canister_role) {
                        return Err(ConfigSchemaError::ValidationError(format!(
                            "canister '{parent_role}' scaling pool '{pool_name}' references unknown canister role '{role}'",
                            role = pool.canister_role
                        )));
                    }

                    if pool.policy.max_workers != 0
                        && pool.policy.max_workers < pool.policy.min_workers
                    {
                        return Err(ConfigSchemaError::ValidationError(format!(
                            "canister '{parent_role}' scaling pool '{pool_name}' has max_workers < min_workers (min {}, max {})",
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
/// PoolImport
/// Per-environment import lists for canister pools.
///
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PoolImport {
    #[serde(default)]
    pub local: Vec<Principal>,
    #[serde(default)]
    pub ic: Vec<Principal>,
}

///
/// CanisterPool
/// defaults to a minimum size of 0
///
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CanisterPool {
    pub minimum_size: u8,
    #[serde(default)]
    pub import: PoolImport,
}

///
/// CanisterConfig
///

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CanisterConfig {
    /// Required cardinality for this canister role.
    pub cardinality: CanisterCardinality,

    #[serde(
        default = "defaults::initial_cycles",
        deserialize_with = "Cycles::from_config"
    )]
    pub initial_cycles: Cycles,

    #[serde(default)]
    pub topup: Option<CanisterTopup>,

    #[serde(default)]
    pub randomness: RandomnessConfig,

    #[serde(default)]
    pub scaling: Option<ScalingConfig>,

    #[serde(default)]
    pub sharding: Option<ShardingConfig>,
}

///
/// CanisterCardinality
/// Indicates whether a canister role may have one or many instances.
///

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CanisterCardinality {
    Single,
    Many,
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
/// RandomnessConfig
///

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, default)]
pub struct RandomnessConfig {
    pub enabled: bool,
    pub reseed_interval_secs: u64,
    pub source: RandomnessSource,
}

impl Default for RandomnessConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            reseed_interval_secs: 3600,
            source: RandomnessSource::Ic,
        }
    }
}

///
/// RandomnessSource
///

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RandomnessSource {
    #[default]
    Ic,
    Time,
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
    pub canister_role: CanisterRole,

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
    pub canister_role: CanisterRole,

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

    fn base_canister_config(cardinality: CanisterCardinality) -> CanisterConfig {
        CanisterConfig {
            cardinality,
            initial_cycles: defaults::initial_cycles(),
            topup: None,
            randomness: RandomnessConfig::default(),
            scaling: None,
            sharding: None,
        }
    }

    #[test]
    fn randomness_defaults_to_ic() {
        let cfg = RandomnessConfig::default();

        assert!(cfg.enabled);
        assert_eq!(cfg.reseed_interval_secs, 3600);
        assert_eq!(cfg.source, RandomnessSource::Ic);
    }

    #[test]
    fn randomness_source_parses_ic_and_time() {
        let cfg: RandomnessConfig = toml::from_str("source = \"ic\"").unwrap();
        assert_eq!(cfg.source, RandomnessSource::Ic);

        let cfg: RandomnessConfig = toml::from_str("source = \"time\"").unwrap();
        assert_eq!(cfg.source, RandomnessSource::Time);
    }

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
            .expect_err("expected missing auto-create role to fail");
    }

    #[test]
    fn sharding_pool_references_must_exist_in_subnet() {
        let managing_role: CanisterRole = "shard_hub".into();
        let mut canisters = BTreeMap::new();

        let mut sharding = ShardingConfig::default();
        sharding.pools.insert(
            "primary".into(),
            ShardPool {
                canister_role: CanisterRole::from("missing_shard_worker"),
                policy: ShardPoolPolicy::default(),
            },
        );

        let manager_cfg = CanisterConfig {
            sharding: Some(sharding),
            ..base_canister_config(CanisterCardinality::Single)
        };

        canisters.insert(managing_role, manager_cfg);

        let subnet = SubnetConfig {
            canisters,
            ..Default::default()
        };

        subnet
            .validate()
            .expect_err("expected missing worker role to fail");
    }

    #[test]
    fn sharding_pool_policy_requires_positive_capacity_and_shards() {
        let managing_role: CanisterRole = "shard_hub".into();
        let mut canisters = BTreeMap::new();

        let mut sharding = ShardingConfig::default();
        sharding.pools.insert(
            "primary".into(),
            ShardPool {
                canister_role: managing_role.clone(),
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
                ..base_canister_config(CanisterCardinality::Single)
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
    fn canister_role_name_must_fit_bound() {
        let long_role = "a".repeat(NAME_MAX_BYTES + 1);
        let mut canisters = BTreeMap::new();
        canisters.insert(
            CanisterRole::from(long_role),
            base_canister_config(CanisterCardinality::Single),
        );

        let subnet = SubnetConfig {
            canisters,
            ..Default::default()
        };

        subnet
            .validate()
            .expect_err("expected canister role length to fail");
    }

    #[test]
    fn sharding_pool_name_must_fit_bound() {
        let managing_role: CanisterRole = "shard_hub".into();
        let mut canisters = BTreeMap::new();

        let mut sharding = ShardingConfig::default();
        sharding.pools.insert(
            "a".repeat(NAME_MAX_BYTES + 1),
            ShardPool {
                canister_role: managing_role.clone(),
                policy: ShardPoolPolicy::default(),
            },
        );

        canisters.insert(
            managing_role,
            CanisterConfig {
                sharding: Some(sharding),
                ..base_canister_config(CanisterCardinality::Single)
            },
        );

        let subnet = SubnetConfig {
            canisters,
            ..Default::default()
        };

        subnet
            .validate()
            .expect_err("expected sharding pool name length to fail");
    }

    #[test]
    fn scaling_pool_policy_requires_max_ge_min_when_bounded() {
        let mut canisters = BTreeMap::new();
        let mut pools = BTreeMap::new();
        pools.insert(
            "worker".into(),
            ScalePool {
                canister_role: CanisterRole::from("worker"),
                policy: ScalePoolPolicy {
                    min_workers: 5,
                    max_workers: 3,
                },
            },
        );

        canisters.insert(
            CanisterRole::from("worker"),
            base_canister_config(CanisterCardinality::Single),
        );

        let manager_cfg = CanisterConfig {
            scaling: Some(ScalingConfig { pools }),
            ..base_canister_config(CanisterCardinality::Single)
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

    #[test]
    fn scaling_pool_name_must_fit_bound() {
        let mut canisters = BTreeMap::new();
        let mut pools = BTreeMap::new();
        pools.insert(
            "a".repeat(NAME_MAX_BYTES + 1),
            ScalePool {
                canister_role: CanisterRole::from("worker"),
                policy: ScalePoolPolicy::default(),
            },
        );

        canisters.insert(
            CanisterRole::from("worker"),
            base_canister_config(CanisterCardinality::Single),
        );

        let manager_cfg = CanisterConfig {
            scaling: Some(ScalingConfig { pools }),
            ..base_canister_config(CanisterCardinality::Single)
        };

        canisters.insert(CanisterRole::from("manager"), manager_cfg);

        let subnet = SubnetConfig {
            canisters,
            ..Default::default()
        };

        subnet
            .validate()
            .expect_err("expected scaling pool name length to fail");
    }

    #[test]
    fn randomness_interval_requires_positive_value() {
        let mut canisters = BTreeMap::new();

        let cfg = CanisterConfig {
            randomness: RandomnessConfig {
                enabled: true,
                reseed_interval_secs: 0,
                ..Default::default()
            },
            ..base_canister_config(CanisterCardinality::Single)
        };

        canisters.insert(CanisterRole::from("app"), cfg);

        let subnet = SubnetConfig {
            canisters,
            ..Default::default()
        };

        subnet
            .validate()
            .expect_err("expected invalid randomness interval to fail");
    }
}
