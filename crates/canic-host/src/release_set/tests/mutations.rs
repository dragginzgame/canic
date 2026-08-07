use super::*;
use crate::release_set::AppConfigError;
use canic_core::ids::CanisterRole;
use toml::Value as TomlValue;

#[test]
fn declare_app_role_adds_declared_only_canister_role() {
    let config = r#"
[app]
name = "demo"

[roles.root]
kind = "root"
package = "root"


"#;
    let updated = declare_app_role_source(config, "demo", "store", "store").expect("declare role");

    assert_eq!(updated.role.display, "demo.store");
    assert_eq!(updated.role.package, "store");
    assert!(updated.source.contains("[roles.\"store\"]"));
    assert!(updated.source.contains("kind = \"canister\""));
    assert!(updated.source.contains("package = \"store\""));

    let lifecycle = configured_role_lifecycle_from_config(&parsed_config(&updated.source));
    let store = lifecycle
        .iter()
        .find(|role| role.role == "store")
        .expect("store row");
    assert_eq!(store.state, "declared");
    assert_eq!(store.topology, None);
}

#[test]
fn declare_app_role_rejects_root_and_duplicates() {
    declare_app_role_source(REAL_CONFIG, "demo", "root", "root")
        .expect_err("root declaration should fail");

    declare_app_role_source(REAL_CONFIG, "demo", "user_hub", "user_hub")
        .expect_err("duplicate declaration should fail");
}

#[test]
fn plan_app_role_mutations_validate_without_writing_files() {
    let temp = TempWorkspace::new();
    let config_path = temp.path().join("canic.toml");
    let hub_dir = temp.path().join("user_hub");
    fs::create_dir_all(&hub_dir).expect("create package");
    fs::write(&config_path, REAL_CONFIG).expect("write config");
    fs::write(
        hub_dir.join("Cargo.toml"),
        r#"
[package]
name = "demo_user_hub"

[package.metadata.canic]
app = "demo"
role = "user_hub"
"#,
    )
    .expect("write manifest");
    let before_config = fs::read_to_string(&config_path).expect("read config");
    let before_manifest = fs::read_to_string(hub_dir.join("Cargo.toml")).expect("read manifest");

    let declared =
        plan_declare_app_role(&config_path, "demo", "store", "store").expect("plan declare");
    let attached = plan_attach_app_role(
        &config_path,
        "demo",
        "role_baseline",
        "user_hub",
        "singleton",
    )
    .expect("plan attach");
    let renamed =
        plan_rename_app_role(&config_path, "demo", "user_hub", "user_router").expect("plan rename");

    assert_eq!(declared.display, "demo.store");
    assert_eq!(
        attached.topology,
        "user_hub/user_hub/children/role_baseline"
    );
    assert_eq!(renamed.new_display, "demo.user_router");
    assert!(renamed.package_manifest.is_some());
    assert_eq!(
        fs::read_to_string(&config_path).expect("read config after plans"),
        before_config
    );
    assert_eq!(
        fs::read_to_string(hub_dir.join("Cargo.toml")).expect("read manifest after plans"),
        before_manifest
    );
}

#[test]
fn attach_app_role_creates_component_spec_for_first_attachment() {
    let config = r#"
[app]
name = "demo"

[roles.root]
kind = "root"
package = "root"

[roles.store]
kind = "canister"
package = "store"


"#;
    let updated = attach_app_role_source(config, "demo", "store", "default", "singleton")
        .expect("attach role");

    assert_eq!(updated.role.display, "demo.store");
    assert_eq!(updated.role.topology, "default/store");
    assert_eq!(updated.role.kind, "component");
    assert!(updated.source.contains("[component_specs.\"default\"]"));
    assert!(updated.source.contains("component_role = \"store\""));
    assert!(updated.source.contains("maximum_instances = 1"));

    let lifecycle = configured_role_lifecycle_from_config(&parsed_config(&updated.source));
    let store = lifecycle
        .iter()
        .find(|role| role.role == "store")
        .expect("store row");
    assert_eq!(store.state, "attached");
    assert_eq!(store.topology.as_deref(), Some("default/store"));
}

#[test]
fn attach_app_role_preserves_explicit_supported_kind() {
    let config = r#"
[app]
name = "demo"

[roles.root]
kind = "root"
package = "root"

[roles.worker]
kind = "canister"
package = "worker"

[roles.hub]
kind = "canister"
package = "hub"

[component_specs.default]
component_role = "hub"
maximum_instances = 1

"#;
    let updated = attach_app_role_source(config, "demo", "worker", "default", "replica")
        .expect("attach role");

    assert_eq!(updated.role.kind, "replica");
    assert_eq!(updated.role.topology, "default/hub/children/worker");
    assert!(updated.source.contains("kind = \"replica\""));
    assert!(
        updated
            .source
            .contains("[component_specs.\"default\".spawn_grants.\"hub\".\"worker\"]")
    );
    assert!(updated.source.contains("maximum_instances_per_parent = 1"));
}

#[test]
fn attach_app_role_rejects_removed_service_kind() {
    let config = r#"
[app]
name = "demo"

[roles.root]
kind = "root"
package = "root"

[roles.worker]
kind = "canister"
package = "worker"


"#;
    assert!(matches!(
        attach_app_role_source(config, "demo", "worker", "default", "service")
            .expect_err("removed service kind must reject"),
        AppConfigError::InvalidKind { .. }
    ));
}

#[test]
fn attach_app_role_rejects_missing_duplicate_root_and_unknown_kind() {
    attach_app_role_source(REAL_CONFIG, "demo", "missing", "default", "singleton")
        .expect_err("missing role should fail");

    attach_app_role_source(REAL_CONFIG, "demo", "user_hub", "default", "singleton")
        .expect_err("duplicate attachment should fail");

    attach_app_role_source(REAL_CONFIG, "demo", "root", "default", "singleton")
        .expect_err("root attachment should fail");

    attach_app_role_source(REAL_CONFIG, "demo", "minimal", "default", "worker")
        .expect_err("unknown kind should fail");
}

#[test]
fn attach_app_role_allows_one_child_artifact_in_multiple_component_specs() {
    let config = r#"
[app]
name = "demo"

[roles.root]
kind = "root"
package = "root"

[roles.alpha]
kind = "canister"
package = "alpha"

[roles.beta]
kind = "canister"
package = "beta"

[roles.worker]
kind = "canister"
package = "worker"

[component_specs.alpha]
component_role = "alpha"
maximum_instances = 1

[component_specs.alpha.children.worker]
kind = "replica"

[component_specs.alpha.spawn_grants.alpha.worker]
maximum_instances_per_parent = 4

[component_specs.beta]
component_role = "beta"
maximum_instances = 1
"#;

    let updated = attach_app_role_source(config, "demo", "worker", "beta", "singleton")
        .expect("same declared child artifact may be attached to another Component Spec");

    assert!(
        updated
            .source
            .contains("[component_specs.\"beta\".children.\"worker\"]")
    );
    let parsed = parsed_config(&updated.source);
    assert_eq!(
        parsed
            .component_specs
            .values()
            .filter(|spec| spec.children.contains_key(&CanisterRole::from("worker")))
            .count(),
        2
    );
    assert_eq!(
        configured_role_kinds_from_config(&parsed)
            .get("worker")
            .map(String::as_str),
        Some("mixed")
    );
}

#[test]
fn attach_app_role_allows_a_component_role_as_another_specs_descendant() {
    let config = r#"
[app]
name = "demo"

[roles.root]
kind = "root"
package = "root"

[roles.alpha]
kind = "canister"
package = "alpha"

[roles.beta]
kind = "canister"
package = "beta"

[component_specs.alpha]
component_role = "alpha"
maximum_instances = 1

[component_specs.beta]
component_role = "beta"
maximum_instances = 1
"#;

    let updated = attach_app_role_source(config, "demo", "beta", "alpha", "instance")
        .expect("a top-level Component role may be a potential descendant in another Spec");

    assert!(
        updated
            .source
            .contains("[component_specs.\"alpha\".children.\"beta\"]")
    );
    let parsed = parsed_config(&updated.source);
    assert!(
        parsed
            .component_specs
            .get("alpha")
            .expect("alpha Component Spec")
            .children
            .contains_key(&CanisterRole::from("beta"))
    );
    assert_eq!(
        configured_role_kinds_from_config(&parsed)
            .get("beta")
            .map(String::as_str),
        Some("mixed")
    );
}

#[test]
fn rename_app_role_updates_declaration_topology_and_package_metadata() {
    let temp = TempWorkspace::new();
    let config_path = temp.path().join("canic.toml");
    let package_dir = temp.path().join("hub");
    fs::create_dir_all(&package_dir).expect("create package");
    fs::write(
        package_dir.join("Cargo.toml"),
        r#"
[package]
name = "demo_hub"

[package.metadata.canic]
app = "demo"
role = "hub"
"#,
    )
    .expect("write manifest");
    let config = r#"
[app]
name = "demo"

[roles.root]
kind = "root"
package = "root"

[roles.hub]
kind = "canister"
package = "hub"

[roles.worker]
kind = "canister"
package = "worker"



[component_specs.hub]
component_role = "hub"
maximum_instances = 1

[component_specs.hub.sharding.pools.primary]
canister_role = "worker"

[component_specs.hub.children.worker]
kind = "shard"

[component_specs.hub.spawn_grants.hub.worker]
maximum_instances_per_parent = 4
"#;
    let updated =
        rename_app_role_source(config, &config_path, "demo", "hub", "router").expect("rename role");

    assert_eq!(updated.role.old_display, "demo.hub");
    assert_eq!(updated.role.new_display, "demo.router");
    assert_eq!(
        updated
            .role
            .package_manifest
            .as_deref()
            .and_then(Path::file_name)
            .and_then(std::ffi::OsStr::to_str),
        Some("Cargo.toml")
    );
    assert!(updated.source.contains("[\"roles\".\"router\"]"));
    assert!(updated.source.contains("component_role = \"router\""));
    assert!(
        updated
            .source
            .contains("[component_specs.hub.children.worker]")
    );
    assert!(!updated.source.contains("[roles.hub]"));
    assert!(
        updated
            .package_source
            .as_deref()
            .is_some_and(|source| source.contains("role = \"router\""))
    );
    let package_source = updated.package_source.as_deref().expect("package source");
    let package_manifest =
        toml::from_str::<TomlValue>(package_source).expect("updated package manifest parses");
    let package_canic = package_manifest
        .get("package")
        .and_then(TomlValue::as_table)
        .and_then(|package| package.get("metadata"))
        .and_then(TomlValue::as_table)
        .and_then(|metadata| metadata.get("canic"))
        .and_then(TomlValue::as_table)
        .expect("updated canic metadata");
    assert_eq!(
        package_canic.get("app").and_then(TomlValue::as_str),
        Some("demo")
    );
    assert_eq!(
        package_canic.get("role").and_then(TomlValue::as_str),
        Some("router")
    );

    let lifecycle = configured_role_lifecycle_from_config(&parsed_config(&updated.source));
    assert!(lifecycle.iter().any(|role| role.role == "router"));
    assert!(!lifecycle.iter().any(|role| role.role == "hub"));
}

#[test]
fn rename_app_role_updates_role_bearing_references() {
    let config = r#"
[app]
name = "demo"

[roles.root]
kind = "root"
package = "root"

[roles.hub]
kind = "canister"
package = "hub"

[roles.worker]
kind = "canister"
package = "worker"



[component_specs.hub]
component_role = "hub"
maximum_instances = 1

[component_specs.hub.sharding.pools.primary]
canister_role = "worker"

[component_specs.hub.children.worker]
kind = "shard"

[component_specs.hub.spawn_grants.hub.worker]
maximum_instances_per_parent = 4
"#;
    let config_path = Path::new("canic.toml");
    let updated = rename_app_role_source(config, config_path, "demo", "worker", "worker_v2")
        .expect("rename role");

    assert!(updated.source.contains("canister_role = \"worker_v2\""));
    assert!(updated.source.contains("[\"roles\".\"worker_v2\"]"));
    assert!(
        updated
            .source
            .contains("[\"component_specs\".\"hub\".\"children\".\"worker_v2\"]")
    );
    assert!(
        updated
            .source
            .contains("[\"component_specs\".\"hub\".\"spawn_grants\".\"hub\".\"worker_v2\"]")
    );
}

#[test]
fn rename_app_role_rejects_root_missing_duplicate_and_same_role() {
    rename_app_role_source(
        REAL_CONFIG,
        Path::new("canic.toml"),
        "demo",
        "user_hub",
        "scale_hub",
    )
    .expect_err("duplicate rename should fail");

    rename_app_role_source(
        REAL_CONFIG,
        Path::new("canic.toml"),
        "demo",
        "missing",
        "renamed",
    )
    .expect_err("missing rename should fail");

    rename_app_role_source(REAL_CONFIG, Path::new("canic.toml"), "demo", "root", "app")
        .expect_err("root rename should fail");

    rename_app_role_source(
        REAL_CONFIG,
        Path::new("canic.toml"),
        "demo",
        "user_hub",
        "user_hub",
    )
    .expect_err("same rename should fail");
}
