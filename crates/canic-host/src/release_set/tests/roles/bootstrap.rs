use super::super::*;

#[test]
fn configured_pool_expectations_lists_root_subnet_pools() {
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

[subnets.prime.canisters.user_hub]
kind = "service"

[subnets.prime.canisters.user_hub.sharding.pools.user_shards]
canister_role = "user_shard"
policy.capacity = 100
policy.max_shards = 4

[subnets.prime.canisters.user_hub.directory.pools.projects]
canister_role = "project_instance"
key_name = "project_id"

[subnets.prime.canisters.user_shard]
kind = "shard"

[subnets.prime.canisters.project_instance]
kind = "instance"

[subnets.prime.canisters.scale_hub]
kind = "service"

[subnets.prime.canisters.scale_hub.scaling.pools.scales]
canister_role = "scale_replica"

[subnets.prime.canisters.scale_replica]
kind = "replica"
"#;
    let pools = configured_pool_expectations_from_config(&parsed_config(config));

    assert_eq!(pools.len(), 3);
    assert!(
        pools
            .iter()
            .any(|pool| { pool.pool == "user_shards" && pool.canister_role == "user_shard" })
    );
    assert!(
        pools
            .iter()
            .any(|pool| { pool.pool == "projects" && pool.canister_role == "project_instance" })
    );
    assert!(
        pools
            .iter()
            .any(|pool| { pool.pool == "scales" && pool.canister_role == "scale_replica" })
    );
}

#[test]
fn configured_local_root_create_cycles_estimates_bootstrap_funding() {
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

[subnets.prime]
pool.minimum_size = 2

[subnets.prime.canisters.root]
kind = "root"

[subnets.prime.canisters.app]
kind = "service"
initial_cycles = "7T"

[subnets.prime.canisters.user_hub]
kind = "service"
"#;

    let cycles = configured_local_root_create_cycles_from_config(&parsed_config(config));

    assert_eq!(cycles, 127_000_000_000_000);
}

#[test]
fn configured_role_auto_create_lists_derived_service_roles() {
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

[subnets.prime.canisters.app]
kind = "service"

[subnets.prime.canisters.user_hub]
kind = "service"
"#;
    let auto_create = configured_role_auto_create_from_config(&parsed_config(config));

    assert!(auto_create.contains("app"));
    assert!(auto_create.contains("user_hub"));
    assert!(!auto_create.contains("root"));
}

#[test]
fn configured_bootstrap_roles_include_only_bootstrap_obligations() {
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

[subnets.prime.canisters.app]
kind = "service"

[subnets.prime.canisters.user_hub]
kind = "service"

[subnets.prime.canisters.user_hub.sharding.pools.user_shards]
canister_role = "user_shard"
policy.capacity = 100
policy.initial_shards = 1
policy.max_shards = 4

[subnets.prime.canisters.user_shard]
kind = "shard"

[subnets.prime.canisters.role_baseline]
kind = "replica"
"#;
    let roles = configured_bootstrap_roles_from_config(&parsed_config(config));

    assert_eq!(
        roles,
        vec![
            "root".to_string(),
            "app".to_string(),
            "user_hub".to_string(),
            "user_shard".to_string()
        ]
    );
}
