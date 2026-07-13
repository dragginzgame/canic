use super::super::*;
use crate::release_set::{FleetConfigError, FleetConfigOperation};
use canic_core::bootstrap::ConfigError;

#[test]
fn configured_release_roles_filters_root_and_wasm_store() {
    let config = r#"
controllers = []
app_index = []

[fleet]
name = "demo"

[roles.root]
kind = "root"
package = "root"

[roles.user_hub]
kind = "canister"
package = "user_hub"

[roles.scale_hub]
kind = "canister"
package = "scale_hub"

[app]
init_mode = "enabled"
[app.whitelist]

[subnets.prime.canisters.root]
kind = "root"

[subnets.prime.canisters.user_hub]
kind = "service"

[subnets.prime.canisters.scale_hub]
kind = "service"
"#;

    let roles = configured_release_roles_from_source(config).expect("release roles");

    assert_eq!(roles, vec!["scale_hub".to_string(), "user_hub".to_string()]);
}

#[test]
fn configured_deployable_surfaces_exclude_declared_only_roles() {
    let temp = TempWorkspace::new();
    let config_path = temp.path().join("canic.toml");
    let config = r#"
controllers = []
app_index = []

[fleet]
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

[subnets.prime.canisters.root]
kind = "root"

[subnets.prime.canisters.user_hub]
kind = "service"
"#;
    fs::write(&config_path, config).expect("write config");

    let deployable = configured_deployable_roles_from_source(config).expect("deployable roles");
    let release = configured_release_roles_from_source(config).expect("release roles");
    let install_targets =
        configured_install_targets(&config_path, "root").expect("install targets");

    assert_eq!(deployable, vec!["root".to_string(), "user_hub".to_string()]);
    assert_eq!(release, vec!["user_hub".to_string()]);
    assert_eq!(
        install_targets,
        vec!["root".to_string(), "user_hub".to_string()]
    );
    assert!(!deployable.contains(&"store".to_string()));
    assert!(!release.contains(&"store".to_string()));
    assert!(!install_targets.contains(&"store".to_string()));
}

#[test]
fn configured_deployable_roles_include_root_first() {
    let roles = configured_deployable_roles_from_source(REAL_CONFIG).expect("deployable roles");

    assert_eq!(roles.first().map(String::as_str), Some("root"));
    assert!(roles.contains(&"user_hub".to_string()));
    assert!(roles.contains(&"scale_hub".to_string()));
}

#[test]
fn configured_release_roles_rejects_multiple_root_subnets() {
    let error = configured_release_roles_from_source(MULTI_ROOT_CONFIG)
        .expect_err("multiple root roles must reject");

    assert!(matches!(
        error,
        FleetConfigError::CoreConfig {
            operation: FleetConfigOperation::Project,
            source: ConfigError::ConfigSchema(_),
        }
    ));
}

#[test]
fn configured_release_roles_rejects_missing_root() {
    let error = configured_release_roles_from_source(NO_ROOT_CONFIG)
        .expect_err("missing root role must reject");

    assert!(matches!(
        error,
        FleetConfigError::CoreConfig {
            operation: FleetConfigOperation::Project,
            source: ConfigError::ConfigSchema(_),
        }
    ));
}

#[test]
fn configured_install_targets_prefixes_root_canister() {
    let temp = TempWorkspace::new();
    let config_path = temp.path().join("canic.toml");
    fs::write(&config_path, REAL_CONFIG).expect("write config");

    let targets = configured_install_targets(&config_path, "root").expect("install targets");

    assert_eq!(
        targets,
        vec![
            "root".to_string(),
            "scale_hub".to_string(),
            "user_hub".to_string()
        ]
    );
}
