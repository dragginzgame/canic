use super::*;
use crate::install_root::ConfigDiscoveryError;

#[test]
fn install_config_defaults_to_project_config_when_present() {
    let root = temp_dir("canic-install-config-default");
    let config = root.join("apps/canic.toml");
    fs::create_dir_all(config.parent().expect("config parent")).expect("create parent");
    fs::write(&config, "").expect("write config");

    let resolved = resolve_install_config_path(&root, None, false).expect("resolve config");

    assert_eq!(resolved, config);
    fs::remove_dir_all(root).expect("clean temp dir");
}

#[test]
fn install_config_accepts_explicit_path() {
    let root = temp_dir("canic-install-config-explicit");
    let resolved = resolve_install_config_path(&root, Some("apps/demo/canic.toml"), false)
        .expect("resolve config");

    assert_eq!(resolved, root.join("apps/demo/canic.toml"));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn install_config_error_lists_choices_when_project_default_missing() {
    let root = temp_dir("canic-install-config-choices");
    let demo = root.join("apps/demo/canic.toml");
    let test = root.join("canisters/test/runtime_probe/canic.toml");
    fs::create_dir_all(demo.parent().expect("demo parent")).expect("create demo parent");
    fs::create_dir_all(test.parent().expect("test parent")).expect("create test parent");
    fs::create_dir_all(root.join("apps/demo/root")).expect("create demo root");
    fs::write(
        &demo,
        r#"
[app]
name = "demo"

[roles.root]
kind = "root"
package = "root"

[roles.app]
kind = "canister"
package = "app"

[roles.project_registry]
kind = "canister"
package = "project_registry"

[roles.oracle_pokemon]
kind = "canister"
package = "oracle_pokemon"

[roles.user_hub]
kind = "canister"
package = "user_hub"

[roles.user_shard]
kind = "canister"
package = "user_shard"

[roles.scale_hub]
kind = "canister"
package = "scale_hub"

[roles.scale_replica]
kind = "canister"
package = "scale"

[roles.role_baseline]
kind = "canister"
package = "role_baseline"

[roles.worker]
kind = "canister"
package = "worker"

[subnets.default.canisters.root]
kind = "root"

[subnets.default.canisters.app]
kind = "service"

[subnets.default.canisters.user_hub]
kind = "service"
"#,
    )
    .expect("write demo config");
    fs::write(&test, "").expect("write test config");
    fs::write(root.join("apps/demo/root/Cargo.toml"), "").expect("write demo root manifest");
    let err = resolve_install_config_path(&root, None, false).expect_err("selection must fail");
    let message = err.to_string();

    assert!(message.contains("missing default Canic config at apps/canic.toml"));
    assert!(!message.contains("found one install config:"));
    assert!(message.contains("apps/demo/canic.toml"));
    assert!(message.contains("3 (root, app, user_hub)"));
    assert!(message.contains("apps/canic.toml\n\n#"));
    assert!(message.contains("3 (root, app, user_hub)\n\nrun:"));
    assert!(!message.contains("canisters/test/runtime_probe/canic.toml"));
    assert!(message.contains("run: canic install demo"));

    fs::remove_dir_all(root).expect("clean temp dir");
}

#[test]
fn config_selection_error_is_whitespace_table() {
    let root = temp_dir("canic-install-config-single-table");
    let config = root.join("apps/demo/canic.toml");
    fs::create_dir_all(config.parent().expect("config parent")).expect("create config parent");
    fs::write(
        &config,
        r#"
[app]
name = "demo"

[roles.root]
kind = "root"
package = "root"

[roles.app]
kind = "canister"
package = "app"

[roles.project_registry]
kind = "canister"
package = "project_registry"

[roles.oracle_pokemon]
kind = "canister"
package = "oracle_pokemon"

[roles.user_hub]
kind = "canister"
package = "user_hub"

[roles.user_shard]
kind = "canister"
package = "user_shard"

[roles.scale_hub]
kind = "canister"
package = "scale_hub"

[roles.scale_replica]
kind = "canister"
package = "scale"

[roles.role_baseline]
kind = "canister"
package = "role_baseline"

[roles.worker]
kind = "canister"
package = "worker"

[subnets.default.canisters.root]
kind = "root"

[subnets.default.canisters.app]
kind = "service"
"#,
    )
    .expect("write config");
    let message = config_selection_error(
        &root,
        &root.join("apps/canic.toml"),
        std::slice::from_ref(&config),
    );

    assert!(message.contains('#'));
    assert!(message.contains("CONFIG"));
    assert!(message.contains("CANISTERS"));
    assert!(message.contains("apps/demo/canic.toml"));
    assert!(message.contains("2 (root, app)"));
    assert!(message.contains("apps/canic.toml\n\n#"));
    assert!(message.contains("2 (root, app)\n\nrun:"));
    assert!(message.contains("run: canic install demo"));
    fs::remove_dir_all(root).expect("clean temp dir");
}

#[test]
fn config_selection_error_lists_multiple_paths_with_numbered_options() {
    let root = temp_dir("canic-install-config-multiple-table");
    let demo = root.join("apps/demo/canic.toml");
    let example = root.join("apps/example/canic.toml");
    fs::create_dir_all(demo.parent().expect("demo parent")).expect("create demo parent");
    fs::create_dir_all(example.parent().expect("example parent")).expect("create example parent");
    fs::write(
        &demo,
        demo_config_source(
            r#"
[subnets.default.canisters.root]
kind = "root"

[subnets.default.canisters.app]
kind = "service"
"#,
        ),
    )
    .expect("write demo config");
    fs::write(
        &example,
        demo_config_source(
            r#"
[subnets.default.canisters.root]
kind = "root"

[subnets.default.canisters.user_hub]
kind = "service"

[subnets.default.canisters.user_shard]
kind = "service"

[subnets.default.canisters.scale_replica]
kind = "service"

[subnets.default.canisters.scale_hub]
kind = "service"
"#,
        ),
    )
    .expect("write example config");
    let message = config_selection_error(&root, &root.join("apps/canic.toml"), &[demo, example]);

    assert!(message.contains("choose an App explicitly:"));
    assert!(message.contains("choose an App explicitly:\n\n#"));
    assert!(message.contains('#'));
    assert!(message.contains("CONFIG"));
    assert!(message.contains("CANISTERS"));
    assert!(message.contains("1   apps/demo/canic.toml"));
    assert!(message.contains("2   apps/example/canic.toml"));
    assert!(message.contains("apps/demo/canic.toml"));
    assert!(message.contains("2 (root, app)"));
    assert!(message.contains("apps/example/canic.toml"));
    assert!(message.contains("5 (root, scale_hub, scale_replica, user_hub, user_shard)"));
    assert!(message.contains("5 (root, scale_hub, scale_replica, user_hub, user_shard)\n\nrun:"));
    assert!(message.contains("run: canic install <app>"));
    fs::remove_dir_all(root).expect("clean temp dir");
}

#[test]
fn config_selection_preview_lists_six_canisters_before_ellipsis() {
    let root = temp_dir("canic-install-config-preview-limit");
    let config = root.join("apps/demo/canic.toml");
    fs::create_dir_all(config.parent().expect("config parent")).expect("create config parent");
    fs::write(
        &config,
        r#"
[app]
name = "demo"

[roles.root]
kind = "root"
package = "root"

[roles.app]
kind = "canister"
package = "app"

[roles.project_registry]
kind = "canister"
package = "project_registry"

[roles.oracle_pokemon]
kind = "canister"
package = "oracle_pokemon"

[roles.user_hub]
kind = "canister"
package = "user_hub"

[roles.user_shard]
kind = "canister"
package = "user_shard"

[roles.scale_hub]
kind = "canister"
package = "scale_hub"

[roles.scale_replica]
kind = "canister"
package = "scale"

[roles.role_baseline]
kind = "canister"
package = "role_baseline"

[roles.worker]
kind = "canister"
package = "worker"

[subnets.default.canisters.root]
kind = "root"

[subnets.default.canisters.app]
kind = "service"

[subnets.default.canisters.role_baseline]
kind = "service"

[subnets.default.canisters.scale_replica]
kind = "service"

[subnets.default.canisters.scale_hub]
kind = "service"

[subnets.default.canisters.user_hub]
kind = "service"

[subnets.default.canisters.user_shard]
kind = "service"

[subnets.default.canisters.worker]
kind = "service"
"#,
    )
    .expect("write config");

    let message = config_selection_error(
        &root,
        &root.join("apps/canic.toml"),
        std::slice::from_ref(&config),
    );

    assert!(
        message.contains("8 (root, app, role_baseline, scale_hub, scale_replica, user_hub, ...)")
    );
    fs::remove_dir_all(root).expect("clean temp dir");
}

#[test]
fn discovered_install_config_choices_are_path_sorted() {
    let root = temp_dir("canic-install-config-sorted");
    let alpha = root.join("alpha/canic.toml");
    let zeta = root.join("zeta/canic.toml");
    fs::create_dir_all(alpha.parent().expect("alpha parent").join("root"))
        .expect("create alpha root");
    fs::create_dir_all(zeta.parent().expect("zeta parent").join("root")).expect("create zeta root");
    fs::write(&zeta, "").expect("write zeta config");
    fs::write(&alpha, "").expect("write alpha config");
    fs::write(
        alpha
            .parent()
            .expect("alpha parent")
            .join("root/Cargo.toml"),
        "",
    )
    .expect("write alpha root manifest");
    fs::write(
        zeta.parent().expect("zeta parent").join("root/Cargo.toml"),
        "",
    )
    .expect("write zeta root manifest");

    let choices = discover_canic_config_choices(&root).expect("discover choices");

    assert_eq!(choices, vec![alpha, zeta]);
    fs::remove_dir_all(root).expect("clean temp dir");
}

#[test]
fn discovered_install_config_choices_accept_split_source_app_configs() {
    let root = temp_dir("canic-install-config-split-source");
    let config = root.join("toko/canic.toml");
    fs::create_dir_all(config.parent().expect("config parent")).expect("create config parent");
    fs::write(&config, "[app]\nname = \"toko\"\n").expect("write config");

    let choices = discover_canic_config_choices(&root).expect("discover choices");

    assert_eq!(choices, vec![config]);
    fs::remove_dir_all(root).expect("clean temp dir");
}

#[test]
fn discovered_workspace_config_choices_accept_root_apps() {
    let root = temp_dir("canic-install-config-root-apps");
    let config = root.join("apps/toko/canic.toml");
    fs::create_dir_all(config.parent().expect("config parent")).expect("create config parent");
    fs::write(&config, "[app]\nname = \"toko\"\n").expect("write config");

    let choices = discover_project_canic_config_choices(&root).expect("discover choices");

    assert_eq!(choices, vec![config.clone()]);
    assert_eq!(
        select_discovered_app_config_path(&choices, "toko").expect("select app"),
        Some(config)
    );
    assert_eq!(
        select_discovered_app_config_path(&choices, "missing").expect("select missing app"),
        None
    );
    fs::remove_dir_all(root).expect("clean temp dir");
}

#[test]
fn discovered_install_config_choices_reject_duplicate_app_names() {
    let root = temp_dir("canic-install-config-duplicate-app");
    let demo = root.join("demo/canic.toml");
    let copy = root.join("copy/canic.toml");
    fs::create_dir_all(demo.parent().expect("demo parent").join("root")).expect("create demo root");
    fs::create_dir_all(copy.parent().expect("copy parent").join("root")).expect("create copy root");
    fs::write(
        demo.parent().expect("demo parent").join("root/Cargo.toml"),
        "",
    )
    .expect("write demo root manifest");
    fs::write(
        copy.parent().expect("copy parent").join("root/Cargo.toml"),
        "",
    )
    .expect("write copy root manifest");
    let config = r#"
controllers = []
[services.fleet]
roles = []

[app]
name = "demo"

[roles.root]
kind = "root"
package = "root"

[roles.app]
kind = "canister"
package = "app"

[roles.project_registry]
kind = "canister"
package = "project_registry"

[roles.oracle_pokemon]
kind = "canister"
package = "oracle_pokemon"

[roles.user_hub]
kind = "canister"
package = "user_hub"

[roles.user_shard]
kind = "canister"
package = "user_shard"

[roles.scale_hub]
kind = "canister"
package = "scale_hub"

[roles.scale_replica]
kind = "canister"
package = "scale"

[roles.role_baseline]
kind = "canister"
package = "role_baseline"

[roles.worker]
kind = "canister"
package = "worker"

[subnets.default.canisters.root]
kind = "root"
"#;
    fs::write(&demo, config).expect("write demo config");
    fs::write(&copy, config).expect("write copy config");

    let err = discover_canic_config_choices(&root).expect_err("duplicate app names should fail");
    let ConfigDiscoveryError::DuplicateApp { app, configs } = err else {
        panic!("expected typed duplicate app error");
    };

    assert_eq!(app, "demo");
    assert!(configs.contains("demo/canic.toml"));
    assert!(configs.contains("copy/canic.toml"));
    fs::remove_dir_all(root).expect("clean temp dir");
}
