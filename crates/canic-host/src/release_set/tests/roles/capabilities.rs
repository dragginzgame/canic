use super::super::*;
use std::collections::BTreeSet;

#[test]
fn configured_role_capabilities_lists_enabled_role_features() {
    use canic_core::role_contract::RoleCapabilityKey;

    let capabilities = BTreeSet::from([
        RoleCapabilityKey::DelegatedTokenIssuer,
        RoleCapabilityKey::Directory,
        RoleCapabilityKey::Root,
        RoleCapabilityKey::RootControlPlane,
        RoleCapabilityKey::Scaling,
        RoleCapabilityKey::Sharding,
    ]);

    assert_eq!(
        crate::release_set::config::project_role_capabilities(&capabilities),
        vec![
            "auth".to_string(),
            "directory".to_string(),
            "scaling".to_string(),
            "sharding".to_string(),
        ]
    );
}

#[test]
fn configured_role_capabilities_resolves_exact_role_package_contracts() {
    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let config = workspace.join("canisters/audit/root_probe/canic.toml");

    let capabilities = crate::release_set::FleetConfigSnapshot::load(&config)
        .expect("load config")
        .role_capabilities()
        .expect("resolved capabilities");
    assert!(capabilities.is_empty());
}

#[test]
fn configured_role_metrics_profiles_lists_resolved_profiles() {
    let config = r#"
controllers = []
app_index = []

[fleet]
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

[subnets.prime.canisters.root]
kind = "root"

[subnets.prime.canisters.user_hub]
kind = "service"

[subnets.prime.canisters.user_hub.sharding.pools.user_shards]
canister_role = "user_shard"

[subnets.prime.canisters.user_shard]
kind = "shard"

[subnets.prime.canisters.scale_replica]
kind = "replica"

[subnets.prime.canisters.scale_replica.metrics]
profile = "full"
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
controllers = []
app_index = []

[fleet]
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

[app]
init_mode = "enabled"
[app.whitelist]

[subnets.prime.canisters.root]
kind = "root"

[subnets.prime.canisters.scale_hub]
kind = "service"
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
