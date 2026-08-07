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
[app]
init_mode = "enabled"
[app.whitelist]


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
