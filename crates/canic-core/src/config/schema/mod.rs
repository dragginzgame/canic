mod log;
mod subnet;

pub use log::*;
pub use subnet::*;

use crate::{
    InternalError, InternalErrorOrigin,
    cdk::candid::Principal,
    ids::{CanisterRole, SubnetRole},
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error as ThisError;

///
/// Configuration schema and validation.
///
/// WHY THIS MODULE EXISTS
/// -----------------------
/// This module defines the **authoritative configuration contract** for the
/// entire canister network.
///
/// All configuration MUST:
///   1. Deserialize into these types
///   2. Pass `Validate::validate()`
///
/// Invariants enforced here are assumed everywhere else in the system and
/// MUST NOT be revalidated at runtime.
///
/// This module is intentionally strict:
/// - `deny_unknown_fields` prevents silent misconfiguration
/// - Validation fails fast with human-readable errors
/// - Defaults are explicit and conservative
///
/// If validation passes, downstream code is allowed to trust the config.
///

///
/// ConfigSchemaError
///
/// Errors produced during schema validation.
/// These represent *configuration mistakes*, not runtime failures.
///
#[derive(Debug, ThisError)]
pub enum ConfigSchemaError {
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
#[cfg(any(not(target_arch = "wasm32"), test))]
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
/// - App index canisters must be SINGLETONs in PRIME
/// - Role names are length-limited
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
    /// These must exist in the PRIME subnet and be SINGLETON canisters.
    #[serde(default)]
    pub app_index: BTreeSet<CanisterRole>,

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
                randomness: RandomnessConfig::default(),
                scaling: None,
                sharding: None,
                directory: None,
                auth: CanisterAuthConfig::default(),
                standards: StandardsCanisterConfig::default(),
                metrics: MetricsCanisterConfig::default(),
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
            .is_none_or(|w| w.principals.contains(&principal.to_string()))
    }
}

///
/// FleetConfig
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

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AppConfig {
    #[serde(default)]
    pub init_mode: AppInitMode,

    /// Principal whitelist.
    ///
    /// Semantics:
    /// - None  => allow all principals (default-open)
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
/// - max_ttl_secs = None => use the runtime default TTL ceiling
/// - max_ttl_secs = Some => hard upper bound on token lifetime
///

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DelegatedTokenConfig {
    #[serde(default = "default_delegated_tokens_enabled")]
    pub enabled: bool,

    #[serde(default = "default_delegated_tokens_ecdsa_key_name")]
    pub ecdsa_key_name: String,

    #[serde(default)]
    pub max_ttl_secs: Option<u64>,
}

const fn default_delegated_tokens_enabled() -> bool {
    true
}

fn default_delegated_tokens_ecdsa_key_name() -> String {
    "key_1".to_string()
}

impl Default for DelegatedTokenConfig {
    fn default() -> Self {
        Self {
            enabled: default_delegated_tokens_enabled(),
            ecdsa_key_name: default_delegated_tokens_ecdsa_key_name(),
            max_ttl_secs: None,
        }
    }
}

///
/// RoleAttestationConfig
///
/// Controls root-signed role attestation issuance/verification defaults.
///

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RoleAttestationConfig {
    #[serde(default = "default_role_attestation_ecdsa_key_name")]
    pub ecdsa_key_name: String,

    #[serde(default = "default_role_attestation_max_ttl_secs")]
    pub max_ttl_secs: u64,

    #[serde(default)]
    pub min_accepted_epoch_by_role: BTreeMap<String, u64>,
}

fn default_role_attestation_ecdsa_key_name() -> String {
    "key_1".to_string()
}

const fn default_role_attestation_max_ttl_secs() -> u64 {
    900
}

impl Default for RoleAttestationConfig {
    fn default() -> Self {
        Self {
            ecdsa_key_name: default_role_attestation_ecdsa_key_name(),
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
///
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Standards {
    #[serde(default)]
    pub icrc21: bool,

    #[serde(default)]
    pub icrc103: bool,
}

#[cfg(test)]
mod tests;
