mod cfg_catalog;
mod config;
mod metrics;

pub use cfg_catalog::CANIC_CUSTOM_CFG_NAMES;
pub use config::{
    assert_canonical_role_contract_build, compile_role_build_sources, config_app_id,
    config_contains_role, config_declares_role, declared_package_metadata, declared_package_role,
    read_config_source_or_default, required_package_metadata, required_package_role,
};
pub use metrics::{
    METRICS_TIER_CORE, METRICS_TIER_PLACEMENT, METRICS_TIER_PLATFORM, METRICS_TIER_RUNTIME,
    METRICS_TIER_SECURITY, METRICS_TIER_STORAGE, configured_role_metrics_tier_mask,
    metrics_profile_tier_mask,
};
