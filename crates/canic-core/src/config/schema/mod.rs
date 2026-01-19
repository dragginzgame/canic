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
pub const NAME_MAX_BYTES: usize = 40;

fn validate_canister_role_len(role: &CanisterRole, context: &str) -> Result<(), ConfigSchemaError> {
    if role.as_ref().len() > NAME_MAX_BYTES {
        return Err(ConfigSchemaError::ValidationError(format!(
            "{context} '{role}' exceeds {NAME_MAX_BYTES} bytes",
        )));
    }
    Ok(())
}

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
/// - App directory canisters must be NODEs in PRIME
/// - Role names are length-limited
/// - Delegation TTL is sane
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
    pub delegation: DelegationConfig,

    /// Canister roles that participate in the application directory.
    /// These must exist in the PRIME subnet and be NODE canisters.
    #[serde(default)]
    pub app_directory: BTreeSet<CanisterRole>,

    /// All subnets keyed by role.
    #[serde(default)]
    pub subnets: BTreeMap<SubnetRole, SubnetConfig>,

    /// Principal whitelist.
    ///
    /// Semantics:
    /// - None  => allow all principals (default-open)
    /// - Some  => allow only listed principals (default-closed)
    #[serde(default)]
    pub whitelist: Option<Whitelist>,
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
        self.whitelist
            .as_ref()
            .is_none_or(|w| w.principals.contains(&principal.to_string()))
    }
}

impl Validate for ConfigModel {
    fn validate(&self) -> Result<(), ConfigSchemaError> {
        // Validation order is intentional to surface the most meaningful
        // errors first and avoid cascaded failures.

        for subnet_role in self.subnets.keys() {
            validate_subnet_role_len(subnet_role, "subnet")?;
        }

        self.log.validate()?;
        self.delegation.validate()?;

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

        // App directory canisters must exist in PRIME and be NODEs
        for canister_role in &self.app_directory {
            validate_canister_role_len(canister_role, "app directory canister")?;

            let canister_cfg = prime_subnet.canisters.get(canister_role).ok_or_else(|| {
                ConfigSchemaError::ValidationError(format!(
                    "app directory canister '{canister_role}' is not in prime subnet",
                ))
            })?;

            if canister_cfg.kind != CanisterKind::Node {
                return Err(ConfigSchemaError::ValidationError(format!(
                    "app directory canister '{canister_role}' must have kind = \"node\"",
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

        if let Some(list) = &self.whitelist {
            list.validate()?;
        }

        for subnet in self.subnets.values() {
            subnet.validate()?;
        }

        Ok(())
    }
}

///
/// DelegationConfig
///
/// Controls delegated token authentication.
///
/// Semantics:
/// - enabled = false => delegation disabled entirely
/// - max_ttl_secs = None => no upper TTL bound
/// - max_ttl_secs = Some => hard upper bound on token lifetime
///
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DelegationConfig {
    #[serde(default)]
    pub enabled: bool,

    #[serde(default)]
    pub max_ttl_secs: Option<u64>,
}

impl Validate for DelegationConfig {
    fn validate(&self) -> Result<(), ConfigSchemaError> {
        if let Some(max_ttl_secs) = self.max_ttl_secs
            && max_ttl_secs == 0
        {
            return Err(ConfigSchemaError::ValidationError(
                "delegation.max_ttl_secs must be greater than zero".into(),
            ));
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
            topup: None,
            randomness: RandomnessConfig::default(),
            scaling: None,
            sharding: None,
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

        canisters.insert(CanisterRole::ROOT, base_canister_config(CanisterKind::Node));

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
    fn delegation_max_ttl_zero_is_invalid() {
        let mut cfg = ConfigModel::test_default();
        cfg.delegation.enabled = true;
        cfg.delegation.max_ttl_secs = Some(0);

        cfg.validate().expect_err("expected zero ttl to fail");
    }

    #[test]
    fn invalid_whitelist_principal_is_rejected() {
        let mut cfg = ConfigModel::test_default();
        cfg.whitelist = Some(Whitelist {
            principals: std::iter::once("not-a-principal".into()).collect(),
        });

        cfg.validate()
            .expect_err("expected invalid principal to fail");
    }
}
