use super::*;

#[test]
fn configured_fleet_name_reads_required_config_identity() {
    let name = configured_fleet_name_from_source(REAL_CONFIG).expect("fleet name");

    assert_eq!(name, "demo");
}

#[test]
fn configured_fleet_name_rejects_missing_config_identity() {
    configured_fleet_name_from_source(
        r#"
controllers = []
app_index = []

[app]
init_mode = "enabled"
[app.whitelist]

[subnets.prime.canisters.root]
kind = "root"
"#,
    )
    .expect_err("missing fleet name must reject");
}

#[test]
fn configured_controllers_reads_top_level_authority() {
    let controllers = configured_controllers_from_source(
        r#"
controllers = [
  "zbf4m-zw3nk-6owqc-qmluz-xhwxt-2pkky-xhjy2-kqxor-qzxsn-6d2bz-nae",
  "aaaaa-aa",
  "aaaaa-aa",
]
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
"#,
    )
    .expect("configured controllers");

    assert_eq!(
        controllers,
        vec![
            "aaaaa-aa".to_string(),
            "zbf4m-zw3nk-6owqc-qmluz-xhwxt-2pkky-xhjy2-kqxor-qzxsn-6d2bz-nae".to_string(),
        ]
    );
}
