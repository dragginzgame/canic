use crate::{
    InternalError,
    config::{
        Config, ConfigError, ConfigModel,
        schema::{
            AppInitMode, CanisterConfig, DelegatedTokenConfig, DelegationProofCacheProfile,
            LogConfig, RoleAttestationConfig, ScalingConfig, SubnetConfig, WasmStoreConfig,
        },
    },
    ids::{CanisterRole, SubnetRole, WasmStoreBinding},
    ops::{
        OpsError, ic::IcOps, prelude::*, runtime::env::EnvOps,
        storage::state::subnet::SubnetStateOps,
    },
    storage::stable::state::app::AppMode,
};
use std::sync::Arc;
use thiserror::Error as ThisError;

const IMPLICIT_WASM_STORE_ROLE: CanisterRole = CanisterRole::WASM_STORE;
///
/// ConfigOpsError
///

#[derive(Debug, ThisError)]
pub enum ConfigOpsError {
    #[error(transparent)]
    Config(#[from] ConfigError),

    #[error("subnet {0} not found in configuration")]
    SubnetNotFound(String),

    #[error("canister {0} not defined in subnet {1}")]
    CanisterNotFound(String, String),

    #[error("current canister {0} is not configured as a wasm store")]
    CurrentCanisterNotWasmStore(String),

    #[error("wasm store binding {0} not configured for subnet {1}")]
    WasmStoreBindingNotConfigured(String, String),
}

impl From<ConfigOpsError> for InternalError {
    fn from(err: ConfigOpsError) -> Self {
        OpsError::from(err).into()
    }
}

///
/// DelegationProofCachePolicy
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DelegationProofCachePolicy {
    pub profile: DelegationProofCacheProfile,
    pub capacity: usize,
    pub active_window_secs: u64,
}

///
/// ConfigOps
///
/// Ops-layer façade for configuration access.
///
/// Responsibilities:
/// - Provide fallible lookups over the configuration model (`try_get_*`)
/// - Provide infallible access to the *current* subnet/canister context,
///   assuming environment initialization has completed
///

pub struct ConfigOps;

impl ConfigOps {
    fn is_wasm_store_role(role: &CanisterRole) -> bool {
        *role == IMPLICIT_WASM_STORE_ROLE
    }

    /// Export the full current configuration as TOML.
    /// Intended for diagnostics and tooling only.
    pub fn export_toml() -> Result<String, InternalError> {
        let toml = Config::to_toml()?;

        Ok(toml)
    }

    // ---------------------------------------------------------------------
    // Explicit / fallible lookups
    // ---------------------------------------------------------------------

    /// Fetch a subnet configuration by role.
    pub(crate) fn try_get_subnet(role: &SubnetRole) -> Result<SubnetConfig, InternalError> {
        let cfg = Config::get()?;

        cfg.get_subnet(role)
            .ok_or_else(|| ConfigOpsError::SubnetNotFound(role.to_string()).into())
    }

    /// Fetch a canister configuration within a specific subnet.
    pub(crate) fn try_get_canister(
        subnet_role: &SubnetRole,
        canister_role: &CanisterRole,
    ) -> Result<CanisterConfig, InternalError> {
        let subnet_cfg = Self::try_get_subnet(subnet_role)?;

        subnet_cfg.get_canister(canister_role).ok_or_else(|| {
            ConfigOpsError::CanisterNotFound(canister_role.to_string(), subnet_role.to_string())
                .into()
        })
    }

    // ---------------------------------------------------------------------
    // Current-context / infallible helpers
    // ---------------------------------------------------------------------

    pub(crate) fn get() -> Result<Arc<ConfigModel>, InternalError> {
        let cfg = Config::get()?;

        Ok(cfg)
    }

    pub(crate) fn controllers() -> Result<Vec<Principal>, InternalError> {
        Ok(Config::get()?.controllers.clone())
    }

    pub(crate) fn log_config() -> Result<LogConfig, InternalError> {
        Ok(Config::get()?.log.clone())
    }

    pub(crate) fn delegated_tokens_config() -> Result<DelegatedTokenConfig, InternalError> {
        Ok(Config::get()?.auth.delegated_tokens.clone())
    }

    pub(crate) fn delegation_proof_cache_policy()
    -> Result<DelegationProofCachePolicy, InternalError> {
        let cfg = Self::delegated_tokens_config()?;
        let profile = cfg.proof_cache.resolved_profile();

        Ok(DelegationProofCachePolicy {
            profile,
            capacity: cfg.proof_cache.resolved_capacity(),
            active_window_secs: u64::from(cfg.proof_cache.active_window_secs),
        })
    }

    pub(crate) fn role_attestation_config() -> Result<RoleAttestationConfig, InternalError> {
        Ok(Config::get()?.auth.role_attestation.clone())
    }

    pub(crate) fn app_init_mode() -> Result<AppMode, InternalError> {
        let mode = match Config::get()?.app.init_mode {
            AppInitMode::Enabled => AppMode::Enabled,
            AppInitMode::Readonly => AppMode::Readonly,
            AppInitMode::Disabled => AppMode::Disabled,
        };

        Ok(mode)
    }

    /// Fetch the configuration record for the *current* subnet.
    ///
    /// Requires that environment initialization has completed.
    pub(crate) fn current_subnet() -> Result<SubnetConfig, InternalError> {
        let subnet_role = EnvOps::subnet_role()?;

        Self::try_get_subnet(&subnet_role)
    }

    /// Fetch the configuration record for the *current* canister.
    pub(crate) fn current_canister() -> Result<CanisterConfig, InternalError> {
        let subnet_role = EnvOps::subnet_role()?;
        let canister_role = EnvOps::canister_role()?;

        Self::try_get_canister(&subnet_role, &canister_role)
    }

    /// Fetch the scaling configuration for the *current* canister.
    pub(crate) fn current_scaling_config() -> Result<Option<ScalingConfig>, InternalError> {
        Ok(Self::current_canister()?.scaling)
    }

    /// Fetch the configuration for a specific canister in the *current* subnet.
    pub(crate) fn current_subnet_canister(
        canister_role: &CanisterRole,
    ) -> Result<CanisterConfig, InternalError> {
        let subnet_role = EnvOps::subnet_role()?;

        Self::try_get_canister(&subnet_role, canister_role)
    }

    /// Return the default wasm-store binding for the current subnet.
    pub(crate) fn current_subnet_default_wasm_store_binding() -> WasmStoreBinding {
        SubnetStateOps::publication_store_binding()
            .filter(|binding| SubnetStateOps::wasm_store_pid(binding).is_some())
            .or_else(|| {
                SubnetStateOps::wasm_stores()
                    .into_iter()
                    .min_by(|left, right| left.created_at.cmp(&right.created_at))
                    .map(|record| record.binding)
            })
            .unwrap_or_else(|| WasmStoreBinding::new("primary"))
    }

    /// Return the configured binding for one wasm-store canister role in the current subnet.
    pub(crate) fn current_subnet_wasm_store_binding_for_role(
        role: &CanisterRole,
    ) -> Result<WasmStoreBinding, InternalError> {
        let subnet_role = EnvOps::subnet_role()?;
        Err(ConfigOpsError::WasmStoreBindingNotConfigured(
            role.to_string(),
            subnet_role.to_string(),
        )
        .into())
    }

    /// Return the wasm-store config for the current canister.
    pub(crate) fn current_wasm_store() -> Result<WasmStoreConfig, InternalError> {
        let canister_role = EnvOps::canister_role()?;

        if Self::is_wasm_store_role(&canister_role) {
            Ok(WasmStoreConfig::implicit())
        } else {
            Err(ConfigOpsError::CurrentCanisterNotWasmStore(canister_role.to_string()).into())
        }
    }

    /// Return the logical binding for the current wasm-store canister.
    pub(crate) fn current_wasm_store_binding() -> Result<WasmStoreBinding, InternalError> {
        let canister_role = EnvOps::canister_role()?;

        if Self::is_wasm_store_role(&canister_role) {
            let pid = IcOps::canister_self();
            Ok(SubnetStateOps::wasm_store_binding_for_pid(pid)
                .unwrap_or_else(|| WasmStoreBinding::owned(pid.to_text())))
        } else {
            Self::current_subnet_wasm_store_binding_for_role(&canister_role)
        }
    }
}
