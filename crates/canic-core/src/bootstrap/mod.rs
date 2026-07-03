//! Module: bootstrap
//!
//! Responsibility: install compiled configuration and expose bootstrap artifacts.
//! Does not own: config schema validation rules, runtime lifecycle ordering, or artifact builds.
//! Boundary: lifecycle and build tooling call bootstrap after config generation.

#[cfg(any(not(target_arch = "wasm32"), test))]
mod render;

use crate::config::{Config, ConfigError, schema::ConfigModel};
#[cfg(any(target_arch = "wasm32", test))]
use crate::domain::auth::{
    DelegatedAuthNetwork, ic_root_public_key_raw_from_der_or_raw, is_mainnet_ic_root_public_key_raw,
};
#[cfg(any(not(target_arch = "wasm32"), test))]
use crate::{config::schema::CanisterConfig, ids::CanisterRole};
#[cfg(any(not(target_arch = "wasm32"), test))]
use std::collections::BTreeSet;
#[cfg(any(target_arch = "wasm32", test))]
use std::fmt::Write as _;
use std::sync::Arc;

pub const AUTH_ROOT_CANISTER_SIG_CREATE_FEATURE: &str = "auth-root-canister-sig-create";
pub const AUTH_ROOT_CANISTER_SIG_VERIFY_FEATURE: &str = "auth-root-canister-sig-verify";
pub const AUTH_ISSUER_CANISTER_SIG_CREATE_FEATURE: &str = "auth-issuer-canister-sig-create";
pub const AUTH_DELEGATED_TOKEN_VERIFY_FEATURE: &str = "auth-delegated-token-verify";

///
/// CanicFeatureRequirement
///
/// Build feature required by one validated role configuration.
///
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CanicFeatureRequirement {
    pub config_key: &'static str,
    pub feature: &'static str,
    pub reason: &'static str,
}

#[doc(hidden)]
pub mod compiled {
    pub use crate::{
        cdk::{candid::Principal, types::Cycles},
        config::schema::{
            AppConfig, AppInitMode, AuthConfig, CanisterAuthConfig, CanisterConfig, CanisterKind,
            CanisterPool, ChainKeyRootProofConfig, ConfigModel, CyclesFundingPolicyConfig,
            DelegatedTokenConfig, DiagnosticsCanisterConfig, DirectoryConfig, DirectoryPool,
            FleetConfig, IcpRefillPolicy, LogConfig, MetricsCanisterConfig, MetricsProfile,
            PoolImport, RandomnessConfig, RandomnessSource, RoleAttestationConfig, RoleDeclaration,
            RoleDeclarationKind, ScalePool, ScalePoolPolicy, ScalingConfig, ShardPool,
            ShardPoolPolicy, ShardingConfig, Standards, StandardsCanisterConfig, SubnetConfig,
            TopupPolicy, Whitelist,
        },
        ids::{CanisterRole, SubnetRole},
    };
}

/// EmbeddedRootBootstrapEntry
///
/// Metadata and bytes for a root bootstrap artifact embedded at build time.
/// Owned by bootstrap and consumed during generated install/start flows.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EmbeddedRootBootstrapEntry {
    pub role: &'static str,
    pub wasm_module: &'static [u8],
    pub artifact_path: &'static str,
    pub embedded_artifact_path: &'static str,
    pub artifact_kind: &'static str,
    pub artifact_size_bytes: u64,
    pub artifact_sha256_hex: &'static str,
    pub decompressed_size_bytes: Option<u64>,
    pub decompressed_sha256_hex: Option<&'static str>,
}

/// init_compiled_config
///
/// Install a build-produced configuration model and its canonical TOML source.
pub fn init_compiled_config(
    config: ConfigModel,
    source_toml: &str,
) -> Result<Arc<ConfigModel>, ConfigError> {
    #[cfg(target_arch = "wasm32")]
    let config = {
        let mut config = config;
        inject_runtime_ic_root_public_key(&mut config)?;
        config
    };
    Config::init_from_model(config, source_toml)
}

/// parse_config_model
///
/// Parse and validate the source TOML into a configuration model on host targets.
#[cfg(any(not(target_arch = "wasm32"), test))]
pub fn parse_config_model(toml: &str) -> Result<ConfigModel, ConfigError> {
    Config::parse_toml(toml)
}

/// Return the explicit `canic` crate features required by one package role.
#[cfg(any(not(target_arch = "wasm32"), test))]
#[must_use]
pub fn role_required_canic_features(
    config: &ConfigModel,
    role: &CanisterRole,
) -> Vec<CanicFeatureRequirement> {
    let mut requirements = Vec::new();
    let mut seen = BTreeSet::new();

    if role.is_root() && config_requires_root_role_attestation_signing(config) {
        push_requirement(
            &mut requirements,
            &mut seen,
            CanicFeatureRequirement {
                config_key: "auth.role_attestation_cache",
                feature: AUTH_ROOT_CANISTER_SIG_CREATE_FEATURE,
                reason: "root signs role-attestation canister-signature proofs for cache users",
            },
        );
    }

    if !role.is_root() {
        for subnet in config.subnets.values() {
            if let Some(canister) = subnet.get_canister(role) {
                push_canister_feature_requirements(&canister, &mut requirements, &mut seen);
            }
        }
    }

    requirements
}

#[cfg(any(not(target_arch = "wasm32"), test))]
fn push_canister_feature_requirements(
    canister: &CanisterConfig,
    requirements: &mut Vec<CanicFeatureRequirement>,
    seen: &mut BTreeSet<&'static str>,
) {
    if canister.auth.role_attestation_cache {
        push_requirement(
            requirements,
            seen,
            CanicFeatureRequirement {
                config_key: "auth.role_attestation_cache",
                feature: AUTH_ROOT_CANISTER_SIG_VERIFY_FEATURE,
                reason: "role-attestation cache verifies root canister-signature proofs locally",
            },
        );
    }

    if canister.auth.delegated_token_issuer {
        push_requirement(
            requirements,
            seen,
            CanicFeatureRequirement {
                config_key: "auth.delegated_token_issuer",
                feature: AUTH_ISSUER_CANISTER_SIG_CREATE_FEATURE,
                reason: "delegated-token issuers create issuer canister-signature proofs",
            },
        );
    }

    if canister.auth.delegated_token_issuer || canister.auth.delegated_token_verifier {
        let config_key = if canister.auth.delegated_token_issuer {
            "auth.delegated_token_issuer"
        } else {
            "auth.delegated_token_verifier"
        };
        push_requirement(
            requirements,
            seen,
            CanicFeatureRequirement {
                config_key,
                feature: AUTH_DELEGATED_TOKEN_VERIFY_FEATURE,
                reason: "delegated-token roles verify delegated-token root proof material",
            },
        );
    }
}

#[cfg(any(not(target_arch = "wasm32"), test))]
fn push_requirement(
    requirements: &mut Vec<CanicFeatureRequirement>,
    seen: &mut BTreeSet<&'static str>,
    requirement: CanicFeatureRequirement,
) {
    if seen.insert(requirement.feature) {
        requirements.push(requirement);
    }
}

#[cfg(any(not(target_arch = "wasm32"), test))]
fn config_requires_root_role_attestation_signing(config: &ConfigModel) -> bool {
    config.subnets.values().any(|subnet| {
        subnet
            .canisters
            .values()
            .any(|canister| canister.auth.role_attestation_cache)
    })
}

/// compact_config_source
///
/// Compact a validated Canic TOML source without changing value encodings.
#[cfg(any(not(target_arch = "wasm32"), test))]
#[must_use]
pub fn compact_config_source(toml: &str) -> String {
    let mut compact = String::new();

    for line in toml.lines() {
        let trimmed = line.trim();

        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        compact.push_str(trimmed);
        compact.push('\n');
    }

    compact
}

/// emit_config_model_source
///
/// Render the validated configuration model as Rust source for `include!` at runtime.
#[cfg(any(not(target_arch = "wasm32"), test))]
#[must_use]
pub fn emit_config_model_source(config: &ConfigModel) -> String {
    render::config_model(config)
}

#[cfg(target_arch = "wasm32")]
fn inject_runtime_ic_root_public_key(config: &mut ConfigModel) -> Result<(), ConfigError> {
    if should_inject_runtime_ic_root_public_key(config) {
        let root_key = crate::cdk::api::root_key();
        inject_runtime_ic_root_public_key_from(config, &root_key)?;
    }
    Ok(())
}

#[cfg(any(target_arch = "wasm32", test))]
fn inject_runtime_ic_root_public_key_from(
    config: &mut ConfigModel,
    root_key: &[u8],
) -> Result<(), ConfigError> {
    if !should_inject_runtime_ic_root_public_key(config) {
        return Ok(());
    }

    let network = DelegatedAuthNetwork::parse(config.auth.delegated_tokens.network.trim())
        .expect("validated delegated auth network");
    let raw_root_key =
        ic_root_public_key_raw_from_der_or_raw(root_key).map_err(ConfigError::RuntimeRootKey)?;
    if is_mainnet_ic_root_public_key_raw(&raw_root_key) {
        return Err(ConfigError::RuntimeRootKey(format!(
            "auth.delegated_tokens.network=\"{}\" must not use the mainnet IC root public key",
            network.label()
        )));
    }

    config.auth.delegated_tokens.ic_root_public_key_raw_hex = Some(hex_bytes(&raw_root_key));
    Ok(())
}

#[cfg(any(target_arch = "wasm32", test))]
fn should_inject_runtime_ic_root_public_key(config: &ConfigModel) -> bool {
    if !config.auth.delegated_tokens.enabled
        || config
            .auth
            .delegated_tokens
            .ic_root_public_key_raw_hex
            .is_some()
    {
        return false;
    }

    DelegatedAuthNetwork::parse(config.auth.delegated_tokens.network.trim())
        .is_some_and(|network| !network.is_mainnet())
}

#[cfg(any(target_arch = "wasm32", test))]
fn hex_bytes(bytes: &[u8]) -> String {
    let mut hex = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(&mut hex, "{byte:02x}");
    }
    hex
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::schema::{CanisterAuthConfig, CanisterKind},
        domain::auth::{IC_ROOT_PUBLIC_KEY_RAW_LENGTH, MAINNET_IC_ROOT_PUBLIC_KEY_RAW},
        test::config::ConfigTestBuilder,
    };

    #[test]
    fn runtime_root_key_injection_sets_local_missing_key() {
        let mut config = ConfigModel::test_default();
        config.auth.delegated_tokens.network = "local".to_string();

        inject_runtime_ic_root_public_key_from(&mut config, &[9; IC_ROOT_PUBLIC_KEY_RAW_LENGTH])
            .expect("local runtime root key should inject");

        let expected = hex_bytes(&[9; IC_ROOT_PUBLIC_KEY_RAW_LENGTH]);
        assert_eq!(
            config
                .auth
                .delegated_tokens
                .ic_root_public_key_raw_hex
                .as_deref(),
            Some(expected.as_str())
        );
    }

    #[test]
    fn runtime_root_key_injection_preserves_explicit_key() {
        let mut config = ConfigModel::test_default();
        config.auth.delegated_tokens.network = "local".to_string();
        config.auth.delegated_tokens.ic_root_public_key_raw_hex =
            Some(hex_bytes(&[8; IC_ROOT_PUBLIC_KEY_RAW_LENGTH]));

        inject_runtime_ic_root_public_key_from(&mut config, &[9; IC_ROOT_PUBLIC_KEY_RAW_LENGTH])
            .expect("explicit local runtime root key should be preserved");

        let expected = hex_bytes(&[8; IC_ROOT_PUBLIC_KEY_RAW_LENGTH]);
        assert_eq!(
            config
                .auth
                .delegated_tokens
                .ic_root_public_key_raw_hex
                .as_deref(),
            Some(expected.as_str())
        );
    }

    #[test]
    fn runtime_root_key_injection_leaves_mainnet_missing_key_unresolved() {
        let mut config = ConfigModel::test_default();
        config.auth.delegated_tokens.network = "mainnet".to_string();

        inject_runtime_ic_root_public_key_from(&mut config, &[9; IC_ROOT_PUBLIC_KEY_RAW_LENGTH])
            .expect("mainnet runtime root key must not be injected");

        assert!(
            config
                .auth
                .delegated_tokens
                .ic_root_public_key_raw_hex
                .is_none()
        );
    }

    #[test]
    fn runtime_root_key_injection_rejects_mainnet_key_for_local() {
        let mut config = ConfigModel::test_default();
        config.auth.delegated_tokens.network = "local".to_string();

        let err =
            inject_runtime_ic_root_public_key_from(&mut config, &MAINNET_IC_ROOT_PUBLIC_KEY_RAW)
                .expect_err("local runtime root key must not accept mainnet key");

        assert!(
            err.to_string()
                .contains("must not use the mainnet IC root public key"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn role_required_canic_features_maps_role_attestation_cache() {
        let mut app = ConfigTestBuilder::canister_config(CanisterKind::Service);
        app.auth = CanisterAuthConfig {
            delegated_token_issuer: false,
            delegated_token_verifier: false,
            role_attestation_cache: true,
        };
        let config = ConfigTestBuilder::new()
            .with_prime_canister(
                CanisterRole::ROOT,
                ConfigTestBuilder::canister_config(CanisterKind::Root),
            )
            .with_prime_canister("app", app)
            .build();

        let app_requirements =
            role_required_canic_features(&config, &CanisterRole::owned("app".to_string()));
        assert_eq!(
            app_requirements
                .iter()
                .map(|requirement| requirement.feature)
                .collect::<Vec<_>>(),
            vec![AUTH_ROOT_CANISTER_SIG_VERIFY_FEATURE]
        );

        let root_requirements = role_required_canic_features(&config, &CanisterRole::ROOT);
        assert_eq!(
            root_requirements
                .iter()
                .map(|requirement| requirement.feature)
                .collect::<Vec<_>>(),
            vec![AUTH_ROOT_CANISTER_SIG_CREATE_FEATURE]
        );
    }

    #[test]
    fn role_required_canic_features_maps_delegated_token_roles() {
        let mut issuer = ConfigTestBuilder::canister_config(CanisterKind::Shard);
        issuer.auth = CanisterAuthConfig {
            delegated_token_issuer: true,
            delegated_token_verifier: false,
            role_attestation_cache: false,
        };
        let mut verifier = ConfigTestBuilder::canister_config(CanisterKind::Service);
        verifier.auth = CanisterAuthConfig {
            delegated_token_issuer: false,
            delegated_token_verifier: true,
            role_attestation_cache: false,
        };
        let config = ConfigTestBuilder::new()
            .with_prime_canister(
                CanisterRole::ROOT,
                ConfigTestBuilder::canister_config(CanisterKind::Root),
            )
            .with_prime_canister("issuer", issuer)
            .with_prime_canister("verifier", verifier)
            .build();

        let issuer_requirements =
            role_required_canic_features(&config, &CanisterRole::owned("issuer".to_string()));
        assert_eq!(
            issuer_requirements
                .iter()
                .map(|requirement| requirement.feature)
                .collect::<Vec<_>>(),
            vec![
                AUTH_ISSUER_CANISTER_SIG_CREATE_FEATURE,
                AUTH_DELEGATED_TOKEN_VERIFY_FEATURE,
            ]
        );

        let verifier_requirements =
            role_required_canic_features(&config, &CanisterRole::owned("verifier".to_string()));
        assert_eq!(
            verifier_requirements
                .iter()
                .map(|requirement| requirement.feature)
                .collect::<Vec<_>>(),
            vec![AUTH_DELEGATED_TOKEN_VERIFY_FEATURE]
        );
    }
}
