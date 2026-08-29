//! Module: bootstrap
//!
//! Responsibility: install compiled configuration and expose bootstrap artifacts.
//! Does not own: config schema validation rules, runtime lifecycle ordering, or artifact builds.
//! Boundary: lifecycle and build tooling call bootstrap after config generation.

#[cfg(any(not(target_arch = "wasm32"), test))]
mod render;

#[cfg(any(target_arch = "wasm32", test))]
use crate::cdk::utils::hash::hex_bytes;
use crate::config::{Config, RoleRuntimeAuthority, RoleRuntimeConfig, schema::ConfigModel};
#[cfg(any(target_arch = "wasm32", test))]
use crate::domain::auth::{
    ic_root_public_key_raw_from_der_or_raw, is_mainnet_ic_root_public_key_raw,
};
#[cfg(any(target_arch = "wasm32", test))]
use crate::ids::BuildNetwork;
use std::sync::Arc;

#[doc(hidden)]
pub use crate::config::{ConfigError, ConfigTomlIssue};

#[doc(hidden)]
pub mod compiled {
    pub use crate::config::{
        ComponentChildFundingPolicy, ComponentChildSpec, ComponentDeploymentConfiguration,
        ComponentDeploymentConfigurationDigestError, ComponentDeploymentLabel,
        ComponentDeploymentLabelKey, ComponentDeploymentLabelParseError,
        ComponentDeploymentLabelValue, ComponentDeploymentLimits, ComponentDeploymentMemberLimit,
        ComponentDeploymentMemberLimitError, ComponentDeploymentPurpose,
        ComponentDeploymentSpawnGrantLimit, ComponentGroupDeploymentSpec,
        ComponentGroupDeploymentTopology, ComponentGroupDeploymentTopologyError,
        ComponentGroupLeafKind, ComponentGroupMember, ComponentGroupPlacementPolicy,
        ComponentGroupSpec, ComponentGroupTopology, ComponentGroupTopologyError, ComponentLimits,
        ComponentProvisioningGrant, ComponentSpawnGrant, ComponentSpec, ComponentTopology,
        ComponentTopologyError, FlattenedComponentGroup, FlattenedComponentGroupDeploymentMember,
        FlattenedComponentGroupMember, FleetServiceMemberPurpose, FleetServicePlacementPolicy,
        FleetServiceTarget, FleetServiceTargetMode, FleetServiceTopology,
        FleetServiceTopologyError, MAX_COMPONENT_DEPLOYMENT_CONFIGURATION_CANONICAL_BYTES,
        MAX_COMPONENT_DEPLOYMENT_LABEL_KEY_BYTES, MAX_COMPONENT_DEPLOYMENT_LABEL_VALUE_BYTES,
        MAX_COMPONENT_DEPLOYMENT_LABELS, MAX_COMPONENT_DEPLOYMENT_MEMBER_LIMITS,
        MAX_COMPONENT_DEPLOYMENT_SPAWN_GRANT_REDUCTIONS, MAX_COMPONENT_GROUP_DECLARED_MEMBERS,
        MAX_COMPONENT_GROUP_DEPLOYMENT_MEMBERS,
        MAX_COMPONENT_GROUP_DEPLOYMENT_TOPOLOGY_CANONICAL_BYTES, MAX_COMPONENT_GROUP_DEPLOYMENTS,
        MAX_COMPONENT_GROUP_FLATTENED_MEMBERS, MAX_COMPONENT_GROUP_GRAPH_CANONICAL_BYTES,
        MAX_COMPONENT_GROUP_INCLUSIONS, MAX_COMPONENT_GROUP_MEMBERS, MAX_COMPONENT_GROUP_SPECS,
        MAX_COMPONENT_TOPOLOGY_CANONICAL_BYTES, MAX_FLEET_SERVICE_TARGETS,
        MAX_FLEET_SERVICE_TOPOLOGY_CANONICAL_BYTES, RoleRuntimeAuthority,
        RuntimeApplicationAuthorization, RuntimeCanisterAuthority, RuntimeCanisterConfig,
        RuntimeChildCanisterAuthority, RuntimeDeploymentMemberAuthority,
    };
    pub use crate::{
        cdk::{candid::Principal, types::Cycles},
        config::schema::{
            AppConfig, AuthConfig, CanisterAuthConfig, CanisterConfig, CanisterKind,
            CanisterRoleNameIssue, ChainKeyRootProofConfig, ComponentChildConfig,
            ComponentChildKind, ComponentDeploymentMemberLimitConfig,
            ComponentDeploymentSpawnGrantLimitConfig, ComponentGroupComponentConfig,
            ComponentGroupDeploymentConfig, ComponentGroupIncludeConfig,
            ComponentGroupPlacementPolicyConfig, ComponentGroupSpecConfig, ComponentLimitsConfig,
            ComponentProvisioningGrantConfig, ComponentSpawnGrantConfig, ComponentSpecConfig,
            ConfigModel, CyclesFundingBudgetConfig, CyclesFundingPolicyConfig,
            DelegatedTokenConfig, DiagnosticsCanisterConfig, FleetInitMode,
            FleetServicePlacementPolicyConfig, FleetServiceTargetConfig, FleetServicesConfig,
            IndexConfig, IndexPool, LocalApplicationAuthorizationConfig, LogConfig,
            MAX_COMPONENT_CHILD_ROLES, MAX_COMPONENT_PROVISIONING_GRANTS,
            MAX_COMPONENT_SPAWN_GRANTS, MAX_FLEET_COMPONENT_INSTANCES, MetricsCanisterConfig,
            MetricsProfile, NAME_MAX_BYTES, RoleAttestationConfig, RoleDeclaration,
            RoleDeclarationKind, ScalePool, ScalePoolPolicy, ScalingConfig, ServicesConfig,
            ShardPool, ShardPoolPolicy, ShardingConfig, Standards, StandardsCanisterConfig,
            TopupPolicy, implicit_root_canister_config, implicit_wasm_store_canister_config,
            validate_app_name, validate_canister_role_name,
        },
        ids::{
            AppId, BuildNetwork, CanisterRole, ComponentDeploymentConfigurationDigest,
            ComponentGroupDeploymentId, ComponentGroupMemberId, ComponentGroupMemberPath,
            ComponentGroupSpecId, ComponentSpecId, CyclesFundingBudget, FleetServiceId,
        },
    };
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

/// Install one build-compiled immutable role runtime authority.
pub fn init_role_runtime_authority(
    expected_role: &crate::ids::CanisterRole,
    authority: RoleRuntimeAuthority,
) -> Result<Arc<RoleRuntimeAuthority>, crate::InternalError> {
    if &authority.role != expected_role {
        return Err(crate::InternalError::invariant());
    }
    #[cfg(target_arch = "wasm32")]
    let authority = {
        let mut authority = authority;
        inject_runtime_ic_root_public_key_into_auth(&mut authority.auth)
            .map_err(crate::InternalError::from)?;
        authority
    };
    RoleRuntimeConfig::init(authority)
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

/// Render one validated role runtime authority as Rust source for `include!`.
#[cfg(any(not(target_arch = "wasm32"), test))]
pub fn emit_role_runtime_authority_source(
    config: &ConfigModel,
    role: &crate::ids::CanisterRole,
    wasm_store: bool,
) -> Result<String, crate::config::RoleRuntimeAuthorityError> {
    let authority = if wasm_store {
        crate::config::RoleRuntimeAuthority::compile_wasm_store(config)?
    } else {
        crate::config::RoleRuntimeAuthority::compile(config, role)?
    };
    Ok(render::role_runtime_authority(&authority))
}

#[cfg(target_arch = "wasm32")]
fn inject_runtime_ic_root_public_key(config: &mut ConfigModel) -> Result<(), ConfigError> {
    inject_runtime_ic_root_public_key_into_auth(&mut config.auth)
}

#[cfg(target_arch = "wasm32")]
fn inject_runtime_ic_root_public_key_into_auth(
    auth: &mut crate::config::schema::AuthConfig,
) -> Result<(), ConfigError> {
    if !should_inject_runtime_ic_root_public_key_for_auth(auth) {
        return Ok(());
    }
    let root_key = ic_cdk::api::root_key();
    inject_runtime_ic_root_public_key_into_auth_from(auth, &root_key)
}

#[cfg(test)]
fn inject_runtime_ic_root_public_key_from(
    config: &mut ConfigModel,
    root_key: &[u8],
) -> Result<(), ConfigError> {
    inject_runtime_ic_root_public_key_into_auth_from(&mut config.auth, root_key)
}

#[cfg(any(target_arch = "wasm32", test))]
fn inject_runtime_ic_root_public_key_into_auth_from(
    auth: &mut crate::config::schema::AuthConfig,
    root_key: &[u8],
) -> Result<(), ConfigError> {
    if !should_inject_runtime_ic_root_public_key_for_auth(auth) {
        return Ok(());
    }

    let build_network = auth.delegated_tokens.build_network;
    let raw_root_key =
        ic_root_public_key_raw_from_der_or_raw(root_key).map_err(ConfigError::RuntimeRootKey)?;
    if is_mainnet_ic_root_public_key_raw(&raw_root_key) {
        return Err(ConfigError::RuntimeRootKey(format!(
            "auth.delegated_tokens.build_network=\"{build_network}\" must not use the mainnet IC root public key"
        )));
    }

    auth.delegated_tokens.ic_root_public_key_raw_hex = Some(hex_bytes(&raw_root_key));
    Ok(())
}

#[cfg(any(target_arch = "wasm32", test))]
fn should_inject_runtime_ic_root_public_key_for_auth(
    auth: &crate::config::schema::AuthConfig,
) -> bool {
    if !auth.delegated_tokens.enabled || auth.delegated_tokens.ic_root_public_key_raw_hex.is_some()
    {
        return false;
    }

    auth.delegated_tokens.build_network == BuildNetwork::Local
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::auth::{IC_ROOT_PUBLIC_KEY_RAW_LENGTH, MAINNET_IC_ROOT_PUBLIC_KEY_RAW};

    const MINIMAL_CONFIG: &str = r#"
[app]
name = "probe"

[roles.root]
kind = "root"
package = "root"

[roles.app]
kind = "canister"
package = "app"

[component_specs.default]
component_role = "app"
maximum_instances = 1
"#;

    #[test]
    fn strict_schema_accepts_current_canister_fields() {
        parse_config_model(MINIMAL_CONFIG).expect("current config should parse");
    }

    #[test]
    fn role_runtime_source_contains_only_the_compiled_runtime_authority() {
        let config = parse_config_model(MINIMAL_CONFIG).expect("current config should parse");
        let role = crate::ids::CanisterRole::from("app");
        let source = emit_role_runtime_authority_source(&config, &role, false)
            .expect("compile role runtime authority");

        assert!(source.contains("RoleRuntimeAuthority"));
        assert!(source.contains("RuntimeCanisterConfig"));
        assert!(!source.contains("ConfigModel"));
        assert!(!source.contains("ComponentSpecConfig"));
        assert!(!source.contains("initial_cycles"));

        let authority = crate::config::RoleRuntimeAuthority::compile(&config, &role)
            .expect("compile role runtime authority");
        assert!(
            init_role_runtime_authority(
                &crate::ids::CanisterRole::from("other"),
                authority.clone()
            )
            .is_err()
        );
        assert!(crate::config::RoleRuntimeConfig::try_get().is_none());
        let installed =
            init_role_runtime_authority(&role, authority).expect("install exact role authority");
        assert_eq!(installed.role, role);
        crate::config::Config::reset_for_tests();
    }

    #[test]
    fn strict_schema_reports_typed_nested_unknown_field() {
        let source =
            format!("{MINIMAL_CONFIG}\n[component_specs.default.randomness]\nenabled = true\n");
        let error = parse_config_model(&source).expect_err("unknown field must reject");

        assert!(matches!(
            error,
            ConfigError::CannotParseToml {
                issue: ConfigTomlIssue::UnknownField {
                    logical_path,
                    unknown_field,
                },
                ..
            } if logical_path == "component_specs.default.randomness"
                && unknown_field == "randomness"
        ));
    }

    #[test]
    fn protected_fleet_funding_policy_is_not_application_config_authority() {
        let source = format!(
            "{MINIMAL_CONFIG}\n[component_specs.default.icp_refill]\nmax_refill_e8s_per_call = 1\n"
        );
        let error =
            parse_config_model(&source).expect_err("Fleet policy must reject in canic.toml");

        assert!(matches!(
            error,
            ConfigError::CannotParseToml {
                issue: ConfigTomlIssue::UnknownField { unknown_field, .. },
                ..
            } if unknown_field == "icp_refill"
        ));
    }

    #[test]
    fn runtime_root_key_injection_sets_local_missing_key() {
        let mut config = ConfigModel::test_default();
        config.auth.delegated_tokens.build_network = BuildNetwork::Local;

        inject_runtime_ic_root_public_key_from(&mut config, &[9; IC_ROOT_PUBLIC_KEY_RAW_LENGTH])
            .expect("local runtime root key should inject");

        let expected = hex_bytes([9; IC_ROOT_PUBLIC_KEY_RAW_LENGTH]);
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
        config.auth.delegated_tokens.build_network = BuildNetwork::Local;
        config.auth.delegated_tokens.ic_root_public_key_raw_hex =
            Some(hex_bytes([8; IC_ROOT_PUBLIC_KEY_RAW_LENGTH]));

        inject_runtime_ic_root_public_key_from(&mut config, &[9; IC_ROOT_PUBLIC_KEY_RAW_LENGTH])
            .expect("explicit local runtime root key should be preserved");

        let expected = hex_bytes([8; IC_ROOT_PUBLIC_KEY_RAW_LENGTH]);
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
    fn runtime_root_key_injection_leaves_ic_missing_key_unresolved() {
        let mut config = ConfigModel::test_default();
        config.auth.delegated_tokens.build_network = BuildNetwork::Ic;

        inject_runtime_ic_root_public_key_from(&mut config, &[9; IC_ROOT_PUBLIC_KEY_RAW_LENGTH])
            .expect("IC runtime root key must not be injected");

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
        config.auth.delegated_tokens.build_network = BuildNetwork::Local;

        inject_runtime_ic_root_public_key_from(&mut config, &MAINNET_IC_ROOT_PUBLIC_KEY_RAW)
            .expect_err("local runtime root key must not accept mainnet key");
    }
}
