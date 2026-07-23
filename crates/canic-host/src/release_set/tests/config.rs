use super::*;
use canic_core::bootstrap::ConfigError;

#[test]
fn configured_app_name_reads_required_config_identity() {
    let config = parsed_config(REAL_CONFIG);
    let name = config.app_id();

    assert_eq!(name.as_str(), "demo");
}

#[test]
fn configured_app_name_rejects_missing_config_identity() {
    let error = canic_core::bootstrap::parse_config_model(
        r#"
controllers = []
[services.fleet]
roles = []

[app]
init_mode = "enabled"
[app.whitelist]

[subnets.default.canisters.root]
kind = "root"
"#,
    )
    .expect_err("missing App name must reject");

    assert!(matches!(
        error,
        ConfigError::CannotParseToml {
            issue: canic_core::bootstrap::ConfigTomlIssue::InvalidValue { logical_path },
            ..
        } if logical_path == "app"
    ));
}

#[test]
fn configured_controllers_reads_top_level_authority() {
    let config = parsed_config(
        r#"
controllers = [
  "zbf4m-zw3nk-6owqc-qmluz-xhwxt-2pkky-xhjy2-kqxor-qzxsn-6d2bz-nae",
  "aaaaa-aa",
  "aaaaa-aa",
]
[services.fleet]
roles = []

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
"#,
    );
    let controllers = configured_controllers_from_config(&config);

    assert_eq!(
        controllers,
        vec![
            "aaaaa-aa".to_string(),
            "zbf4m-zw3nk-6owqc-qmluz-xhwxt-2pkky-xhjy2-kqxor-qzxsn-6d2bz-nae".to_string(),
        ]
    );
}
