use super::super::*;

#[test]
fn configured_role_auto_create_lists_component_roles() {
    let config = r#"
[app]
name = "demo"
init_mode = "enabled"


[roles.root]
kind = "root"

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

[component_specs.user_hub]
component_role = "user_hub"
maximum_instances = 1
"#;
    let auto_create = configured_role_auto_create_from_config(&parsed_config(config));

    assert!(auto_create.contains("app"));
    assert!(auto_create.contains("user_hub"));
    assert!(!auto_create.contains("root"));
}
