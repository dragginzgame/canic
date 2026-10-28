//! Host-selected durability for bounded, explicitly registered metric providers.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Storage selection for historical diagnostic accumulators, never business state.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MetricStorage {
    /// No durable writes; restoration starts a new observation window.
    #[default]
    Heap,
    /// Checkpoint each accepted observation through the host's shared Metrics map.
    Stable,
}

/// One explicit provider override; unknown fields are configuration errors.
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MetricProviderStorage {
    pub storage: MetricStorage,
}

/// One immutable host policy. Provider registration owns names and bounded rosters.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MetricStorageConfig {
    #[serde(default)]
    pub storage: MetricStorage,
    #[serde(default)]
    pub providers: BTreeMap<String, MetricProviderStorage>,
}
impl MetricStorageConfig {
    /// Resolve an admitted provider's explicit override or the canister default.
    #[must_use]
    pub fn storage_for(&self, provider: &str) -> MetricStorage {
        self.providers
            .get(provider)
            .map_or(self.storage, |value| value.storage)
    }
    /// Whether this policy needs a governed durable store. No store is opened here.
    #[must_use]
    pub fn needs_stable(&self) -> bool {
        self.storage == MetricStorage::Stable
            || self
                .providers
                .values()
                .any(|value| value.storage == MetricStorage::Stable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn default_and_overrides_are_independent_of_publication() {
        let empty: MetricStorageConfig = toml::from_str("").unwrap();
        assert_eq!(empty.storage_for("blob"), MetricStorage::Heap);
        assert!(!empty.needs_stable());
        let mixed: MetricStorageConfig = toml::from_str(
            "storage = 'heap'\n[providers.blob]\nstorage = 'stable'\n[providers.auth]\nstorage = 'heap'",
        ).unwrap();
        assert_eq!(mixed.storage_for("blob"), MetricStorage::Stable);
        assert_eq!(mixed.storage_for("auth"), MetricStorage::Heap);
        assert_eq!(mixed.storage_for("performance"), MetricStorage::Heap);
        assert!(mixed.needs_stable());
    }
    #[test]
    fn malformed_choices_and_override_fields_refuse() {
        for value in [
            "storage='disk'",
            "[providers.blob]\nstorage='stable'\nreset=true",
            "[providers.blob]\nstable=true",
        ] {
            assert!(toml::from_str::<MetricStorageConfig>(value).is_err());
        }
    }
}

#[cfg(any(not(target_arch = "wasm32"), test))]
impl super::Validate for MetricStorageConfig {
    fn validate(&self) -> Result<(), super::ConfigSchemaError> {
        if self.providers.len() > 16 {
            return Err(super::ConfigSchemaError::ValidationError(
                "too many metric provider overrides".into(),
            ));
        }
        for name in self.providers.keys() {
            ic_metrics_stable::MetricKey::new(name, "configuration").map_err(|_| {
                super::ConfigSchemaError::ValidationError("invalid metric provider name".into())
            })?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod role_tests {
    use super::*;
    use crate::config::schema::{MetricsCanisterConfig, MetricsProfile};
    #[test]
    fn existing_role_metrics_table_keeps_profile_and_scopes_its_policy() {
        let config: MetricsCanisterConfig =
            toml::from_str("profile='full'\nstorage='heap'\n[providers.blob]\nstorage='stable'")
                .unwrap();
        assert_eq!(config.profile, Some(MetricsProfile::Full));
        assert_eq!(
            config.storage_policy().storage_for("blob"),
            MetricStorage::Stable
        );
        assert_eq!(
            MetricsCanisterConfig::default()
                .storage_policy()
                .storage_for("blob"),
            MetricStorage::Heap
        );
    }

    #[test]
    fn compiled_storage_policy_changes_authority_and_only_allocates_for_its_role() {
        use crate::config::{Config, RoleRuntimeAuthority};
        use canic_contracts::ids::CanisterRole;
        let source = "[app]\nname='metrics'\n[roles.root]\nkind='root'\n[roles.worker]\nkind='canister'\npackage='worker'\n[component_specs.worker]\ncomponent_role='worker'\nmaximum_instances=1\n[component_groups.workers.components.worker]\ncomponent_spec='worker'\n[component_group_deployments.workers]\ncomponent_group='workers'\ninitial_placements=1\nmaximum_placements=1\nplacement.maximum_per_root=1\nplacement.minimum_distinct_roots=1\n";
        let heap = Config::parse_toml(source).unwrap();
        let stable = Config::parse_toml(&format!(
            "{source}\n[component_specs.worker.metrics.providers.blob]\nstorage='stable'\n"
        ))
        .unwrap();
        let worker = CanisterRole::from("worker");
        let heap = RoleRuntimeAuthority::compile(&heap, &worker).unwrap();
        let selected = RoleRuntimeAuthority::compile(&stable, &worker).unwrap();
        let root = RoleRuntimeAuthority::compile(&stable, &CanisterRole::ROOT).unwrap();
        let requests = crate::ops::runtime::metric_providers::host::role_memory_requests;
        assert!(requests(&heap).unwrap().is_empty());
        assert!(requests(&root).unwrap().is_empty());
        assert_eq!(requests(&selected).unwrap().len(), 1);
        assert_ne!(heap.configuration_digest, selected.configuration_digest);
        assert_eq!(
            selected.canisters[0].config.metrics.storage_for("blob"),
            MetricStorage::Stable
        );
    }
}
