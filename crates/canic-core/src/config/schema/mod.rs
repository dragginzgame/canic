//! Module: config::schema
//!
//! Responsibility: define and validate the authoritative configuration schema.
//! Does not own: runtime config storage, workflow orchestration, or endpoint DTOs.
//! Boundary: configuration input deserializes here before downstream code trusts it.
//!
//! All configuration must deserialize into these types and pass validation.
//! Invariants enforced here are assumed everywhere else in the system.

mod log;
mod role;
mod subnet;

pub use log::*;
pub use role::*;
pub use subnet::*;

use crate::{
    InternalError, InternalErrorOrigin,
    cdk::candid::Principal,
    ids::{BuildNetwork, CanisterRole, SubnetRole},
};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error as _};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error as ThisError;

///
/// ConfigSchemaError
///
/// Errors produced during schema validation.
/// These represent *configuration mistakes*, not runtime failures.
///

#[derive(Debug, ThisError)]
pub enum ConfigSchemaError {
    #[error("validation error: {context} '{role}' {issue}")]
    InvalidCanisterRoleName {
        context: &'static str,
        role: String,
        issue: CanisterRoleNameIssue,
    },

    #[error("validation error: {0}")]
    ValidationError(String),
}

///
/// Maximum allowed byte length for role identifiers.
///
/// WHY THIS EXISTS:
/// - Prevents unbounded metric cardinality
/// - Keeps stable storage keys predictable
/// - Avoids accidental abuse via extremely long role names
///

pub const NAME_MAX_BYTES: usize = 40;

///
/// Config schema errors are internal configuration failures.
/// They are surfaced as InternalError with origin = Config.
///
impl From<ConfigSchemaError> for InternalError {
    fn from(err: ConfigSchemaError) -> Self {
        Self::domain(InternalErrorOrigin::Config, err.to_string())
    }
}

///
/// Validate
///
/// Trait implemented by all schema elements that require validation.
///
/// Validation is:
/// - Explicit
/// - Non-recursive unless explicitly called
/// - Guaranteed to run before config is used
///

#[cfg(any(not(target_arch = "wasm32"), test))]
pub trait Validate {
    fn validate(&self) -> Result<(), ConfigSchemaError>;
}

///
/// ConfigModel
///
/// Top-level configuration object.
///
/// Invariants enforced here:
/// - A PRIME subnet MUST exist
/// - Exactly one ROOT canister MUST exist globally
/// - ROOT canister MUST be in the PRIME subnet
/// - App index canisters must be SERVICEs in PRIME
/// - Canister role names follow the canonical deployment identity rule
/// - Delegated token TTL is sane
/// - Whitelist principals are valid
///

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigModel {
    /// Operator-facing fleet identity for host install state.
    #[serde(default)]
    pub fleet: Option<FleetConfig>,

    /// Controllers for the canister.
    /// Stored as a Vec because they are appended directly to controller args.
    #[serde(default)]
    pub controllers: Vec<Principal>,

    #[serde(default)]
    pub standards: Option<Standards>,

    #[serde(default)]
    pub log: LogConfig,

    #[serde(default)]
    pub auth: AuthConfig,

    /// App-level configuration (init mode, whitelist).
    #[serde(default)]
    pub app: AppConfig,

    /// Canister roles that participate in the application index.
    /// These must exist in the PRIME subnet and be SERVICE canisters.
    #[serde(default)]
    pub app_index: BTreeSet<CanisterRole>,

    /// Fleet-scoped role declarations. Topology attachment is derived from
    /// `subnets`; this table declares which package-backed roles exist.
    #[serde(default)]
    pub roles: BTreeMap<CanisterRole, RoleDeclaration>,

    /// All subnets keyed by role.
    #[serde(default)]
    pub subnets: BTreeMap<SubnetRole, SubnetConfig>,
}

impl ConfigModel {
    /// Get a subnet configuration by role.
    #[must_use]
    pub fn get_subnet(&self, role: &SubnetRole) -> Option<SubnetConfig> {
        self.subnets.get(role).cloned()
    }

    /// Return the configured fleet name.
    #[must_use]
    pub fn fleet_name(&self) -> Option<&str> {
        self.fleet.as_ref().and_then(|fleet| fleet.name.as_deref())
    }

    /// Return a fleet-scoped role reference for a local role.
    #[must_use]
    pub fn fleet_role_ref(&self, role: &CanisterRole) -> Option<FleetRoleRef> {
        self.fleet_name()
            .map(|fleet| FleetRoleRef::new(fleet, role.clone()))
    }

    /// Return whether a local canister role is explicitly declared.
    #[must_use]
    pub fn declares_role(&self, role: &CanisterRole) -> bool {
        self.roles.contains_key(role)
    }

    /// Return the local canister roles attached to topology.
    #[must_use]
    pub fn attached_roles(&self) -> BTreeSet<CanisterRole> {
        let mut attached = BTreeSet::new();
        let mut pending = Vec::new();

        for subnet in self.subnets.values() {
            for role in subnet.canisters.keys() {
                if attached.insert(role.clone()) {
                    pending.push(role.clone());
                }
            }
        }

        while let Some(role) = pending.pop() {
            for subnet in self.subnets.values() {
                let Some(canister) = subnet.canisters.get(&role) else {
                    continue;
                };

                for child in canister.role_bearing_child_roles() {
                    if attached.insert(child.clone()) {
                        pending.push(child.clone());
                    }
                }
            }
        }

        attached
    }

    /// Return the fleet-scoped roles attached to topology.
    #[must_use]
    pub fn attached_fleet_roles(&self) -> BTreeSet<FleetRoleRef> {
        let Some(fleet) = self.fleet_name() else {
            return BTreeSet::new();
        };

        self.attached_roles()
            .into_iter()
            .map(|role| FleetRoleRef::new(fleet, role))
            .collect()
    }

    /// Test-only helper: produces a minimally valid config.
    ///
    /// Includes:
    /// - PRIME subnet
    /// - ROOT canister of correct kind
    ///
    /// This avoids tests accidentally relying on invalid configs.
    #[cfg(test)]
    #[must_use]
    pub fn test_default() -> Self {
        let mut cfg = Self::default();
        let mut prime = SubnetConfig::default();

        prime.canisters.insert(
            CanisterRole::ROOT,
            CanisterConfig {
                kind: CanisterKind::Root,
                initial_cycles: crate::cdk::types::Cycles::new(0),
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
            },
        );

        cfg.fleet = Some(FleetConfig {
            name: Some("test".to_string()),
        });
        cfg.auth.delegated_tokens.enabled = true;
        cfg.auth.delegated_tokens.build_network = BuildNetwork::Local;
        cfg.auth.delegated_tokens.root_proof_mode = "chain_key_batch".to_string();
        cfg.auth.delegated_tokens.chain_key_root_proof.key_id = Some("key_1".to_string());
        cfg.auth
            .delegated_tokens
            .chain_key_root_proof
            .derivation_path_hash_hex =
            Some("fe51a87b988d221227b134c48f36787e891a902dcb5d48ea5f94cff8bfed5a16".to_string());
        cfg.auth
            .delegated_tokens
            .chain_key_root_proof
            .derivation_path_hex = Some(vec![
            "63616e6963".to_string(),
            "64656c65676174696f6e".to_string(),
        ]);
        cfg.auth
            .delegated_tokens
            .chain_key_root_proof
            .public_key_hex = Some("02".repeat(33));
        cfg.auth.delegated_tokens.chain_key_root_proof.key_version = Some(1);
        cfg.auth
            .delegated_tokens
            .chain_key_root_proof
            .min_accepted_key_version = Some(1);
        cfg.auth
            .delegated_tokens
            .chain_key_root_proof
            .min_accepted_proof_epoch = Some(1);
        cfg.auth
            .delegated_tokens
            .chain_key_root_proof
            .min_accepted_registry_epoch = Some(1);
        cfg.auth.delegated_tokens.chain_key_root_proof.valid_from_ns = Some(1);
        cfg.auth
            .delegated_tokens
            .chain_key_root_proof
            .accept_until_ns = Some(2);
        cfg.auth
            .delegated_tokens
            .chain_key_root_proof
            .max_revocation_latency_ns = Some(1);
        cfg.roles.insert(
            CanisterRole::ROOT,
            RoleDeclaration {
                kind: RoleDeclarationKind::Root,
                package: "root".to_string(),
            },
        );
        cfg.subnets.insert(SubnetRole::PRIME, prime);
        cfg
    }

    /// Check whether a principal is whitelisted.
    ///
    /// NOTE:
    /// Principals are stored as text intentionally so invalid values
    /// can be rejected at config load time.
    #[must_use]
    pub fn is_whitelisted(&self, principal: &Principal) -> bool {
        self.app
            .whitelist
            .as_ref()
            .is_some_and(|w| w.principals.contains(&principal.to_string()))
    }
}

///
/// FleetConfig
///
/// Operator-facing fleet identity configuration.
/// Owned by config schema and validated before install-state paths use it.
///

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FleetConfig {
    #[serde(default)]
    pub name: Option<String>,
}

///
/// AppConfig
///
/// Application startup mode and optional whitelist configuration.
/// Owned by config schema and consumed by access/app-state setup.
///

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AppConfig {
    #[serde(default)]
    pub init_mode: AppInitMode,

    /// Principal whitelist.
    ///
    /// Semantics:
    /// - None  => allow no principals (default-closed)
    /// - Some  => allow only listed principals (default-closed)
    #[serde(default)]
    pub whitelist: Option<Whitelist>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            init_mode: AppInitMode::Enabled,
            whitelist: None,
        }
    }
}

///
/// AppInitMode
///
/// Configurable initial app state.
/// Owned by config schema and mapped into app runtime state during bootstrap.
///

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AppInitMode {
    #[default]
    Enabled,
    Readonly,
    Disabled,
}

///
/// AuthConfig
///
/// Groups authentication-related configuration.
/// Owned by config schema and consumed by auth/runtime setup.
///

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AuthConfig {
    #[serde(default)]
    pub delegated_tokens: DelegatedTokenConfig,

    #[serde(default)]
    pub role_attestation: RoleAttestationConfig,
}

///
/// DelegatedTokenConfig
///
/// Controls root-signed delegated token authentication.
///
/// Semantics:
/// - enabled = false => delegated token auth disabled entirely
/// - root_canister_id = None => use the initialized Canic root env
/// - ic_root_public_key_raw_hex = None => allowed only when no canister in
///   this build verifies delegated tokens or role attestations
/// - max_ttl_secs = None => use the runtime default TTL ceiling
/// - max_ttl_secs = Some => hard upper bound on token lifetime
/// - root_proof_mode = "chain_key_batch" => current chain-key root proof contract
///
/// Owned by config schema and validated before delegated auth is enabled.
///

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DelegatedTokenConfig {
    #[serde(default = "default_delegated_tokens_enabled")]
    pub enabled: bool,

    #[serde(default)]
    pub root_canister_id: Option<String>,

    #[serde(default)]
    pub ic_root_public_key_raw_hex: Option<String>,

    #[serde(default = "default_delegated_tokens_root_proof_mode")]
    pub root_proof_mode: String,

    #[serde(default)]
    pub chain_key_root_proof: ChainKeyRootProofConfig,

    #[serde(
        default = "default_delegated_tokens_build_network",
        deserialize_with = "deserialize_build_network",
        serialize_with = "serialize_build_network"
    )]
    pub build_network: BuildNetwork,

    #[serde(default)]
    pub max_ttl_secs: Option<u64>,
}

///
/// ChainKeyRootProofConfig
///
/// Explicit verifier policy for chain-key batch root proofs.
///

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ChainKeyRootProofConfig {
    #[serde(default)]
    pub key_id: Option<String>,

    #[serde(default)]
    pub derivation_path_hash_hex: Option<String>,

    #[serde(default)]
    pub derivation_path_hex: Option<Vec<String>>,

    #[serde(default)]
    pub public_key_hex: Option<String>,

    #[serde(default)]
    pub key_version: Option<u64>,

    #[serde(default)]
    pub min_accepted_key_version: Option<u64>,

    #[serde(default)]
    pub min_accepted_proof_epoch: Option<u64>,

    #[serde(default)]
    pub min_accepted_registry_epoch: Option<u64>,

    #[serde(default)]
    pub valid_from_ns: Option<u64>,

    #[serde(default)]
    pub accept_until_ns: Option<u64>,

    #[serde(default)]
    pub max_revocation_latency_ns: Option<u64>,

    #[serde(default)]
    pub allow_test_key: bool,
}

const fn default_delegated_tokens_enabled() -> bool {
    false
}

fn default_delegated_tokens_root_proof_mode() -> String {
    "chain_key_batch".to_string()
}

const fn default_delegated_tokens_build_network() -> BuildNetwork {
    BuildNetwork::Ic
}

fn deserialize_build_network<'de, D>(deserializer: D) -> Result<BuildNetwork, D::Error>
where
    D: Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    BuildNetwork::parse(&value).ok_or_else(|| {
        D::Error::custom("auth.delegated_tokens.build_network must be one of ic, local")
    })
}

#[expect(
    clippy::trivially_copy_pass_by_ref,
    reason = "serde serialize_with requires a shared reference"
)]
fn serialize_build_network<S>(
    build_network: &BuildNetwork,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(build_network.as_str())
}

impl Default for DelegatedTokenConfig {
    fn default() -> Self {
        Self {
            enabled: default_delegated_tokens_enabled(),
            root_canister_id: None,
            ic_root_public_key_raw_hex: None,
            root_proof_mode: default_delegated_tokens_root_proof_mode(),
            chain_key_root_proof: ChainKeyRootProofConfig::default(),
            build_network: default_delegated_tokens_build_network(),
            max_ttl_secs: None,
        }
    }
}

///
/// RoleAttestationConfig
///
/// Controls root-signed role attestation issuance/verification defaults.
/// Owned by config schema and validated before role attestation is enabled.
///

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RoleAttestationConfig {
    #[serde(default = "default_role_attestation_max_ttl_secs")]
    pub max_ttl_secs: u64,

    #[serde(default)]
    pub min_accepted_epoch_by_role: BTreeMap<String, u64>,
}

const fn default_role_attestation_max_ttl_secs() -> u64 {
    900
}

impl Default for RoleAttestationConfig {
    fn default() -> Self {
        Self {
            max_ttl_secs: default_role_attestation_max_ttl_secs(),
            min_accepted_epoch_by_role: BTreeMap::new(),
        }
    }
}

///
/// Whitelist
///
/// Stores principals as text to allow validation at config load time.
/// Text representation is treated as canonical.
/// Owned by config schema and consumed by access whitelist checks.
///

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Whitelist {
    #[serde(default)]
    pub principals: BTreeSet<String>,
}

///
/// Standards
///
/// Feature flags for supported standards.
/// Owned by config schema and consumed by standards dispatch.
///

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Standards {
    #[serde(default)]
    pub icrc21: bool,

    #[serde(default)]
    pub icrc103: bool,
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests;
