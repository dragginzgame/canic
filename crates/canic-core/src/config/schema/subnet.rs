use crate::{
    cdk::{
        candid::Principal,
        types::{Cycles, TC},
    },
    ids::CanisterRole,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

#[cfg(any(not(target_arch = "wasm32"), test))]
use crate::config::schema::{ConfigSchemaError, NAME_MAX_BYTES, Validate};

mod defaults {
    use super::Cycles;

    pub const fn initial_cycles() -> Cycles {
        Cycles::new(5_000_000_000_000)
    }
}

const IMPLICIT_WASM_STORE_ROLE: CanisterRole = CanisterRole::WASM_STORE;

#[cfg(any(not(target_arch = "wasm32"), test))]
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
    pub subnet_index: BTreeSet<CanisterRole>,

    #[serde(default)]
    pub pool: CanisterPool,
}

impl SubnetConfig {
    /// Get a canister configuration by role.
    #[must_use]
    pub fn get_canister(&self, role: &CanisterRole) -> Option<CanisterConfig> {
        self.canisters.get(role).cloned().or_else(|| {
            if *role == IMPLICIT_WASM_STORE_ROLE {
                Some(implicit_wasm_store_canister_config())
            } else {
                None
            }
        })
    }
}

#[cfg(any(not(target_arch = "wasm32"), test))]
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

        // subnet_index must reference singleton canisters
        for role in &self.subnet_index {
            validate_role_len(role, "subnet index canister")?;
            let cfg = self.canisters.get(role).ok_or_else(|| {
                ConfigSchemaError::ValidationError(format!(
                    "subnet index canister '{role}' is not defined in subnet",
                ))
            })?;

            if cfg.kind != CanisterKind::Singleton {
                return Err(ConfigSchemaError::ValidationError(format!(
                    "subnet index canister '{role}' must have kind = \"singleton\"",
                )));
            }
        }

        if self.canisters.contains_key(&IMPLICIT_WASM_STORE_ROLE) {
            return Err(ConfigSchemaError::ValidationError(format!(
                "{} is implicit and must not be configured under subnets.<name>.canisters",
                CanisterRole::WASM_STORE
            )));
        }

        for (role, cfg) in &self.canisters {
            validate_role_len(role, "canister")?;

            if cfg.randomness.enabled && cfg.randomness.reseed_interval_secs == 0 {
                return Err(ConfigSchemaError::ValidationError(format!(
                    "canister '{role}' randomness reseed_interval_secs must be > 0",
                )));
            }

            cfg.validate_kind(role)?;
            cfg.validate_topup_policy(role)?;
            cfg.validate_scaling(role, &self.canisters)?;
            cfg.validate_sharding(role, &self.canisters)?;
            cfg.validate_directory(role, &self.canisters)?;
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

///
/// DelegatedAuthCanisterConfig
///

// Build the implicit canister configuration for the mandatory store role.
fn implicit_wasm_store_canister_config() -> CanisterConfig {
    CanisterConfig {
        kind: CanisterKind::Singleton,
        initial_cycles: defaults::initial_cycles(),
        topup_policy: None,
        randomness: RandomnessConfig::default(),
        scaling: None,
        sharding: None,
        directory: None,
        delegated_auth: DelegatedAuthCanisterConfig::default(),
        standards: StandardsCanisterConfig::default(),
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DelegatedAuthCanisterConfig {
    #[serde(default)]
    pub signer: bool,

    #[serde(default)]
    pub attestation_cache: bool,
}

///
/// StandardsCanisterConfig
///

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StandardsCanisterConfig {
    #[serde(default)]
    pub icrc21: bool,
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
    pub topup_policy: Option<TopupPolicy>,

    #[serde(default)]
    pub randomness: RandomnessConfig,

    #[serde(default)]
    pub scaling: Option<ScalingConfig>,

    #[serde(default)]
    pub sharding: Option<ShardingConfig>,

    #[serde(default)]
    pub directory: Option<DirectoryConfig>,

    #[serde(default)]
    pub delegated_auth: DelegatedAuthCanisterConfig,

    #[serde(default)]
    pub standards: StandardsCanisterConfig,
}

impl CanisterConfig {
    // Enforce top-up bounds at config load time to avoid runaway refill cascades.
    #[cfg(any(not(target_arch = "wasm32"), test))]
    fn validate_topup_policy(&self, canister: &CanisterRole) -> Result<(), ConfigSchemaError> {
        let Some(topup_policy) = &self.topup_policy else {
            return Ok(());
        };

        let threshold = topup_policy.threshold.to_u128();
        let amount = topup_policy.amount.to_u128();

        if amount.saturating_mul(2) > threshold {
            return Err(ConfigSchemaError::ValidationError(format!(
                "canister '{canister}' topup_policy.amount must be <= 50% of topup_policy.threshold (got amount={amount}, threshold={threshold})",
            )));
        }

        Ok(())
    }

    #[cfg(any(not(target_arch = "wasm32"), test))]
    fn validate_kind(&self, canister: &CanisterRole) -> Result<(), ConfigSchemaError> {
        match self.kind {
            CanisterKind::Root => {
                if self.scaling.is_some()
                    || self.sharding.is_some()
                    || self.directory.is_some()
                    || self.delegated_auth.signer
                    || self.delegated_auth.attestation_cache
                    || self.standards.icrc21
                {
                    return Err(ConfigSchemaError::ValidationError(format!(
                        "canister '{canister}' kind = \"root\" cannot define scaling, sharding, directory, delegated auth signer/cache roles, or canister-local standards",
                    )));
                }
            }

            CanisterKind::Singleton => {
                // Singletons are the only canisters allowed to define scaling, sharding, and/or directory
            }

            CanisterKind::Replica | CanisterKind::Shard | CanisterKind::Instance => {
                if self.scaling.is_some() || self.sharding.is_some() || self.directory.is_some() {
                    return Err(ConfigSchemaError::ValidationError(format!(
                        "canister '{canister}' kind = \"{}\" cannot define scaling, sharding, or directory",
                        self.kind,
                    )));
                }
            }
        }

        Ok(())
    }

    #[cfg(any(not(target_arch = "wasm32"), test))]
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

            let target = &all_roles[&pool.canister_role];
            if target.kind != CanisterKind::Shard {
                return Err(ConfigSchemaError::ValidationError(format!(
                    "canister '{role}' sharding pool '{pool_name}' references canister '{}' which is not kind = \"shard\"",
                    pool.canister_role
                )));
            }

            if pool.policy.capacity == 0 || pool.policy.max_shards == 0 {
                return Err(ConfigSchemaError::ValidationError(format!(
                    "canister '{role}' sharding pool '{pool_name}' must have positive capacity and max_shards",
                )));
            }

            if pool.policy.initial_shards > pool.policy.max_shards {
                return Err(ConfigSchemaError::ValidationError(format!(
                    "canister '{role}' sharding pool '{pool_name}' has initial_shards > max_shards",
                )));
            }
        }

        Ok(())
    }

    #[cfg(any(not(target_arch = "wasm32"), test))]
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

            let target = &all_roles[&pool.canister_role];
            if target.kind != CanisterKind::Replica {
                return Err(ConfigSchemaError::ValidationError(format!(
                    "canister '{role}' scaling pool '{pool_name}' references canister '{}' which is not kind = \"replica\"",
                    pool.canister_role
                )));
            }

            if pool.policy.max_workers != 0 && pool.policy.max_workers < pool.policy.min_workers {
                return Err(ConfigSchemaError::ValidationError(format!(
                    "canister '{role}' scaling pool '{pool_name}' has max_workers < min_workers",
                )));
            }

            if pool.policy.max_workers != 0 && pool.policy.max_workers < pool.policy.initial_workers
            {
                return Err(ConfigSchemaError::ValidationError(format!(
                    "canister '{role}' scaling pool '{pool_name}' has max_workers < initial_workers",
                )));
            }
        }

        Ok(())
    }

    // Validate keyed instance-placement pools for singleton directory parents.
    #[cfg(any(not(target_arch = "wasm32"), test))]
    fn validate_directory(
        &self,
        role: &CanisterRole,
        all_roles: &BTreeMap<CanisterRole, Self>,
    ) -> Result<(), ConfigSchemaError> {
        let Some(directory) = &self.directory else {
            return Ok(());
        };

        for (pool_name, pool) in &directory.pools {
            if pool_name.len() > NAME_MAX_BYTES {
                return Err(ConfigSchemaError::ValidationError(format!(
                    "canister '{role}' directory pool '{pool_name}' name exceeds {NAME_MAX_BYTES} bytes",
                )));
            }

            if pool.key_name.is_empty() {
                return Err(ConfigSchemaError::ValidationError(format!(
                    "canister '{role}' directory pool '{pool_name}' must define a non-empty key_name",
                )));
            }

            if pool.key_name.len() > NAME_MAX_BYTES {
                return Err(ConfigSchemaError::ValidationError(format!(
                    "canister '{role}' directory pool '{pool_name}' key_name '{}' exceeds {NAME_MAX_BYTES} bytes",
                    pool.key_name
                )));
            }

            if !all_roles.contains_key(&pool.canister_role) {
                return Err(ConfigSchemaError::ValidationError(format!(
                    "canister '{role}' directory pool '{pool_name}' references unknown canister role '{}'",
                    pool.canister_role
                )));
            }

            let target = &all_roles[&pool.canister_role];
            if target.kind != CanisterKind::Instance {
                return Err(ConfigSchemaError::ValidationError(format!(
                    "canister '{role}' directory pool '{pool_name}' references canister '{}' which is not kind = \"instance\"",
                    pool.canister_role
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
    Singleton,
    Replica,
    Shard,
    Instance,
}

impl fmt::Display for CanisterKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::Root => "root",
            Self::Singleton => "singleton",
            Self::Replica => "replica",
            Self::Shard => "shard",
            Self::Instance => "instance",
        };

        f.write_str(label)
    }
}

///
/// TopupPolicy
///

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TopupPolicy {
    #[serde(default, deserialize_with = "Cycles::from_config")]
    pub threshold: Cycles,

    #[serde(default, deserialize_with = "Cycles::from_config")]
    pub amount: Cycles,
}

impl Default for TopupPolicy {
    fn default() -> Self {
        Self {
            threshold: Cycles::new(10 * TC),
            amount: Cycles::new(4 * TC),
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
/// * Organizes canisters into **replica groups** (e.g. "oracle").
/// * Replicas are interchangeable and handle transient tasks (no stable instance assignment).
/// * Scaling is about throughput, not capacity.
/// * Hence: `ReplicaManager → pools → ReplicaSpec → ReplicaPolicy`.
///

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ScalingConfig {
    #[serde(default)]
    pub pools: BTreeMap<String, ScalePool>,
}

///
/// ScalePool
/// One stateless replica group (e.g. "oracle").
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
    /// Number of replica canisters to create during startup warmup
    pub initial_workers: u32,

    /// Minimum number of replica canisters to keep alive
    pub min_workers: u32,

    /// Maximum number of replica canisters to allow
    pub max_workers: u32,
}

impl Default for ScalePoolPolicy {
    fn default() -> Self {
        Self {
            initial_workers: 1,
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
/// * Stable logical keys are assigned to shards via HRW and stay there.
/// * Hence: `ShardManager → pools → ShardPoolSpec → ShardPoolPolicy`.
///

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ShardingConfig {
    #[serde(default)]
    pub pools: BTreeMap<String, ShardPool>,
}

///
/// DirectoryConfig
/// (keyed instance placement)
///
/// * Organizes canisters into named **pools**.
/// * Each pool maps one configured key name to at most one dedicated instance root.
/// * The resolved instance identity is stable and usually owns a recursive subtree.
/// * Hence: `DirectoryManager → pools → DirectoryPool`.
///

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DirectoryConfig {
    #[serde(default)]
    pub pools: BTreeMap<String, DirectoryPool>,
}

///
/// DirectoryPool
///

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DirectoryPool {
    pub canister_role: CanisterRole,
    pub key_name: String,
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
    pub initial_shards: u32,
    pub max_shards: u32,
}

impl Default for ShardPoolPolicy {
    fn default() -> Self {
        Self {
            capacity: 1_000,
            initial_shards: 1,
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
            topup_policy: None,
            randomness: RandomnessConfig::default(),
            scaling: None,
            sharding: None,
            directory: None,
            delegated_auth: DelegatedAuthCanisterConfig::default(),
            standards: StandardsCanisterConfig::default(),
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
    fn root_canister_rejects_configured_delegated_auth_roles() {
        let mut cfg = base_canister_config(CanisterKind::Root);
        cfg.delegated_auth = DelegatedAuthCanisterConfig {
            signer: true,
            attestation_cache: true,
        };

        let err = cfg.validate_kind(&CanisterRole::ROOT).expect_err(
            "root delegated auth signer/cache roles must be implicit services, not config toggles",
        );

        assert!(
            err.to_string()
                .contains("delegated auth signer/cache roles"),
            "expected root delegated-auth role validation error, got: {err}"
        );
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
            .expect_err("expected missing replica role to fail");
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
                    initial_shards: 1,
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
    fn sharding_pool_policy_defaults_to_one_initial_shard() {
        let policy: ShardPoolPolicy =
            toml::from_str("capacity = 100\nmax_shards = 4").expect("policy should parse");

        assert_eq!(policy.initial_shards, 1);
    }

    #[test]
    fn sharding_pool_policy_rejects_initial_shards_above_max() {
        let managing_role: CanisterRole = "shard_hub".into();
        let worker_role: CanisterRole = "shard_worker".into();
        let mut canisters = BTreeMap::new();

        let mut sharding = ShardingConfig::default();
        sharding.pools.insert(
            "primary".into(),
            ShardPool {
                canister_role: worker_role.clone(),
                policy: ShardPoolPolicy {
                    capacity: 10,
                    initial_shards: 3,
                    max_shards: 2,
                },
            },
        );

        canisters.insert(worker_role, base_canister_config(CanisterKind::Shard));
        canisters.insert(
            managing_role,
            CanisterConfig {
                sharding: Some(sharding),
                ..base_canister_config(CanisterKind::Singleton)
            },
        );

        let subnet = SubnetConfig {
            canisters,
            ..Default::default()
        };

        subnet
            .validate()
            .expect_err("expected oversized initial_shards to fail");
    }

    #[test]
    fn canister_role_name_must_fit_bound() {
        let long_role = "a".repeat(NAME_MAX_BYTES + 1);
        let mut canisters = BTreeMap::new();
        canisters.insert(
            CanisterRole::from(long_role),
            base_canister_config(CanisterKind::Singleton),
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
            "replica".into(),
            ScalePool {
                canister_role: CanisterRole::from("replica"),
                policy: ScalePoolPolicy {
                    initial_workers: 1,
                    min_workers: 5,
                    max_workers: 3,
                },
            },
        );

        canisters.insert(
            CanisterRole::from("replica"),
            base_canister_config(CanisterKind::Replica),
        );

        let manager_cfg = CanisterConfig {
            scaling: Some(ScalingConfig { pools }),
            ..base_canister_config(CanisterKind::Singleton)
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
    fn scaling_pool_policy_defaults_to_one_initial_worker() {
        let policy: ScalePoolPolicy =
            toml::from_str("min_workers = 2\nmax_workers = 4").expect("policy should parse");

        assert_eq!(policy.initial_workers, 1);
    }

    #[test]
    fn scaling_pool_policy_rejects_initial_workers_above_bounded_max() {
        let mut canisters = BTreeMap::new();
        let mut pools = BTreeMap::new();
        pools.insert(
            "replica".into(),
            ScalePool {
                canister_role: CanisterRole::from("replica"),
                policy: ScalePoolPolicy {
                    initial_workers: 4,
                    min_workers: 1,
                    max_workers: 3,
                },
            },
        );

        canisters.insert(
            CanisterRole::from("replica"),
            base_canister_config(CanisterKind::Replica),
        );

        let manager_cfg = CanisterConfig {
            scaling: Some(ScalingConfig { pools }),
            ..base_canister_config(CanisterKind::Singleton)
        };

        canisters.insert(CanisterRole::from("manager"), manager_cfg);

        let subnet = SubnetConfig {
            canisters,
            ..Default::default()
        };

        subnet
            .validate()
            .expect_err("expected oversized initial_workers to fail");
    }

    #[test]
    fn scaling_pool_name_must_fit_bound() {
        let mut canisters = BTreeMap::new();
        let mut pools = BTreeMap::new();
        pools.insert(
            "a".repeat(NAME_MAX_BYTES + 1),
            ScalePool {
                canister_role: CanisterRole::from("replica"),
                policy: ScalePoolPolicy::default(),
            },
        );

        canisters.insert(
            CanisterRole::from("replica"),
            base_canister_config(CanisterKind::Replica),
        );

        let manager_cfg = CanisterConfig {
            scaling: Some(ScalingConfig { pools }),
            ..base_canister_config(CanisterKind::Singleton)
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
    fn directory_pool_references_must_exist_in_subnet() {
        let managing_role: CanisterRole = "project_hub".into();
        let mut canisters = BTreeMap::new();

        let mut directory = DirectoryConfig::default();
        directory.pools.insert(
            "projects".into(),
            DirectoryPool {
                canister_role: CanisterRole::from("missing_project_instance"),
                key_name: "project".into(),
            },
        );

        let manager_cfg = CanisterConfig {
            directory: Some(directory),
            ..base_canister_config(CanisterKind::Singleton)
        };

        canisters.insert(managing_role, manager_cfg);

        let subnet = SubnetConfig {
            canisters,
            ..Default::default()
        };

        subnet
            .validate()
            .expect_err("expected missing directory target role to fail");
    }

    #[test]
    fn directory_pool_target_must_be_instance_kind() {
        let managing_role: CanisterRole = "project_hub".into();
        let mut canisters = BTreeMap::new();

        let mut directory = DirectoryConfig::default();
        directory.pools.insert(
            "projects".into(),
            DirectoryPool {
                canister_role: CanisterRole::from("project_instance"),
                key_name: "project".into(),
            },
        );

        canisters.insert(
            CanisterRole::from("project_instance"),
            base_canister_config(CanisterKind::Singleton),
        );
        canisters.insert(
            managing_role,
            CanisterConfig {
                directory: Some(directory),
                ..base_canister_config(CanisterKind::Singleton)
            },
        );

        let subnet = SubnetConfig {
            canisters,
            ..Default::default()
        };

        subnet
            .validate()
            .expect_err("expected non-instance directory target role to fail");
    }

    #[test]
    fn directory_pool_requires_non_empty_key_name() {
        let managing_role: CanisterRole = "project_hub".into();
        let mut canisters = BTreeMap::new();

        let mut directory = DirectoryConfig::default();
        directory.pools.insert(
            "projects".into(),
            DirectoryPool {
                canister_role: CanisterRole::from("project_instance"),
                key_name: String::new(),
            },
        );

        canisters.insert(
            CanisterRole::from("project_instance"),
            base_canister_config(CanisterKind::Instance),
        );
        canisters.insert(
            managing_role,
            CanisterConfig {
                directory: Some(directory),
                ..base_canister_config(CanisterKind::Singleton)
            },
        );

        let subnet = SubnetConfig {
            canisters,
            ..Default::default()
        };

        subnet
            .validate()
            .expect_err("expected empty directory key name to fail");
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
            ..base_canister_config(CanisterKind::Singleton)
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
    fn wasm_store_canister_config_is_implicit() {
        let subnet = SubnetConfig::default();
        let cfg = subnet
            .get_canister(&CanisterRole::WASM_STORE)
            .expect("expected implicit wasm_store canister");

        assert_eq!(cfg.kind, CanisterKind::Singleton);
        assert_eq!(cfg.initial_cycles, defaults::initial_cycles());
    }

    #[test]
    fn explicit_wasm_store_canister_config_is_rejected() {
        let mut canisters = BTreeMap::new();
        canisters.insert(
            CanisterRole::WASM_STORE,
            base_canister_config(CanisterKind::Singleton),
        );

        let subnet = SubnetConfig {
            canisters,
            ..Default::default()
        };

        subnet
            .validate()
            .expect_err("expected explicit wasm_store config to fail");
    }

    #[test]
    fn topup_policy_amount_above_half_threshold_fails() {
        let mut canisters = BTreeMap::new();

        let cfg = CanisterConfig {
            topup_policy: Some(TopupPolicy {
                threshold: Cycles::new(10 * TC),
                amount: Cycles::new(6 * TC),
            }),
            ..base_canister_config(CanisterKind::Singleton)
        };

        canisters.insert(CanisterRole::from("app"), cfg);

        let subnet = SubnetConfig {
            canisters,
            ..Default::default()
        };

        subnet
            .validate()
            .expect_err("expected topup_policy amount above half threshold to fail");
    }

    #[test]
    fn topup_policy_amount_equal_half_threshold_is_valid() {
        let mut canisters = BTreeMap::new();

        let cfg = CanisterConfig {
            topup_policy: Some(TopupPolicy {
                threshold: Cycles::new(50 * TC),
                amount: Cycles::new(25 * TC),
            }),
            ..base_canister_config(CanisterKind::Singleton)
        };

        canisters.insert(CanisterRole::from("app"), cfg);

        let subnet = SubnetConfig {
            canisters,
            ..Default::default()
        };

        subnet
            .validate()
            .expect("expected topup_policy amount equal to half threshold to validate");
    }

    #[test]
    fn topup_policy_amount_below_half_threshold_is_valid() {
        let mut canisters = BTreeMap::new();

        let cfg = CanisterConfig {
            topup_policy: Some(TopupPolicy {
                threshold: Cycles::new(10 * TC),
                amount: Cycles::new(4 * TC),
            }),
            ..base_canister_config(CanisterKind::Singleton)
        };

        canisters.insert(CanisterRole::from("app"), cfg);

        let subnet = SubnetConfig {
            canisters,
            ..Default::default()
        };

        subnet
            .validate()
            .expect("expected topup_policy amount below half threshold to validate");
    }

    #[test]
    fn default_topup_policy_is_below_half_threshold() {
        let mut canisters = BTreeMap::new();

        let cfg = CanisterConfig {
            topup_policy: Some(TopupPolicy::default()),
            ..base_canister_config(CanisterKind::Singleton)
        };

        canisters.insert(CanisterRole::from("app"), cfg);

        let subnet = SubnetConfig {
            canisters,
            ..Default::default()
        };

        subnet
            .validate()
            .expect("expected default topup_policy to satisfy half-threshold invariant");
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
kind = "singleton"
"#,
        )
        .expect_err("expected explicit role to fail validation");
    }

    #[test]
    fn explicit_canister_type_is_rejected() {
        toml::from_str::<SubnetConfig>(
            r#"
[canisters.app]
kind = "singleton"
type = "singleton"
"#,
        )
        .expect_err("expected explicit type to fail validation");
    }

    #[test]
    fn explicit_sharding_role_is_rejected() {
        toml::from_str::<SubnetConfig>(
            r#"
[canisters.manager]
kind = "singleton"

[canisters.manager.sharding]
role = "shard"
"#,
        )
        .expect_err("expected explicit sharding role to fail validation");
    }

    #[test]
    fn instance_kind_parses() {
        let subnet = toml::from_str::<SubnetConfig>(
            r#"
[canisters.instance_role]
kind = "instance"
"#,
        )
        .expect("expected instance kind to parse");

        let cfg = subnet
            .canisters
            .get(&CanisterRole::from("instance_role"))
            .expect("instance role config should exist");
        assert_eq!(cfg.kind, CanisterKind::Instance);
    }

    #[test]
    fn removed_node_kind_is_rejected() {
        toml::from_str::<SubnetConfig>(
            r#"
[canisters.app]
kind = "node"
"#,
        )
        .expect_err("expected removed node kind to fail parsing");
    }

    #[test]
    fn removed_worker_kind_is_rejected() {
        toml::from_str::<SubnetConfig>(
            r#"
[canisters.app]
kind = "worker"
"#,
        )
        .expect_err("expected removed worker kind to fail parsing");
    }
}
