use super::super::*;

#[test]
fn configured_role_kinds_lists_configured_roles() {
    let kinds = configured_role_kinds_from_config(&parsed_config(REAL_CONFIG));

    assert_eq!(kinds.get("root").map(String::as_str), Some("root"));
    assert_eq!(kinds.get("user_hub").map(String::as_str), Some("service"));
    assert_eq!(kinds.get("scale_hub").map(String::as_str), Some("service"));
}

#[test]
fn configured_role_lifecycle_lists_declared_and_attached_roles() {
    let config = r#"
controllers = []
[services.fleet]
roles = []

[app]
name = "demo"

[roles.root]
kind = "root"
package = "canisters/root"

[roles.user_hub]
kind = "canister"
package = "canisters/user_hub"

[roles.user_shard]
kind = "canister"
package = "canisters/user_shard"

[roles.store]
kind = "canister"
package = "canisters/store"

[subnets.default.canisters.root]
kind = "root"

[subnets.default.canisters.user_hub]
kind = "service"

[subnets.default.canisters.user_hub.sharding.pools.users]
canister_role = "user_shard"

[subnets.default.canisters.user_shard]
kind = "shard"
"#;
    let lifecycle = configured_role_lifecycle_from_config(&parsed_config(config));

    let root = lifecycle
        .iter()
        .find(|role| role.role == "root")
        .expect("root lifecycle row");
    assert_eq!(root.display, "demo.root");
    assert_eq!(root.state, "attached");
    assert_eq!(root.topology.as_deref(), Some("default/root"));

    let shard = lifecycle
        .iter()
        .find(|role| role.role == "user_shard")
        .expect("shard lifecycle row");
    assert_eq!(shard.state, "attached");
    assert_eq!(
        shard.topology.as_deref(),
        Some("default/user_hub/sharding/users,default/user_shard")
    );

    let store = lifecycle
        .iter()
        .find(|role| role.role == "store")
        .expect("store lifecycle row");
    assert_eq!(store.package, "canisters/store");
    assert_eq!(store.state, "declared");
    assert_eq!(store.topology, None);
}

#[test]
fn configured_role_details_lists_verbose_config_features() {
    let config = r#"
controllers = []
[services.fleet]
roles = ["user_hub", "scale_hub"]

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
[app.whitelist]

[subnets.default.canisters.root]
kind = "root"

[subnets.default.canisters.user_hub]
kind = "service"
topup.threshold = "10T"
topup.amount = "4T"

[subnets.default.canisters.user_hub.sharding.pools.user_shards]
canister_role = "user_shard"
policy.capacity = 100
policy.max_shards = 4

[subnets.default.canisters.user_shard]
kind = "shard"

[subnets.default.canisters.user_shard.auth]
delegated_token_issuer = true
role_attestation_cache = true

[subnets.default.canisters.scale_hub]
kind = "service"

[subnets.default.canisters.scale_hub.scaling.pools.scales]
canister_role = "scale_replica"
policy.initial_workers = 2
policy.min_workers = 2

[subnets.default.canisters.scale_replica]
kind = "replica"

[subnets.default.canisters.scale_replica.metrics]
profile = "full"
"#;
    let details = configured_role_details_from_config(&parsed_config(config));

    assert!(
        details
            .get("user_hub")
            .is_some_and(|details| details.contains(&"services.fleet".to_string()))
    );
    assert!(details.get("user_hub").is_some_and(|details| {
        details
            .iter()
            .any(|detail| detail == "sharding user_shards->user_shard cap=100 initial=1 max=4")
    }));
    assert!(
        details
            .get("user_shard")
            .is_some_and(|details| details.contains(&"auth delegated-token-issuer".to_string()))
    );
    assert!(details.get("scale_hub").is_some_and(|details| {
        details.contains(&"scaling scales->scale_replica initial=2 min=2 max=32".to_string())
    }));
    assert!(details.get("user_hub").is_some_and(|details| {
        details.contains(
            &"metrics profile=hub tiers=core,placement,runtime,security (inferred)".to_string(),
        )
    }));
    assert!(details.get("scale_replica").is_some_and(|details| {
        details.contains(
            &"metrics profile=full tiers=core,placement,platform,runtime,security,storage (configured)"
                .to_string()
        )
    }));
}
