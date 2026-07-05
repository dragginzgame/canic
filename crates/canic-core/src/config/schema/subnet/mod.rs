//! Module: config::schema::subnet
//!
//! Responsibility: define subnet, canister, placement, and refill config shapes.
//! Does not own: topology validation, placement execution, or runtime canister state.
//! Boundary: config schema re-exports these data shapes for validated models.

use crate::{
    cdk::{candid::Principal, types::Cycles},
    ids::CanisterRole,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

mod defaults {
    use super::Cycles;
    use crate::cdk::types::TC;

    pub const fn initial_cycles() -> Cycles {
        Cycles::new(5_000_000_000_000)
    }

    pub const fn topup_threshold() -> Cycles {
        Cycles::new(10 * TC)
    }

    pub const fn topup_amount() -> Cycles {
        Cycles::new(5 * TC)
    }

    pub const fn cycles_funding_max_per_request() -> Cycles {
        Cycles::new(crate::domain::policy::pure::cycles_funding::DEFAULT_MAX_PER_REQUEST)
    }

    pub const fn cycles_funding_max_per_child() -> Cycles {
        Cycles::new(crate::domain::policy::pure::cycles_funding::DEFAULT_MAX_PER_CHILD)
    }

    pub const fn cycles_funding_cooldown_secs() -> u64 {
        crate::domain::policy::pure::cycles_funding::DEFAULT_COOLDOWN_SECS
    }
}

const IMPLICIT_WASM_STORE_ROLE: CanisterRole = CanisterRole::WASM_STORE;

///
/// SubnetConfig
///
/// Configuration for one subnet role and its declared canisters.
/// Owned by config schema and validated before topology workflows use it.
///

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SubnetConfig {
    #[serde(default)]
    pub canisters: BTreeMap<CanisterRole, CanisterConfig>,

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

    /// Roles that root creates automatically during subnet bootstrap.
    ///
    /// Configured service roles are the stable subnet services. Singletons,
    /// shards, replicas, and instances are created by their placement managers
    /// instead.
    #[must_use]
    pub fn auto_create_roles(&self) -> BTreeSet<CanisterRole> {
        self.service_roles()
    }

    /// Roles exposed through the subnet index.
    #[must_use]
    pub fn subnet_index_roles(&self) -> BTreeSet<CanisterRole> {
        self.service_roles()
    }

    fn service_roles(&self) -> BTreeSet<CanisterRole> {
        self.canisters
            .iter()
            .filter(|&(_role, canister)| canister.kind == CanisterKind::Service)
            .map(|(role, _canister)| role.clone())
            .collect()
    }
}

///
/// PoolImport
///
/// Per-environment import lists for canister pools.
/// Owned by config schema and consumed by pool import workflows.
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
///
/// Pool sizing and import configuration for root-managed canister pools.
/// Owned by config schema and validated before pool workflows use it.
///

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CanisterPool {
    pub minimum_size: u8,
    #[serde(default)]
    pub import: PoolImport,
}

///
/// CanisterAuthConfig
///
/// Canister-local auth feature flags.
/// Owned by config schema and consumed by auth/cache setup.
///

// Build the implicit canister configuration for the mandatory store role.
fn implicit_wasm_store_canister_config() -> CanisterConfig {
    CanisterConfig {
        kind: CanisterKind::Singleton,
        initial_cycles: defaults::initial_cycles(),
        topup: None,
        cycles_funding: CyclesFundingPolicyConfig::default(),
        randomness: RandomnessConfig::default(),
        scaling: None,
        sharding: None,
        directory: None,
        auth: CanisterAuthConfig::default(),
        standards: StandardsCanisterConfig::default(),
        diagnostics: DiagnosticsCanisterConfig::default(),
        metrics: MetricsCanisterConfig::default(),
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CanisterAuthConfig {
    #[serde(default)]
    pub delegated_token_issuer: bool,

    #[serde(default)]
    pub delegated_token_verifier: bool,

    #[serde(default)]
    pub role_attestation_cache: bool,
}

///
/// StandardsCanisterConfig
///
/// Canister-local standards feature flags.
/// Owned by config schema and consumed by standards dispatch.
///

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StandardsCanisterConfig {
    #[serde(default)]
    pub icrc21: bool,
}

///
/// DiagnosticsCanisterConfig
///
/// Canister-local diagnostics feature flags.
/// Owned by config schema and consumed by diagnostics endpoints.
///

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DiagnosticsCanisterConfig {
    #[serde(default)]
    pub memory_ledger: bool,
}

///
/// CanisterConfig
///
/// Configuration for one declared canister role.
/// Owned by config schema and consumed by bootstrap and topology workflows.
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
    pub topup: Option<TopupPolicy>,

    #[serde(default)]
    pub cycles_funding: CyclesFundingPolicyConfig,

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
    pub diagnostics: DiagnosticsCanisterConfig,

    #[serde(default)]
    pub metrics: MetricsCanisterConfig,
}

impl CanisterConfig {
    /// Resolve the effective metrics profile for a canister role.
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

    /// Return child roles referenced by exact role-bearing placement fields.
    #[must_use]
    pub fn role_bearing_child_roles(&self) -> Vec<&CanisterRole> {
        let scaling_roles = self
            .scaling
            .iter()
            .flat_map(|scaling| scaling.pools.values().map(|pool| &pool.canister_role));
        let sharding_roles = self
            .sharding
            .iter()
            .flat_map(|sharding| sharding.pools.values().map(|pool| &pool.canister_role));
        let directory_roles = self
            .directory
            .iter()
            .flat_map(|directory| directory.pools.values().map(|pool| &pool.canister_role));

        scaling_roles
            .chain(sharding_roles)
            .chain(directory_roles)
            .collect()
    }
}

///
/// CyclesFundingPolicyConfig
///
/// Parent funding limits applied when this role requests cycles from its parent.
/// Owned by config schema and consumed by cycles funding authorization.
///

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CyclesFundingPolicyConfig {
    #[serde(
        default = "defaults::cycles_funding_max_per_request",
        deserialize_with = "Cycles::from_config"
    )]
    pub max_per_request: Cycles,

    #[serde(
        default = "defaults::cycles_funding_max_per_child",
        deserialize_with = "Cycles::from_config"
    )]
    pub max_per_child: Cycles,

    #[serde(default = "defaults::cycles_funding_cooldown_secs")]
    pub cooldown_secs: u64,
}

impl Default for CyclesFundingPolicyConfig {
    fn default() -> Self {
        Self {
            max_per_request: defaults::cycles_funding_max_per_request(),
            max_per_child: defaults::cycles_funding_max_per_child(),
            cooldown_secs: defaults::cycles_funding_cooldown_secs(),
        }
    }
}

///
/// MetricsCanisterConfig
///
/// Canister-local metrics profile override.
/// Owned by config schema and consumed by metrics setup.
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
/// Metrics collection profile for a configured canister role.
/// Owned by config schema and consumed by metrics setup.
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
///
/// Kind semantics for canister roles within the topology.
///
/// Do not encode parent relationships here; this is role-level intent only.
///

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CanisterKind {
    Root,
    Service,
    Singleton,
    Replica,
    Shard,
    Instance,
}

impl fmt::Display for CanisterKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::Root => "root",
            Self::Service => "service",
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
/// Cycle top-up policy for one configured canister role.
/// Owned by config schema and consumed by funding workflows.
///

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TopupPolicy {
    #[serde(
        default = "defaults::topup_threshold",
        deserialize_with = "Cycles::from_config"
    )]
    pub threshold: Cycles,

    #[serde(
        default = "defaults::topup_amount",
        deserialize_with = "Cycles::from_config"
    )]
    pub amount: Cycles,

    #[serde(default)]
    pub icp_refill: Option<IcpRefillPolicy>,
}

impl Default for TopupPolicy {
    fn default() -> Self {
        Self {
            threshold: defaults::topup_threshold(),
            amount: defaults::topup_amount(),
            icp_refill: None,
        }
    }
}

///
/// IcpRefillPolicy
///
/// ICP-funded cycle refill policy for one configured canister role.
/// Owned by config schema and consumed by ICP refill workflows.
///

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IcpRefillPolicy {
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    #[serde(deserialize_with = "Cycles::from_config")]
    pub min_hub_cycles_before_refill: Cycles,

    pub max_refill_e8s_per_call: u64,

    #[serde(default)]
    pub min_xdr_permyriad_per_icp: Option<u64>,

    #[serde(default)]
    pub ledger_canister_id: Option<Principal>,

    #[serde(default)]
    pub cmc_canister_id: Option<Principal>,

    #[serde(default)]
    pub allow_ic_system_canister_overrides: bool,
}

const fn default_enabled() -> bool {
    true
}

///
/// RandomnessConfig
///
/// Randomness behavior configuration for one canister role.
/// Owned by config schema and consumed by runtime randomness setup.
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
/// Randomness source selected for one canister role.
/// Owned by config schema and consumed by runtime randomness setup.
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
///
/// Stateless replica-group placement configuration.
/// Owned by config schema and consumed by scaling placement workflows.
///

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ScalingConfig {
    #[serde(default)]
    pub pools: BTreeMap<String, ScalePool>,
}

///
/// ScalePool
///
/// One stateless replica group.
/// Owned by config schema and consumed by scaling placement workflows.
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
/// Worker bounds for one stateless replica group.
/// Owned by config schema and consumed by scaling placement policy.
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
///
/// Stateful partitioned shard-pool configuration.
/// Owned by config schema and consumed by sharding placement workflows.
///

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ShardingConfig {
    #[serde(default)]
    pub pools: BTreeMap<String, ShardPool>,
}

///
/// DirectoryConfig
///
/// Keyed instance placement configuration.
/// Owned by config schema and consumed by directory placement workflows.
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
/// One keyed instance placement pool.
/// Owned by config schema and consumed by directory placement workflows.
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
/// One stateful shard placement pool.
/// Owned by config schema and consumed by sharding placement workflows.
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
/// Capacity and shard-count bounds for one shard pool.
/// Owned by config schema and consumed by sharding placement policy.
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

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests;
