use crate::config::{Config, ConfigError, schema::ConfigModel};
use std::sync::Arc;

#[cfg(any(not(target_arch = "wasm32"), test))]
mod render;

#[doc(hidden)]
pub mod compiled {
    pub use crate::{
        cdk::{candid::Principal, types::Cycles},
        config::schema::{
            AppConfig, AppInitMode, AuthConfig, CanisterConfig, CanisterKind, CanisterPool,
            ConfigModel, DelegatedAuthCanisterConfig, DelegatedTokenConfig,
            DelegationProofCacheConfig, DelegationProofCacheProfile, LogConfig, PoolImport,
            RandomnessConfig, RandomnessSource, RoleAttestationConfig, ScalePool, ScalePoolPolicy,
            ScalingConfig, ShardPool, ShardPoolPolicy, ShardingConfig, Standards,
            StandardsCanisterConfig, SubnetConfig, TopupPolicy, Whitelist,
        },
        ids::{CanisterRole, SubnetRole},
    };
}

///
/// EmbeddedRootReleaseEntry
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EmbeddedRootReleaseEntry {
    pub role: &'static str,
    pub wasm_module: &'static [u8],
}

///
/// EmbeddedRootBootstrapEntry
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EmbeddedRootBootstrapEntry {
    pub role: &'static str,
    pub wasm_module: &'static [u8],
    pub artifact_path: &'static str,
    pub artifact_kind: &'static str,
    pub artifact_size_bytes: u64,
    pub artifact_sha256_hex: &'static str,
    pub decompressed_size_bytes: Option<u64>,
    pub decompressed_sha256_hex: Option<&'static str>,
}

// Install a build-produced configuration model and its canonical TOML source.
pub fn init_compiled_config(
    config: ConfigModel,
    source_toml: &str,
) -> Result<Arc<ConfigModel>, ConfigError> {
    Config::init_from_model(config, source_toml)
}

// Parse and validate the source TOML into a configuration model on host targets.
#[cfg(any(not(target_arch = "wasm32"), test))]
pub fn parse_config_model(toml: &str) -> Result<ConfigModel, ConfigError> {
    Config::parse_toml(toml)
}

// Compact a validated Canic TOML source without changing value encodings.
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

// Render the validated configuration model as Rust source for `include!` at runtime.
#[cfg(any(not(target_arch = "wasm32"), test))]
#[must_use]
pub fn emit_config_model_source(config: &ConfigModel) -> String {
    render::config_model(config)
}
