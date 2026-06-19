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
#[cfg(any(target_arch = "wasm32", test))]
use std::fmt::Write as _;
use std::sync::Arc;

#[doc(hidden)]
pub mod compiled {
    pub use crate::{
        cdk::{candid::Principal, types::Cycles},
        config::schema::{
            AppConfig, AppInitMode, AuthConfig, CanisterAuthConfig, CanisterConfig, CanisterKind,
            CanisterPool, ConfigModel, DelegatedTokenConfig, DiagnosticsCanisterConfig,
            DirectoryConfig, DirectoryPool, FleetConfig, IcpRefillPolicy, LogConfig,
            MetricsCanisterConfig, MetricsProfile, PoolImport, RandomnessConfig, RandomnessSource,
            RoleAttestationConfig, RoleDeclaration, RoleDeclarationKind, ScalePool,
            ScalePoolPolicy, ScalingConfig, ShardPool, ShardPoolPolicy, ShardingConfig, Standards,
            StandardsCanisterConfig, SubnetConfig, TopupPolicy, Whitelist,
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
    use crate::domain::auth::{IC_ROOT_PUBLIC_KEY_RAW_LENGTH, MAINNET_IC_ROOT_PUBLIC_KEY_RAW};

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
}
