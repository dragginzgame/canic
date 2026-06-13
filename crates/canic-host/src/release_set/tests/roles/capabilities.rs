use super::super::*;

#[test]
fn configured_role_capabilities_lists_enabled_role_features() {
    let config = r#"
controllers = []
app_index = ["user_hub", "scale_hub"]

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

[subnets.prime.canisters.user_hub]
kind = "service"

[subnets.prime.canisters.user_hub.sharding.pools.user_shards]
canister_role = "user_shard"
policy.capacity = 100
policy.max_shards = 4

[subnets.prime.canisters.user_shard]
kind = "shard"

[subnets.prime.canisters.user_shard.auth]
delegated_token_issuer = true

[subnets.prime.canisters.scale_hub]
kind = "service"

[subnets.prime.canisters.scale_hub.scaling.pools.scales]
canister_role = "scale_replica"

[subnets.prime.canisters.scale_replica]
kind = "replica"
"#;
    let capabilities = configured_role_capabilities_from_source(config).expect("role capabilities");

    assert_eq!(
        capabilities.get("user_hub"),
        Some(&vec!["sharding".to_string()])
    );
    assert_eq!(
        capabilities.get("user_shard"),
        Some(&vec!["auth".to_string()])
    );
    assert_eq!(
        capabilities.get("scale_hub"),
        Some(&vec!["scaling".to_string()])
    );
    assert!(!capabilities.contains_key("root"));
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
    let profiles = configured_role_metrics_profiles_from_source(config).expect("metrics profiles");

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
    let topups = configured_role_topups_from_source(config).expect("role topups");

    assert_eq!(
        topups.get("scale_hub").map(String::as_str),
        Some("4.00 TC @ 10.00 TC")
    );
    assert!(!topups.contains_key("root"));
}
