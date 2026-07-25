//! Module: ops::config
//!
//! Responsibility: expose fallible configuration lookups for ops and workflows.
//! Does not own: config parsing, environment initialization, or endpoint DTOs.
//! Boundary: ops layer between runtime context and immutable configuration model.

use crate::{
    InternalError,
    config::{
        Config, ConfigError, ConfigModel,
        schema::{
            BindingConfig, CanisterConfig, DelegatedTokenConfig, FleetInitMode, LogConfig,
            RoleAttestationConfig, ScalingConfig, TreeSpecConfig,
        },
    },
    ids::{CanisterRole, TreeSpecId},
    model::cycles_funding::FundingLimits,
    ops::{OpsError, prelude::*, runtime::env::EnvOps},
    storage::stable::state::fleet::FleetMode,
};
use std::sync::Arc;
use thiserror::Error as ThisError;

///
/// ConfigOpsError
///
/// Typed failure surface for configuration lookup operations.
///

#[derive(Debug, ThisError)]
pub enum ConfigOpsError {
    #[error(transparent)]
    Config(#[from] ConfigError),

    #[error("Tree Spec {0} not found in configuration")]
    TreeSpecNotFound(String),

    #[error("canister {0} not defined in Tree Spec {1}")]
    CanisterNotFound(String, String),

    #[error(
        "current single-Tree bootstrap requires exactly one initial Tree; Coordinator installation is not active"
    )]
    InitialTreeAmbiguous,
}

impl From<ConfigOpsError> for InternalError {
    fn from(err: ConfigOpsError) -> Self {
        OpsError::from(err).into()
    }
}

///
/// ConfigOps
///
/// Operations-layer facade for configuration access.
///

pub struct ConfigOps;

impl ConfigOps {
    /// Export the full current configuration as TOML.
    /// Intended for diagnostics and tooling only.
    pub fn export_toml() -> Result<String, InternalError> {
        let toml = Config::to_toml()?;

        Ok(toml)
    }

    // ---------------------------------------------------------------------
    // Explicit / fallible lookups
    // ---------------------------------------------------------------------

    /// Fetch a Tree Spec configuration by declared identity.
    pub(crate) fn try_get_tree_spec(
        tree_spec: &TreeSpecId,
    ) -> Result<TreeSpecConfig, InternalError> {
        let cfg = Config::get()?;

        cfg.get_tree_spec(tree_spec)
            .ok_or_else(|| ConfigOpsError::TreeSpecNotFound(tree_spec.to_string()).into())
    }

    /// Fetch a canister configuration within a specific Tree Spec.
    pub(crate) fn try_get_canister(
        tree_spec: &TreeSpecId,
        canister_role: &CanisterRole,
    ) -> Result<CanisterConfig, InternalError> {
        let tree_spec_cfg = Self::try_get_tree_spec(tree_spec)?;

        tree_spec_cfg.get_canister(canister_role).ok_or_else(|| {
            ConfigOpsError::CanisterNotFound(canister_role.to_string(), tree_spec.to_string())
                .into()
        })
    }

    /// Return the only initial Tree Spec admitted by the current bootstrap path.
    pub(crate) fn sole_initial_tree_spec_id() -> Result<TreeSpecId, InternalError> {
        Config::get()?
            .sole_initial_tree_spec_id()
            .cloned()
            .ok_or_else(|| ConfigOpsError::InitialTreeAmbiguous.into())
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

    pub(crate) fn is_whitelisted(caller: &Principal) -> Result<bool, InternalError> {
        Ok(Config::get()?.is_whitelisted(caller))
    }

    pub(crate) fn log_config() -> Result<LogConfig, InternalError> {
        Ok(Config::get()?.log.clone())
    }

    pub(crate) fn delegated_tokens_config() -> Result<DelegatedTokenConfig, InternalError> {
        Ok(Config::get()?.auth.delegated_tokens.clone())
    }

    pub(crate) fn role_attestation_config() -> Result<RoleAttestationConfig, InternalError> {
        Ok(Config::get()?.auth.role_attestation.clone())
    }

    pub(crate) fn app_init_mode() -> Result<FleetMode, InternalError> {
        let mode = match Config::get()?.app.init_mode {
            FleetInitMode::Enabled => FleetMode::Enabled,
            FleetInitMode::Readonly => FleetMode::Readonly,
            FleetInitMode::Disabled => FleetMode::Disabled,
        };

        Ok(mode)
    }

    /// Fetch the configuration record for the current Tree.
    ///
    /// Requires that environment initialization has completed.
    pub fn current_tree_spec() -> Result<TreeSpecConfig, InternalError> {
        let tree_spec = EnvOps::tree_spec()?;

        Self::try_get_tree_spec(&tree_spec)
    }

    /// Fetch the configuration record for the *current* canister.
    pub(crate) fn current_canister() -> Result<CanisterConfig, InternalError> {
        let tree_spec = EnvOps::tree_spec()?;
        let canister_role = EnvOps::canister_role()?;

        Self::try_get_canister(&tree_spec, &canister_role)
    }

    /// Fetch the scaling configuration for the *current* canister.
    pub(crate) fn current_scaling_config() -> Result<Option<ScalingConfig>, InternalError> {
        Ok(Self::current_canister()?.scaling)
    }

    /// Fetch the binding placement config for the *current* canister.
    pub(crate) fn current_binding_config() -> Result<Option<BindingConfig>, InternalError> {
        Ok(Self::current_canister()?.binding)
    }

    /// Fetch the configuration for a specific canister in the current Tree.
    pub(crate) fn current_tree_canister(
        canister_role: &CanisterRole,
    ) -> Result<CanisterConfig, InternalError> {
        let tree_spec = EnvOps::tree_spec()?;

        Self::try_get_canister(&tree_spec, canister_role)
    }

    /// Resolve parent funding limits for a child role in the current Tree.
    pub(crate) fn cycles_funding_limits_for_child_role(
        child_role: &CanisterRole,
    ) -> Result<FundingLimits, InternalError> {
        let cfg = Self::current_tree_canister(child_role)?;
        let policy = cfg.cycles_funding;

        Ok(FundingLimits {
            max_per_request: policy.max_per_request.to_u128(),
            max_per_child: policy.max_per_child.to_u128(),
            cooldown_secs: policy.cooldown_secs,
        })
    }
}
