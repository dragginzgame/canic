use super::super::*;
use canic_core::bootstrap::{ConfigError, parse_config_model};

#[test]
fn configured_release_roles_filters_root_and_wasm_store() {
    let config = r#"
controllers = []
[app]
name = "demo"
init_mode = "enabled"


[roles.root]
kind = "root"
package = "root"

[roles.user_hub]
kind = "canister"
package = "user_hub"

[roles.scale_hub]
kind = "canister"
package = "scale_hub"
[app.whitelist]

[tree_groups.default]
tree_spec = "default"
initial_trees = 1
maximum_trees = 1

[tree_specs.default.canisters.root]
kind = "root"

[tree_specs.default.canisters.user_hub]
kind = "service"

[tree_specs.default.canisters.scale_hub]
kind = "service"
"#;

    let config = parse_config_model(config).expect("valid config");
    let roles = configured_release_roles_from_config(&config);

    assert_eq!(roles, vec!["scale_hub".to_string(), "user_hub".to_string()]);
}

#[test]
fn configured_deployable_surfaces_exclude_declared_only_roles() {
    let config = r#"
controllers = []
[app]
name = "demo"

[roles.root]
kind = "root"
package = "root"

[roles.user_hub]
kind = "canister"
package = "user_hub"

[roles.store]
kind = "canister"
package = "store"

[tree_groups.default]
tree_spec = "default"
initial_trees = 1
maximum_trees = 1

[tree_specs.default.canisters.root]
kind = "root"

[tree_specs.default.canisters.user_hub]
kind = "service"
"#;

    let deployable = configured_deployable_roles_from_config(&parsed_config(config));
    let config = parse_config_model(config).expect("valid config");
    let release = configured_release_roles_from_config(&config);

    assert_eq!(deployable, vec!["root".to_string(), "user_hub".to_string()]);
    assert_eq!(release, vec!["user_hub".to_string()]);
    assert!(!deployable.contains(&"store".to_string()));
    assert!(!release.contains(&"store".to_string()));
}

#[test]
fn configured_deployable_roles_include_root_first() {
    let roles = configured_deployable_roles_from_config(&parsed_config(REAL_CONFIG));

    assert_eq!(roles.first().map(String::as_str), Some("root"));
    assert!(roles.contains(&"user_hub".to_string()));
    assert!(roles.contains(&"scale_hub".to_string()));
}

#[test]
fn configured_release_roles_supports_multiple_tree_specs() {
    let config = parse_config_model(MULTI_ROOT_CONFIG).expect("multiple Tree Specs");
    let roles = configured_release_roles_from_config(&config);

    assert!(roles.is_empty());
}

#[test]
fn configured_release_roles_rejects_missing_root() {
    let error = parse_config_model(NO_ROOT_CONFIG).expect_err("missing root role must reject");

    assert!(matches!(error, ConfigError::ConfigSchema(_)));
}
