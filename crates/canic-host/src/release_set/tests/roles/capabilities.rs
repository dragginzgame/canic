use super::super::*;
use std::collections::BTreeSet;

#[test]
fn configured_role_capabilities_lists_enabled_role_features() {
    use canic_core::role_contract::RoleCapabilityKey;

    let capabilities = BTreeSet::from([
        RoleCapabilityKey::AutomaticTopup,
        RoleCapabilityKey::DelegatedTokenIssuer,
        RoleCapabilityKey::Index,
        RoleCapabilityKey::Root,
        RoleCapabilityKey::RootControlPlane,
        RoleCapabilityKey::Scaling,
        RoleCapabilityKey::Sharding,
    ]);

    assert_eq!(
        crate::release_set::config::project_role_capabilities(&capabilities),
        vec![
            "auth".to_string(),
            "automatic_topup".to_string(),
            "index".to_string(),
            "scaling".to_string(),
            "sharding".to_string(),
        ]
    );
}

#[test]
fn configured_role_capabilities_resolves_exact_role_package_contracts() {
    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let config = workspace.join("canisters/audit/root_probe/canic.toml");

    let capabilities = crate::release_set::AppConfigSnapshot::load(&config)
        .expect("load config")
        .role_capabilities()
        .expect("resolved capabilities");
    assert!(capabilities.is_empty());
}

#[test]
fn configured_role_metrics_profiles_lists_resolved_profiles() {
    let config = r#"
[app]
name = "demo"

[roles.root]
kind = "root"
package = "root"

[roles.app]
kind = "canister"
package = "app"

[roles.user_hub]
kind = "canister"
package = "user_hub"

[roles.user_shard]
kind = "canister"
package = "user_shard"

[roles.project_instance]
kind = "canister"
package = "project_instance"

[roles.scale_hub]
kind = "canister"
package = "scale_hub"

[roles.scale_replica]
kind = "canister"
package = "scale"

[roles.role_baseline]
kind = "canister"
package = "role_baseline"



[component_specs.user_hub]
component_role = "user_hub"
maximum_instances = 1

[component_specs.user_hub.sharding.pools.user_shards]
canister_role = "user_shard"

[component_specs.user_hub.children.user_shard]
kind = "shard"

[component_specs.user_hub.spawn_grants.user_hub.user_shard]
maximum_instances_per_parent = 20_000

[component_specs.scale_hub]
component_role = "scale_hub"
maximum_instances = 1

[component_specs.scale_hub.children.scale_replica]
kind = "replica"

[component_specs.scale_hub.children.scale_replica.metrics]
profile = "full"

[component_specs.scale_hub.spawn_grants.scale_hub.scale_replica]
maximum_instances_per_parent = 20_000
"#;
    let profiles = configured_role_metrics_profiles_from_config(&parsed_config(config));

    assert_eq!(profiles.get("root").map(String::as_str), Some("root"));
    assert_eq!(profiles.get("user_hub").map(String::as_str), Some("hub"));
    assert_eq!(profiles.get("user_shard").map(String::as_str), Some("leaf"));
    assert_eq!(
        profiles.get("scale_replica").map(String::as_str),
        Some("full")
    );
}

#[test]
fn configured_role_topups_lists_configured_policy_summaries() {
    let config = r#"
[app]
name = "demo"
init_mode = "enabled"


[roles.root]
kind = "root"
package = "root"

[roles.app]
kind = "canister"
package = "app"

[roles.user_hub]
kind = "canister"
package = "user_hub"

[roles.user_shard]
kind = "canister"
package = "user_shard"

[roles.project_instance]
kind = "canister"
package = "project_instance"

[roles.scale_hub]
kind = "canister"
package = "scale_hub"

[roles.scale_replica]
kind = "canister"
package = "scale"

[roles.role_baseline]
kind = "canister"
package = "role_baseline"
[component_specs.scale_hub]
component_role = "scale_hub"
maximum_instances = 1
topup.threshold = "10T"
topup.amount = "4T"
"#;
    let topups = configured_role_topups_from_config(&parsed_config(config));

    assert_eq!(
        topups.get("scale_hub").map(String::as_str),
        Some("4.00 TC @ 10.00 TC")
    );
    assert!(!topups.contains_key("root"));
}
