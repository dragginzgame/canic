use crate::{
    cdk::{
        candid::Principal,
        types::{Cycles, TC},
    },
    config::schema::{ConfigSchemaError, NAME_MAX_BYTES, Validate},
    ids::CanisterRole,
};
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
    /// Get a canister configuration by role.
    #[must_use]
    pub fn get_canister(&self, role: &CanisterRole) -> Option<CanisterConfig> {
        self.canisters.get(role).cloned()
    }
}

impl Validate for SubnetConfig {
    fn validate(&self) -> Result<(), ConfigSchemaError> {
        // auto_create must reference defined canisters
        for role in &self.auto_create {
            validate_role_len(role, "auto-create canister")?;
            if !self.canisters.contains_key(role) {
                return Err(ConfigSchemaError::ValidationError(format!(
                    "auto-create canister '{role}' is not defined in subnet",
                )));
            }
        }

        // subnet_directory must reference node canisters
        for role in &self.subnet_directory {
            validate_role_len(role, "subnet directory canister")?;
            let cfg = self.canisters.get(role).ok_or_else(|| {
                ConfigSchemaError::ValidationError(format!(
                    "subnet directory canister '{role}' is not defined in subnet",
                ))
            })?;

            if cfg.kind != CanisterKind::Node {
                return Err(ConfigSchemaError::ValidationError(format!(
                    "subnet directory canister '{role}' must have kind = \"node\"",
                )));
            }
        }

        for (role, cfg) in &self.canisters {
            validate_role_len(role, "canister")?;

            if cfg.randomness.enabled && cfg.randomness.reseed_interval_secs == 0 {
                return Err(ConfigSchemaError::ValidationError(format!(
                    "canister '{role}' randomness reseed_interval_secs must be > 0",
                )));
            }

            cfg.validate_kind(role)?;
            cfg.validate_scaling(role, &self.canisters)?;
            cfg.validate_sharding(role, &self.canisters)?;
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
    /// Optional count of canisters to import immediately before queuing the rest.
    #[serde(default)]
    pub initial: Option<u16>,
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
    /// Kind and placement semantics for this canister role.
    pub kind: CanisterKind,

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

impl CanisterConfig {
    fn validate_kind(&self, role: &CanisterRole) -> Result<(), ConfigSchemaError> {
        match self.kind {
            CanisterKind::Root => {
                if self.scaling.is_some() || self.sharding.is_some() {
                    return Err(ConfigSchemaError::ValidationError(format!(
                        "canister '{role}' kind = \"root\" cannot define scaling or sharding",
                    )));
                }
            }
            CanisterKind::Node => {}
            CanisterKind::Worker => {
                if self.scaling.is_none() {
                    return Err(ConfigSchemaError::ValidationError(format!(
                        "canister '{role}' kind = \"worker\" requires scaling config",
                    )));
                }
                if self.sharding.is_some() {
                    return Err(ConfigSchemaError::ValidationError(format!(
                        "canister '{role}' kind = \"worker\" cannot define sharding",
                    )));
                }
            }
            CanisterKind::Shard => {
                if self.scaling.is_some() {
                    return Err(ConfigSchemaError::ValidationError(format!(
                        "canister '{role}' kind = \"shard\" cannot define scaling",
                    )));
                }
            }
        }

        Ok(())
    }

    fn validate_sharding(
        &self,
        role: &CanisterRole,
        all_roles: &BTreeMap<CanisterRole, Self>,
    ) -> Result<(), ConfigSchemaError> {
        let Some(sharding) = &self.sharding else {
            return Ok(());
        };

        for (pool_name, pool) in &sharding.pools {
            if pool_name.len() > NAME_MAX_BYTES {
                return Err(ConfigSchemaError::ValidationError(format!(
                    "canister '{role}' sharding pool '{pool_name}' name exceeds {NAME_MAX_BYTES} bytes",
                )));
            }

            if !all_roles.contains_key(&pool.canister_role) {
                return Err(ConfigSchemaError::ValidationError(format!(
                    "canister '{role}' sharding pool '{pool_name}' references unknown canister role '{}'",
                    pool.canister_role
                )));
            }

            if pool.policy.capacity == 0 || pool.policy.max_shards == 0 {
                return Err(ConfigSchemaError::ValidationError(format!(
                    "canister '{role}' sharding pool '{pool_name}' must have positive capacity and max_shards",
                )));
            }
        }

        Ok(())
    }

    fn validate_scaling(
        &self,
        role: &CanisterRole,
        all_roles: &BTreeMap<CanisterRole, Self>,
    ) -> Result<(), ConfigSchemaError> {
        let Some(scaling) = &self.scaling else {
            return Ok(());
        };

        for (pool_name, pool) in &scaling.pools {
            if pool_name.len() > NAME_MAX_BYTES {
                return Err(ConfigSchemaError::ValidationError(format!(
                    "canister '{role}' scaling pool '{pool_name}' name exceeds {NAME_MAX_BYTES} bytes",
                )));
            }

            if !all_roles.contains_key(&pool.canister_role) {
                return Err(ConfigSchemaError::ValidationError(format!(
                    "canister '{role}' scaling pool '{pool_name}' references unknown canister role '{}'",
                    pool.canister_role
                )));
            }

            if pool.policy.max_workers != 0 && pool.policy.max_workers < pool.policy.min_workers {
                return Err(ConfigSchemaError::ValidationError(format!(
                    "canister '{role}' scaling pool '{pool_name}' has max_workers < min_workers",
                )));
            }
        }

        Ok(())
    }
}

///
/// CanisterKind
/// Kind semantics for canister roles within the topology.
///
/// Do not encode parent relationships here; this is role-level intent only.
///

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CanisterKind {
    Root,
    Node,
    Worker,
    Shard,
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

    fn base_canister_config(kind: CanisterKind) -> CanisterConfig {
        CanisterConfig {
            kind,
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
            ..base_canister_config(CanisterKind::Shard)
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
                ..base_canister_config(CanisterKind::Shard)
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
            base_canister_config(CanisterKind::Node),
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
                ..base_canister_config(CanisterKind::Shard)
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
            base_canister_config(CanisterKind::Node),
        );

        let manager_cfg = CanisterConfig {
            scaling: Some(ScalingConfig { pools }),
            ..base_canister_config(CanisterKind::Worker)
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
            base_canister_config(CanisterKind::Node),
        );

        let manager_cfg = CanisterConfig {
            scaling: Some(ScalingConfig { pools }),
            ..base_canister_config(CanisterKind::Worker)
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
            ..base_canister_config(CanisterKind::Node)
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

    #[test]
    fn shard_kind_allows_missing_sharding_config() {
        let mut canisters = BTreeMap::new();
        canisters.insert(
            CanisterRole::from("shard"),
            base_canister_config(CanisterKind::Shard),
        );

        let subnet = SubnetConfig {
            canisters,
            ..Default::default()
        };

        subnet
            .validate()
            .expect("expected shard config without sharding to validate");
    }

    #[test]
    fn explicit_canister_role_is_rejected() {
        toml::from_str::<SubnetConfig>(
            r#"
[canisters.app]
role = "app"
kind = "node"
"#,
        )
        .expect_err("expected explicit role to fail validation");
    }

    #[test]
    fn explicit_canister_type_is_rejected() {
        toml::from_str::<SubnetConfig>(
            r#"
[canisters.app]
kind = "node"
type = "node"
"#,
        )
        .expect_err("expected explicit type to fail validation");
    }

    #[test]
    fn explicit_sharding_role_is_rejected() {
        toml::from_str::<SubnetConfig>(
            r#"
[canisters.manager]
kind = "node"

[canisters.manager.sharding]
role = "shard"
"#,
        )
        .expect_err("expected explicit sharding role to fail validation");
    }
}
