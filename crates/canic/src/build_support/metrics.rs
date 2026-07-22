use std::{fs, path::Path};

use canic_core::bootstrap::compiled::MetricsProfile;
use toml::Value as TomlValue;

pub const METRICS_TIER_CORE: u8 = 1 << 0;
pub const METRICS_TIER_PLACEMENT: u8 = 1 << 1;
pub const METRICS_TIER_PLATFORM: u8 = 1 << 2;
pub const METRICS_TIER_RUNTIME: u8 = 1 << 3;
pub const METRICS_TIER_SECURITY: u8 = 1 << 4;
pub const METRICS_TIER_STORAGE: u8 = 1 << 5;

/// Return whether the role package's normal Canic dependency enables metrics.
///
/// # Panics
///
/// Panics when the role manifest cannot be read or its normal `canic`
/// dependency does not use the canonical explicit-feature shape.
#[must_use]
pub fn role_normal_dependency_metrics_enabled(manifest_dir: &Path) -> bool {
    let source = fs::read_to_string(manifest_dir.join("Cargo.toml"))
        .unwrap_or_else(|err| panic!("failed to read role Cargo.toml: {err}"));

    parse_role_normal_dependency_metrics_enabled(&source)
        .unwrap_or_else(|reason| panic!("invalid role [dependencies].canic contract: {reason}"))
}

fn parse_role_normal_dependency_metrics_enabled(source: &str) -> Result<bool, &'static str> {
    let manifest = toml::from_str::<TomlValue>(source).map_err(|_| "Cargo.toml is invalid TOML")?;
    let dependency = manifest
        .get("dependencies")
        .and_then(|dependencies| dependencies.get("canic"))
        .and_then(TomlValue::as_table)
        .ok_or("expected an inline table for the normal `canic` dependency")?;
    let features = dependency
        .get("features")
        .and_then(TomlValue::as_array)
        .ok_or("normal `canic` dependency must declare an explicit feature array")?;

    features.iter().try_fold(false, |enabled, feature| {
        feature
            .as_str()
            .map(|feature| enabled || feature == "metrics")
            .ok_or("normal `canic` dependency features must be strings")
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
    use super::parse_role_normal_dependency_metrics_enabled;

    #[test]
    fn normal_dependency_metrics_feature_is_independent_of_the_build_dependency() {
        let source = r#"
            [dependencies]
            canic = { workspace = true, features = ["metrics", "sharding"] }

            [build-dependencies]
            canic = { workspace = true, features = [] }
        "#;

        assert_eq!(
            parse_role_normal_dependency_metrics_enabled(source),
            Ok(true)
        );
    }

    #[test]
    fn absent_normal_dependency_metrics_feature_disables_metrics() {
        let source = r#"
            [dependencies]
            canic = { workspace = true, features = ["sharding"] }

            [build-dependencies]
            canic = { workspace = true, features = ["metrics"] }
        "#;

        assert_eq!(
            parse_role_normal_dependency_metrics_enabled(source),
            Ok(false)
        );
    }

    #[test]
    fn missing_normal_dependency_feature_contract_is_rejected() {
        let source = r"
            [dependencies]
            canic = { workspace = true }

            [build-dependencies]
            canic = { workspace = true, features = [] }
        ";

        assert_eq!(
            parse_role_normal_dependency_metrics_enabled(source),
            Err("normal `canic` dependency must declare an explicit feature array")
        );
    }
}
