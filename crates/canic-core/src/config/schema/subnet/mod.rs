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

mod defaults {
    use super::Cycles;

    pub const fn initial_cycles() -> Cycles {
        Cycles::new(5_000_000_000_000)
    }
}

const IMPLICIT_WASM_STORE_ROLE: CanisterRole = CanisterRole::WASM_STORE;

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
/// CanisterAuthConfig
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
        auth: CanisterAuthConfig::default(),
        standards: StandardsCanisterConfig::default(),
        metrics: MetricsCanisterConfig::default(),
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CanisterAuthConfig {
    #[serde(default)]
    pub delegated_token_signer: bool,

    #[serde(default)]
    pub role_attestation_cache: bool,
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
    pub auth: CanisterAuthConfig,

    #[serde(default)]
    pub standards: StandardsCanisterConfig,

    #[serde(default)]
    pub metrics: MetricsCanisterConfig,
}

impl CanisterConfig {
    #[must_use]
    pub fn resolved_metrics_profile(&self, role: &CanisterRole) -> MetricsProfile {
        if let Some(profile) = self.metrics.profile {
            return profile;
        }

        if self.kind == CanisterKind::Root || role.is_root() {
            return MetricsProfile::Root;
        }

        if role.is_wasm_store() {
            return MetricsProfile::Storage;
        }

        if self.scaling.is_some() || self.sharding.is_some() || self.directory.is_some() {
            return MetricsProfile::Hub;
        }

        MetricsProfile::Leaf
    }
}

///
/// MetricsCanisterConfig
///

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MetricsCanisterConfig {
    #[serde(default)]
    pub profile: Option<MetricsProfile>,
}

///
/// MetricsProfile
///

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MetricsProfile {
    Leaf,
    Hub,
    Storage,
    Root,
    Full,
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

#[cfg(test)]
mod tests;
