use super::super::*;

#[test]
fn configured_pool_expectations_lists_component_pools() {
    let config = r#"
controllers = []
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



[component_specs.user_hub]
component_role = "user_hub"
maximum_instances = 1

[component_specs.user_hub.sharding.pools.user_shards]
canister_role = "user_shard"
policy.capacity = 100
policy.max_shards = 4

[component_specs.user_hub.binding.pools.projects]
canister_role = "project_instance"
key_name = "project_id"

[component_specs.user_hub.children.user_shard]
kind = "shard"
maximum_instances = 4096

[component_specs.user_hub.children.project_instance]
kind = "instance"
maximum_instances = 4096

[component_specs.scale_hub]
component_role = "scale_hub"
maximum_instances = 1

[component_specs.scale_hub.scaling.pools.scales]
canister_role = "scale_replica"

[component_specs.scale_hub.children.scale_replica]
kind = "replica"
maximum_instances = 4096
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

[component_specs.app]
component_role = "app"
maximum_instances = 1
initial_cycles = "7T"

[component_specs.user_hub]
component_role = "user_hub"
maximum_instances = 1
"#;

    let cycles = configured_local_root_create_cycles_from_config(&parsed_config(config));

    assert_eq!(cycles, Some(117_000_000_000_000));
}

#[test]
fn configured_role_auto_create_lists_component_roles() {
    let config = r#"
controllers = []
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



[component_specs.app]
component_role = "app"
maximum_instances = 1

[component_specs.user_hub]
component_role = "user_hub"
maximum_instances = 1
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



[component_specs.app]
component_role = "app"
maximum_instances = 1

[component_specs.user_hub]
component_role = "user_hub"
maximum_instances = 1

[component_specs.user_hub.sharding.pools.user_shards]
canister_role = "user_shard"
policy.capacity = 100
policy.initial_shards = 1
policy.max_shards = 4

[component_specs.user_hub.children.user_shard]
kind = "shard"
maximum_instances = 4096

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
