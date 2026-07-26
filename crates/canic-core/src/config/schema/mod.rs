//! Module: config::schema
//!
//! Responsibility: define and validate the authoritative configuration schema.
//! Does not own: runtime config storage, workflow orchestration, or endpoint DTOs.
//! Boundary: configuration input deserializes here before downstream code trusts it.
//!
//! All configuration must deserialize into these types and pass validation.
//! Invariants enforced here are assumed everywhere else in the system.

mod component_spec;
mod log;
mod role;

pub use component_spec::*;
pub use log::*;
pub use role::*;

use crate::{
    InternalError, InternalErrorOrigin,
    cdk::candid::Principal,
    ids::{AppId, BuildNetwork, CanisterRole, ComponentSpecId},
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

/// Maximum concrete Component instances admitted by one Fleet declaration.
pub const MAX_FLEET_COMPONENT_INSTANCES: u32 = 4_096;
/// Maximum distinct direct-child roles declared by one Component Spec.
pub const MAX_COMPONENT_CHILD_ROLES: usize = 256;
/// Maximum outbound peer-Component grants declared by one Component Spec.
pub const MAX_COMPONENT_PROVISIONING_GRANTS: usize = 256;

///
/// AppNameIssue
///
/// Typed reason that an App name cannot be used as source or path identity.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq, ThisError)]
pub enum AppNameIssue {
    #[error("must not be empty")]
    Empty,

    #[error("must use only ASCII letters, numbers, '-' or '_'")]
    InvalidCharacters,

    #[error("must not exceed {max_bytes} bytes")]
    TooLong { max_bytes: usize },
}

/// Validate the canonical App-name shape before it is used as identity or in a path.
pub fn validate_app_name(value: &str) -> Result<(), AppNameIssue> {
    if value.is_empty() {
        return Err(AppNameIssue::Empty);
    }
    if value.len() > NAME_MAX_BYTES {
        return Err(AppNameIssue::TooLong {
            max_bytes: NAME_MAX_BYTES,
        });
    }
    if !value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err(AppNameIssue::InvalidCharacters);
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
/// - Every Component Spec contains exactly one direct Component role
/// - Component roles and direct child roles are structurally disjoint
/// - Component Spec instance ceilings are positive and Fleet-bounded
/// - Canister role names follow the canonical deployment identity rule
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

    /// App source identity, startup mode and whitelist.
    pub app: AppConfig,

    /// App-scoped role declarations. Topology attachment is derived from
    /// `component_specs`; this table declares which package-backed roles exist.
    #[serde(default)]
    pub roles: BTreeMap<CanisterRole, RoleDeclaration>,

    /// App-declared non-recursive Component topology templates.
    #[serde(default)]
    pub component_specs: BTreeMap<ComponentSpecId, ComponentSpecConfig>,
}

impl ConfigModel {
    /// Get a Component Spec configuration by its declared identity.
    #[must_use]
    pub fn get_component_spec(
        &self,
        component_spec: &ComponentSpecId,
    ) -> Option<ComponentSpecConfig> {
        self.component_specs.get(component_spec).cloned()
    }

    /// Iterate Component Specs that structurally contain a role.
    pub fn component_specs_for_role<'a>(
        &'a self,
        role: &CanisterRole,
    ) -> impl Iterator<Item = (&'a ComponentSpecId, &'a ComponentSpecConfig)> + 'a {
        let role = role.clone();
        self.component_specs
            .iter()
            .filter(move |(_component_spec, config)| {
                config.roles().any(|candidate| candidate == &role)
            })
    }

    /// Resolve a role only when exactly one Component Spec contains it.
    ///
    /// Component roles are always unique. A direct child role may be reused by
    /// several Specs, in which case callers need an explicit Component Spec
    /// binding and this role-only helper returns `None`.
    #[must_use]
    pub fn component_spec_for_role(
        &self,
        role: &CanisterRole,
    ) -> Option<(&ComponentSpecId, &ComponentSpecConfig)> {
        let mut matches = self.component_specs_for_role(role);
        let one = matches.next()?;
        matches.next().is_none().then_some(one)
    }

    /// Return the configured App identity.
    #[must_use]
    pub const fn app_id(&self) -> &AppId {
        &self.app.name
    }

    /// Return an App-scoped role reference for a local role.
    #[must_use]
    pub fn app_role_ref(&self, role: &CanisterRole) -> AppRoleRef {
        AppRoleRef::new(self.app.name.clone(), role.clone())
    }

    /// Return whether a local canister role is explicitly declared.
    #[must_use]
    pub fn declares_role(&self, role: &CanisterRole) -> bool {
        self.roles.contains_key(role)
    }

    /// Return the local canister roles attached to topology.
    #[must_use]
    pub fn attached_roles(&self) -> BTreeSet<CanisterRole> {
        self.component_specs
            .values()
            .flat_map(ComponentSpecConfig::roles)
            .cloned()
            .collect()
    }

    /// Return every role eligible for a deployment artifact.
    ///
    /// The Fleet Subnet Root is deployable infrastructure outside Component
    /// topology; all other deployable roles come from Component Specs.
    #[must_use]
    pub fn deployable_roles(&self) -> BTreeSet<CanisterRole> {
        let mut roles = self.attached_roles();
        if self.roles.contains_key(&CanisterRole::ROOT) {
            roles.insert(CanisterRole::ROOT);
        }
        roles
    }

    /// Return the App-scoped roles attached to topology.
    #[must_use]
    pub fn attached_app_roles(&self) -> BTreeSet<AppRoleRef> {
        self.attached_roles()
            .into_iter()
            .map(|role| AppRoleRef::new(self.app.name.clone(), role))
            .collect()
    }

    /// Return the currently projected Fleet Directory role set.
    ///
    /// The Component-aware Fleet Registry projection replaces this transitional
    /// role union in the dedicated Directory slice.
    #[must_use]
    pub fn fleet_directory_roles(&self) -> BTreeSet<CanisterRole> {
        self.component_specs
            .values()
            .flat_map(ComponentSpecConfig::component_directory_roles)
            .collect()
    }

    /// Test-only helper: produces a minimally valid config.
    ///
    /// Includes one default Component Spec plus declared root and Component roles.
    ///
    /// This avoids tests accidentally relying on invalid configs.
    ///
    /// # Panics
    ///
    /// Panics only if Canic's canonical `"default"` Component Spec identifier
    /// stops satisfying its declared ID admission.
    #[cfg(test)]
    #[must_use]
    pub fn test_default() -> Self {
        let mut cfg = Self::default();
        let default_component_spec_id = "default"
            .parse::<ComponentSpecId>()
            .expect("valid default Component Spec ID");
        let default_component_spec = ComponentSpecConfig {
            component_role: CanisterRole::from("app"),
            maximum_instances: 1,
            limits: ComponentLimitsConfig::default(),
            initial_cycles: crate::cdk::types::Cycles::new(0),
            topup: None,
            cycles_funding: CyclesFundingPolicyConfig::default(),
            scaling: None,
            sharding: None,
            binding: None,
            auth: CanisterAuthConfig::default(),
            standards: StandardsCanisterConfig::default(),
            diagnostics: DiagnosticsCanisterConfig::default(),
            metrics: MetricsCanisterConfig::default(),
            provisions: BTreeMap::new(),
            children: BTreeMap::new(),
        };

        cfg.app.name = AppId::from("test");
        cfg.auth.delegated_tokens.enabled = true;
        cfg.auth.delegated_tokens.build_network = BuildNetwork::Local;
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
        cfg.roles.insert(
            CanisterRole::from("app"),
            RoleDeclaration {
                kind: RoleDeclarationKind::Canister,
                package: "app".to_string(),
            },
        );
        cfg.component_specs
            .insert(default_component_spec_id, default_component_spec);
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
/// AppConfig
///
/// App identity, startup mode and optional whitelist configuration.
/// Owned by config schema and consumed by access/Fleet-state setup.
///

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AppConfig {
    pub name: AppId,

    #[serde(default)]
    pub init_mode: FleetInitMode,

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
            name: AppId::default(),
            init_mode: FleetInitMode::Enabled,
            whitelist: None,
        }
    }
}

///
/// FleetInitMode
///
/// Configurable initial Fleet state for an App installation.
/// Owned by config schema and mapped into Fleet runtime state during bootstrap.
///

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum FleetInitMode {
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
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests;
