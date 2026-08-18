use canic_core::bootstrap::compiled::{
    CanisterRole, ConfigModel, MetricsProfile, implicit_root_canister_config,
    implicit_wasm_store_canister_config,
};

pub const METRICS_TIER_CORE: u8 = 1 << 0;
pub const METRICS_TIER_PLACEMENT: u8 = 1 << 1;
pub const METRICS_TIER_PLATFORM: u8 = 1 << 2;
pub const METRICS_TIER_RUNTIME: u8 = 1 << 3;
pub const METRICS_TIER_SECURITY: u8 = 1 << 4;
pub const METRICS_TIER_STORAGE: u8 = 1 << 5;

/// Resolve the union of metrics tiers compiled for one configured or built-in role.
#[must_use]
pub fn configured_role_metrics_tier_mask(config: &ConfigModel, role: &CanisterRole) -> u8 {
    if role.is_root() {
        return metrics_profile_tier_mask(
            implicit_root_canister_config().resolved_metrics_profile(role),
        );
    }

    if role.is_wasm_store() {
        return metrics_profile_tier_mask(
            implicit_wasm_store_canister_config().resolved_metrics_profile(role),
        );
    }

    config
        .component_specs
        .values()
        .filter_map(|component_spec| component_spec.get_canister(role))
        .fold(0, |mask, canister| {
            mask | metrics_profile_tier_mask(canister.resolved_metrics_profile(role))
        })
}

#[must_use]
pub const fn metrics_profile_tier_mask(profile: MetricsProfile) -> u8 {
    match profile {
        MetricsProfile::Leaf => METRICS_TIER_CORE | METRICS_TIER_RUNTIME | METRICS_TIER_SECURITY,
        MetricsProfile::Hub => {
            METRICS_TIER_CORE
                | METRICS_TIER_PLACEMENT
                | METRICS_TIER_RUNTIME
                | METRICS_TIER_SECURITY
        }
        MetricsProfile::Storage => METRICS_TIER_CORE | METRICS_TIER_RUNTIME | METRICS_TIER_STORAGE,
        MetricsProfile::Root | MetricsProfile::Full => {
            METRICS_TIER_CORE
                | METRICS_TIER_PLACEMENT
                | METRICS_TIER_PLATFORM
                | METRICS_TIER_RUNTIME
                | METRICS_TIER_SECURITY
                | METRICS_TIER_STORAGE
        }
    }
}

#[cfg(test)]
mod tests {
    use canic_core::bootstrap::{compiled::CanisterRole, parse_config_model};

    use super::{
        METRICS_TIER_CORE, METRICS_TIER_PLACEMENT, METRICS_TIER_PLATFORM, METRICS_TIER_RUNTIME,
        METRICS_TIER_SECURITY, METRICS_TIER_STORAGE, configured_role_metrics_tier_mask,
    };

    const ALL_METRICS_TIERS: u8 = METRICS_TIER_CORE
        | METRICS_TIER_PLACEMENT
        | METRICS_TIER_PLATFORM
        | METRICS_TIER_RUNTIME
        | METRICS_TIER_SECURITY
        | METRICS_TIER_STORAGE;

    #[test]
    fn built_in_root_and_store_metrics_do_not_depend_on_component_specs() {
        let config = parse_config_model(
            r#"
                [app]
                name = "test"

                [roles.root]
                kind = "root"
                package = "../root"
            "#,
        )
        .expect("minimal root config parses");

        assert_eq!(
            configured_role_metrics_tier_mask(&config, &CanisterRole::ROOT),
            ALL_METRICS_TIERS
        );
        assert_eq!(
            configured_role_metrics_tier_mask(&config, &CanisterRole::WASM_STORE),
            METRICS_TIER_CORE | METRICS_TIER_RUNTIME | METRICS_TIER_STORAGE
        );
    }
}
