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

#[cfg(any(not(target_arch = "wasm32"), test))]
fn validate_canister_role_len(role: &CanisterRole, context: &str) -> Result<(), ConfigSchemaError> {
    if role.as_ref().len() > NAME_MAX_BYTES {
        return Err(ConfigSchemaError::ValidationError(format!(
            "{context} '{role}' exceeds {NAME_MAX_BYTES} bytes",
        )));
    }
    Ok(())
}

#[cfg(any(not(target_arch = "wasm32"), test))]
fn validate_subnet_role_len(role: &SubnetRole, context: &str) -> Result<(), ConfigSchemaError> {
    if role.as_ref().len() > NAME_MAX_BYTES {
        return Err(ConfigSchemaError::ValidationError(format!(
            "{context} '{role}' exceeds {NAME_MAX_BYTES} bytes",
        )));
    }
    Ok(())
}

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
    #[serde(default, alias = "app_directory")]
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
                topup_policy: None,
                randomness: RandomnessConfig::default(),
                scaling: None,
                sharding: None,
                delegated_auth: DelegatedAuthCanisterConfig::default(),
                standards: StandardsCanisterConfig::default(),
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

#[cfg(any(not(target_arch = "wasm32"), test))]
impl Validate for ConfigModel {
    fn validate(&self) -> Result<(), ConfigSchemaError> {
        // Validation order is intentional to surface the most meaningful
        // errors first and avoid cascaded failures.

        for subnet_role in self.subnets.keys() {
            validate_subnet_role_len(subnet_role, "subnet")?;
        }

        self.log.validate()?;
        self.auth.validate()?;
        self.app.validate()?;

        // PRIME subnet must exist
        let prime = SubnetRole::PRIME;
        let prime_subnet = self
            .subnets
            .get(&prime)
            .ok_or_else(|| ConfigSchemaError::ValidationError("prime subnet not found".into()))?;

        // ROOT canister must exist in PRIME and be kind=Root
        let root_role = CanisterRole::ROOT;
        let root_cfg = prime_subnet.canisters.get(&root_role).ok_or_else(|| {
            ConfigSchemaError::ValidationError("root canister not defined in prime subnet".into())
        })?;

        if root_cfg.kind != CanisterKind::Root {
            return Err(ConfigSchemaError::ValidationError(
                "root canister must have kind = \"root\"".into(),
            ));
        }

        // App index canisters must exist in PRIME and be SINGLETONs
        for canister_role in &self.app_index {
            validate_canister_role_len(canister_role, "app index canister")?;

            let canister_cfg = prime_subnet.canisters.get(canister_role).ok_or_else(|| {
                ConfigSchemaError::ValidationError(format!(
                    "app index canister '{canister_role}' is not in prime subnet",
                ))
            })?;

            if canister_cfg.kind != CanisterKind::Singleton {
                return Err(ConfigSchemaError::ValidationError(format!(
                    "app index canister '{canister_role}' must have kind = \"singleton\"",
                )));
            }
        }

        // Exactly one ROOT canister must exist globally
        let mut root_roles = Vec::new();
        for (subnet_role, subnet) in &self.subnets {
            for (canister_role, canister_cfg) in &subnet.canisters {
                if canister_cfg.kind == CanisterKind::Root {
                    root_roles.push(format!("{subnet_role}:{canister_role}"));
                }
            }
        }

        if root_roles.len() > 1 {
            return Err(ConfigSchemaError::ValidationError(format!(
                "root kind must be unique globally (found {})",
                root_roles.join(", "),
            )));
        }

        for subnet in self.subnets.values() {
            subnet.validate()?;
        }

        Ok(())
    }
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

#[cfg(any(not(target_arch = "wasm32"), test))]
impl Validate for AppConfig {
    fn validate(&self) -> Result<(), ConfigSchemaError> {
        if let Some(list) = &self.whitelist {
            list.validate()?;
        }
        Ok(())
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

#[cfg(any(not(target_arch = "wasm32"), test))]
impl Validate for AuthConfig {
    fn validate(&self) -> Result<(), ConfigSchemaError> {
        self.delegated_tokens.validate()?;
        self.role_attestation.validate()
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DelegationProofCacheProfile {
    Small,
    Standard,
    Large,
}

impl DelegationProofCacheProfile {
    #[must_use]
    pub const fn capacity(self) -> usize {
        match self {
            Self::Small => 64,
            Self::Standard => 96,
            Self::Large => 160,
        }
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Small => "small",
            Self::Standard => "standard",
            Self::Large => "large",
        }
    }

    const fn from_shard_count_hint(shard_count_hint: Option<u16>) -> Self {
        match shard_count_hint {
            Some(0..=16) => Self::Small,
            Some(17..=48) | None => Self::Standard,
            Some(_) => Self::Large,
        }
    }
}

///
/// DelegationProofCacheConfig
///

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DelegationProofCacheConfig {
    #[serde(default)]
    pub profile: Option<DelegationProofCacheProfile>,

    #[serde(default)]
    pub shard_count_hint: Option<u16>,

    #[serde(default)]
    pub capacity_override: Option<u16>,

    #[serde(default = "default_delegation_proof_cache_active_window_secs")]
    pub active_window_secs: u32,
}

impl DelegationProofCacheConfig {
    #[must_use]
    pub fn resolved_profile(&self) -> DelegationProofCacheProfile {
        self.profile.unwrap_or_else(|| {
            DelegationProofCacheProfile::from_shard_count_hint(self.shard_count_hint)
        })
    }

    pub fn resolved_capacity(&self) -> usize {
        self.capacity_override
            .map_or_else(|| self.resolved_profile().capacity(), usize::from)
    }
}

const fn default_delegation_proof_cache_active_window_secs() -> u32 {
    10 * 60
}

impl Default for DelegationProofCacheConfig {
    fn default() -> Self {
        Self {
            profile: None,
            shard_count_hint: None,
            capacity_override: None,
            active_window_secs: default_delegation_proof_cache_active_window_secs(),
        }
    }
}

#[cfg(any(not(target_arch = "wasm32"), test))]
impl Validate for DelegationProofCacheConfig {
    fn validate(&self) -> Result<(), ConfigSchemaError> {
        if matches!(self.shard_count_hint, Some(0)) {
            return Err(ConfigSchemaError::ValidationError(
                "auth.delegated_tokens.proof_cache.shard_count_hint must be greater than zero"
                    .into(),
            ));
        }

        if matches!(self.capacity_override, Some(0)) {
            return Err(ConfigSchemaError::ValidationError(
                "auth.delegated_tokens.proof_cache.capacity_override must be greater than zero"
                    .into(),
            ));
        }

        if self.active_window_secs == 0 {
            return Err(ConfigSchemaError::ValidationError(
                "auth.delegated_tokens.proof_cache.active_window_secs must be greater than zero"
                    .into(),
            ));
        }

        let minimum_capacity = self.resolved_profile().capacity();
        if let Some(capacity_override) = self.capacity_override
            && usize::from(capacity_override) < minimum_capacity
        {
            return Err(ConfigSchemaError::ValidationError(format!(
                "auth.delegated_tokens.proof_cache.capacity_override must be >= {minimum_capacity} for profile '{}'",
                self.resolved_profile().as_str(),
            )));
        }

        Ok(())
    }
}

///
/// DelegatedTokenConfig
///
/// Controls root-signed delegated token authentication.
///
/// Semantics:
/// - enabled = false => delegated token auth disabled entirely
/// - max_ttl_secs = None => no upper TTL bound
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

    #[serde(default)]
    pub proof_cache: DelegationProofCacheConfig,
}

const fn default_delegated_tokens_enabled() -> bool {
    true
}

fn default_delegated_tokens_ecdsa_key_name() -> String {
    "test_key_1".to_string()
}

impl Default for DelegatedTokenConfig {
    fn default() -> Self {
        Self {
            enabled: default_delegated_tokens_enabled(),
            ecdsa_key_name: default_delegated_tokens_ecdsa_key_name(),
            max_ttl_secs: None,
            proof_cache: DelegationProofCacheConfig::default(),
        }
    }
}

#[cfg(any(not(target_arch = "wasm32"), test))]
impl Validate for DelegatedTokenConfig {
    fn validate(&self) -> Result<(), ConfigSchemaError> {
        if self.ecdsa_key_name.trim().is_empty() {
            return Err(ConfigSchemaError::ValidationError(
                "auth.delegated_tokens.ecdsa_key_name must not be empty".into(),
            ));
        }

        if let Some(max_ttl_secs) = self.max_ttl_secs
            && max_ttl_secs == 0
        {
            return Err(ConfigSchemaError::ValidationError(
                "auth.delegated_tokens.max_ttl_secs must be greater than zero".into(),
            ));
        }

        self.proof_cache.validate()
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
    "test_key_1".to_string()
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

#[cfg(any(not(target_arch = "wasm32"), test))]
impl Validate for RoleAttestationConfig {
    fn validate(&self) -> Result<(), ConfigSchemaError> {
        if self.ecdsa_key_name.trim().is_empty() {
            return Err(ConfigSchemaError::ValidationError(
                "auth.role_attestation.ecdsa_key_name must not be empty".into(),
            ));
        }

        if self.max_ttl_secs == 0 {
            return Err(ConfigSchemaError::ValidationError(
                "auth.role_attestation.max_ttl_secs must be greater than zero".into(),
            ));
        }

        for role in self.min_accepted_epoch_by_role.keys() {
            if role.trim().is_empty() {
                return Err(ConfigSchemaError::ValidationError(
                    "auth.role_attestation.min_accepted_epoch_by_role keys must not be empty"
                        .into(),
                ));
            }
        }

        Ok(())
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

#[cfg(any(not(target_arch = "wasm32"), test))]
impl Validate for Whitelist {
    fn validate(&self) -> Result<(), ConfigSchemaError> {
        for (i, s) in self.principals.iter().enumerate() {
            if Principal::from_text(s).is_err() {
                return Err(ConfigSchemaError::ValidationError(format!(
                    "principal #{i} {s} is invalid"
                )));
            }
        }
        Ok(())
    }
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

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cdk::types::Cycles;
    use std::collections::BTreeMap;

    fn base_canister_config(kind: CanisterKind) -> CanisterConfig {
        CanisterConfig {
            kind,
            initial_cycles: Cycles::new(0),
            topup_policy: None,
            randomness: RandomnessConfig::default(),
            scaling: None,
            sharding: None,
            delegated_auth: DelegatedAuthCanisterConfig::default(),
            standards: StandardsCanisterConfig::default(),
        }
    }

    #[test]
    fn root_canister_must_exist_in_prime_subnet() {
        let mut cfg = ConfigModel::default();
        cfg.subnets
            .insert(SubnetRole::PRIME, SubnetConfig::default());

        cfg.validate()
            .expect_err("expected missing root canister to fail validation");
    }

    #[test]
    fn root_canister_must_be_kind_root() {
        let mut cfg = ConfigModel::test_default();
        let mut canisters = BTreeMap::new();

        canisters.insert(
            CanisterRole::ROOT,
            base_canister_config(CanisterKind::Singleton),
        );

        cfg.subnets.get_mut(&SubnetRole::PRIME).unwrap().canisters = canisters;

        cfg.validate().expect_err("expected non-root kind to fail");
    }

    #[test]
    fn multiple_root_canisters_are_rejected() {
        let mut cfg = ConfigModel::test_default();

        cfg.subnets.insert(
            SubnetRole::new("aux"),
            SubnetConfig {
                canisters: {
                    let mut m = BTreeMap::new();
                    m.insert(CanisterRole::ROOT, base_canister_config(CanisterKind::Root));
                    m
                },
                ..Default::default()
            },
        );

        cfg.validate().expect_err("expected multiple roots to fail");
    }

    #[test]
    fn delegated_tokens_max_ttl_zero_is_invalid() {
        let mut cfg = ConfigModel::test_default();
        cfg.auth.delegated_tokens.max_ttl_secs = Some(0);

        cfg.validate().expect_err("expected zero ttl to fail");
    }

    #[test]
    fn delegated_tokens_proof_cache_shard_count_hint_zero_is_invalid() {
        let mut cfg = ConfigModel::test_default();
        cfg.auth.delegated_tokens.proof_cache.shard_count_hint = Some(0);

        cfg.validate()
            .expect_err("expected zero shard count hint to fail");
    }

    #[test]
    fn delegated_tokens_proof_cache_active_window_zero_is_invalid() {
        let mut cfg = ConfigModel::test_default();
        cfg.auth.delegated_tokens.proof_cache.active_window_secs = 0;

        cfg.validate()
            .expect_err("expected zero active window to fail");
    }

    #[test]
    fn delegated_tokens_proof_cache_capacity_override_below_profile_min_is_invalid() {
        let mut cfg = ConfigModel::test_default();
        cfg.auth.delegated_tokens.proof_cache.profile = Some(DelegationProofCacheProfile::Large);
        cfg.auth.delegated_tokens.proof_cache.capacity_override = Some(96);

        cfg.validate()
            .expect_err("expected undersized override to fail");
    }

    #[test]
    fn delegated_tokens_proof_cache_profile_resolves_from_shard_hint() {
        let mut cfg = ConfigModel::test_default();
        cfg.auth.delegated_tokens.proof_cache.shard_count_hint = Some(12);
        assert_eq!(
            cfg.auth.delegated_tokens.proof_cache.resolved_profile(),
            DelegationProofCacheProfile::Small
        );

        cfg.auth.delegated_tokens.proof_cache.shard_count_hint = Some(32);
        assert_eq!(
            cfg.auth.delegated_tokens.proof_cache.resolved_profile(),
            DelegationProofCacheProfile::Standard
        );

        cfg.auth.delegated_tokens.proof_cache.shard_count_hint = Some(64);
        assert_eq!(
            cfg.auth.delegated_tokens.proof_cache.resolved_profile(),
            DelegationProofCacheProfile::Large
        );
    }

    #[test]
    fn role_attestation_max_ttl_zero_is_invalid() {
        let mut cfg = ConfigModel::test_default();
        cfg.auth.role_attestation.max_ttl_secs = 0;

        cfg.validate().expect_err("expected zero ttl to fail");
    }

    #[test]
    fn role_attestation_empty_min_epoch_role_key_is_invalid() {
        let mut cfg = ConfigModel::test_default();
        cfg.auth
            .role_attestation
            .min_accepted_epoch_by_role
            .insert("   ".to_string(), 1);

        cfg.validate()
            .expect_err("expected empty min epoch role key to fail");
    }

    #[test]
    fn invalid_whitelist_principal_is_rejected() {
        let mut cfg = ConfigModel::test_default();
        cfg.app.whitelist = Some(Whitelist {
            principals: std::iter::once("not-a-principal".into()).collect(),
        });

        cfg.validate()
            .expect_err("expected invalid principal to fail");
    }
}
