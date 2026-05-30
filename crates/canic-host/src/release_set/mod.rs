//! Release-set discovery, manifest emission, and staging helpers.

use std::time::{SystemTime, UNIX_EPOCH};

mod config;
mod manifest;
mod paths;
mod stage;

pub use config::{
    AttachedFleetRole, ConfiguredPoolExpectation, ConfiguredRoleLifecycle, DeclaredFleetRole,
    LOCAL_ROOT_MIN_READY_CYCLES, RenamedFleetRole, attach_fleet_role, configured_bootstrap_roles,
    configured_controllers, configured_deployable_roles, configured_fleet_name,
    configured_install_targets, configured_local_root_create_cycles, configured_pool_expectations,
    configured_release_roles, configured_role_auto_create, configured_role_capabilities,
    configured_role_details, configured_role_kinds, configured_role_lifecycle,
    configured_role_metrics_profiles, configured_role_topups, declare_fleet_role,
    matching_fleet_config_paths, rename_fleet_role,
};
pub use manifest::{
    ReleaseSetEntry, RootReleaseSetManifest, emit_root_release_set_manifest,
    emit_root_release_set_manifest_if_ready, emit_root_release_set_manifest_with_config,
    load_root_release_set_manifest,
};
pub use paths::{
    canister_manifest_path, canisters_root, config_path, display_workspace_path, icp_root,
    load_root_package_version, load_workspace_package_version, resolve_artifact_root,
    root_manifest_path, root_release_set_manifest_path, workspace_manifest_path, workspace_root,
};
use stage::build_release_set_entry;
pub(crate) use stage::icp_query_on_network;
pub use stage::{resume_root_bootstrap, stage_root_release_set};

#[cfg(test)]
use stage::read_release_artifact;

#[cfg(test)]
use config::{
    attach_fleet_role_source, configured_bootstrap_roles_from_source,
    configured_controllers_from_source, configured_deployable_roles_from_source,
    configured_fleet_name_from_source, configured_local_root_create_cycles_from_source,
    configured_pool_expectations_from_source, configured_release_roles_from_source,
    configured_role_auto_create_from_source, configured_role_capabilities_from_source,
    configured_role_details_from_source, configured_role_kinds_from_source,
    configured_role_lifecycle_from_source, configured_role_metrics_profiles_from_source,
    configured_role_topups_from_source, declare_fleet_role_source, rename_fleet_role_source,
};

pub(super) const CANISTERS_ROOT_RELATIVE: &str = "fleets";
pub(super) const ROOT_CONFIG_FILE: &str = "canic.toml";
pub(super) const WORKSPACE_MANIFEST_RELATIVE: &str = "Cargo.toml";
pub const ROOT_RELEASE_SET_MANIFEST_FILE: &str = "root.release-set.json";
pub(super) const GZIP_MAGIC: [u8; 2] = [0x1f, 0x8b];
pub(super) const WASM_MAGIC: [u8; 4] = [0x00, 0x61, 0x73, 0x6d];

// Read the current host wall clock so staged manifests use a stable whole-second
// timestamp without depending on an exported root time endpoint.
pub(super) fn root_time_secs(root_canister: &str) -> Result<u64, Box<dyn std::error::Error>> {
    let _ = root_canister;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|err| format!("system clock before unix epoch: {err}"))?;
    Ok(now.as_secs())
}

#[cfg(test)]
mod tests {
    use super::{
        attach_fleet_role_source, canister_manifest_path, canisters_root, config_path,
        configured_bootstrap_roles_from_source, configured_controllers_from_source,
        configured_deployable_roles_from_source, configured_fleet_name_from_source,
        configured_install_targets, configured_local_root_create_cycles_from_source,
        configured_pool_expectations_from_source, configured_release_roles_from_source,
        configured_role_auto_create_from_source, configured_role_capabilities_from_source,
        configured_role_details_from_source, configured_role_kinds_from_source,
        configured_role_lifecycle_from_source, configured_role_metrics_profiles_from_source,
        configured_role_topups_from_source, declare_fleet_role_source, read_release_artifact,
        rename_fleet_role_source, root_manifest_path,
    };
    use crate::test_support::temp_dir;
    use flate2::{Compression, write::GzEncoder};
    use std::{
        fs,
        io::Write,
        path::{Path, PathBuf},
        sync::{Mutex, OnceLock},
    };

    static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    const REAL_CONFIG: &str = r#"
controllers = []
app_index = ["user_hub", "scale_hub"]

[fleet]
name = "demo"

[roles.root]
kind = "root"

[roles.app]
kind = "canister"

[roles.user_hub]
kind = "canister"

[roles.user_shard]
kind = "canister"

[roles.project_instance]
kind = "canister"

[roles.scale_hub]
kind = "canister"

[roles.scale_replica]
kind = "canister"

[roles.minimal]
kind = "canister"

[app]
init_mode = "enabled"
[app.whitelist]

[auth.delegated_tokens]
enabled = true
ecdsa_key_name = "key_1"

[standards]
icrc21 = true

[subnets.prime.canisters.root]
kind = "root"

[subnets.prime.canisters.user_hub]
kind = "singleton"

[subnets.prime.canisters.scale_hub]
kind = "singleton"
"#;

    const MULTI_ROOT_CONFIG: &str = r#"
controllers = []
app_index = []

[fleet]
name = "demo"

[roles.root]
kind = "root"

[app]
init_mode = "enabled"
[app.whitelist]

[subnets.prime.canisters.root]
kind = "root"

[subnets.secondary.canisters.root]
kind = "root"
"#;

    const NO_ROOT_CONFIG: &str = r#"
controllers = []
app_index = []

[fleet]
name = "demo"

[roles.user_hub]
kind = "canister"

[app]
init_mode = "enabled"
[app.whitelist]

[subnets.prime.canisters.user_hub]
kind = "singleton"
"#;

    fn with_guarded_env<T>(test: impl FnOnce() -> T) -> T {
        let lock = ENV_LOCK.get_or_init(|| Mutex::new(()));
        let _guard = lock.lock().unwrap();
        test()
    }

    fn restore_env(key: &str, previous: Option<std::ffi::OsString>) {
        unsafe {
            if let Some(value) = previous {
                std::env::set_var(key, value);
            } else {
                std::env::remove_var(key);
            }
        }
    }

    struct TempWorkspace {
        path: PathBuf,
    }

    impl TempWorkspace {
        fn new() -> Self {
            let path = temp_dir("canic-host-release-set-tests");
            fs::create_dir_all(&path).expect("create temp workspace");
            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TempWorkspace {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn configured_release_roles_filters_root_and_wasm_store() {
        let config = r#"
controllers = []
app_index = []

[fleet]
name = "demo"

[roles.root]
kind = "root"

[roles.user_hub]
kind = "canister"

[roles.scale_hub]
kind = "canister"

[app]
init_mode = "enabled"
[app.whitelist]

[subnets.prime.canisters.root]
kind = "root"

[subnets.prime.canisters.user_hub]
kind = "singleton"

[subnets.prime.canisters.scale_hub]
kind = "singleton"
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
kind = "singleton"
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
    fn configured_role_kinds_lists_configured_roles() {
        let kinds = configured_role_kinds_from_source(REAL_CONFIG).expect("role kinds");

        assert_eq!(kinds.get("root").map(String::as_str), Some("root"));
        assert_eq!(kinds.get("user_hub").map(String::as_str), Some("singleton"));
        assert_eq!(
            kinds.get("scale_hub").map(String::as_str),
            Some("singleton")
        );
    }

    #[test]
    fn configured_role_lifecycle_lists_declared_and_attached_roles() {
        let config = r#"
controllers = []
app_index = []

[fleet]
name = "demo"

[roles.root]
kind = "root"
package = "canisters/root"

[roles.user_hub]
kind = "canister"
package = "canisters/user_hub"

[roles.user_shard]
kind = "canister"
package = "canisters/user_shard"

[roles.store]
kind = "canister"
package = "canisters/store"

[subnets.prime.canisters.root]
kind = "root"

[subnets.prime.canisters.user_hub]
kind = "singleton"

[subnets.prime.canisters.user_hub.sharding.pools.users]
canister_role = "user_shard"

[subnets.prime.canisters.user_shard]
kind = "shard"
"#;
        let lifecycle = configured_role_lifecycle_from_source(config).expect("role lifecycle");

        let root = lifecycle
            .iter()
            .find(|role| role.role == "root")
            .expect("root lifecycle row");
        assert_eq!(root.display, "demo.root");
        assert_eq!(root.state, "attached");
        assert_eq!(root.topology.as_deref(), Some("prime/root"));

        let shard = lifecycle
            .iter()
            .find(|role| role.role == "user_shard")
            .expect("shard lifecycle row");
        assert_eq!(shard.state, "attached");
        assert_eq!(
            shard.topology.as_deref(),
            Some("prime/user_hub/sharding/users,prime/user_shard")
        );

        let store = lifecycle
            .iter()
            .find(|role| role.role == "store")
            .expect("store lifecycle row");
        assert_eq!(store.package.as_deref(), Some("canisters/store"));
        assert_eq!(store.state, "declared");
        assert_eq!(store.topology, None);
    }

    #[test]
    fn declare_fleet_role_adds_declared_only_canister_role() {
        let config = r#"
controllers = []
app_index = []

[fleet]
name = "demo"

[roles.root]
kind = "root"
package = "root"

[subnets.prime.canisters.root]
kind = "root"
"#;
        let updated =
            declare_fleet_role_source(config, "demo", "store", "store").expect("declare role");

        assert_eq!(updated.role.display, "demo.store");
        assert_eq!(updated.role.package, "store");
        assert!(updated.source.contains("[roles.\"store\"]"));
        assert!(updated.source.contains("kind = \"canister\""));
        assert!(updated.source.contains("package = \"store\""));

        let lifecycle =
            configured_role_lifecycle_from_source(&updated.source).expect("role lifecycle");
        let store = lifecycle
            .iter()
            .find(|role| role.role == "store")
            .expect("store row");
        assert_eq!(store.state, "declared");
        assert_eq!(store.topology, None);
    }

    #[test]
    fn declare_fleet_role_rejects_root_and_duplicates() {
        let root_err = declare_fleet_role_source(REAL_CONFIG, "demo", "root", "root")
            .expect_err("root declaration should fail")
            .to_string();
        assert!(root_err.contains("root role must be attached"));

        let duplicate_err = declare_fleet_role_source(REAL_CONFIG, "demo", "user_hub", "user_hub")
            .expect_err("duplicate declaration should fail")
            .to_string();
        assert!(duplicate_err.contains("already declared"));
    }

    #[test]
    fn attach_fleet_role_adds_direct_topology_attachment() {
        let config = r#"
controllers = []
app_index = []

[fleet]
name = "demo"

[roles.root]
kind = "root"
package = "root"

[roles.store]
kind = "canister"
package = "store"

[subnets.prime.canisters.root]
kind = "root"
"#;
        let updated = attach_fleet_role_source(config, "demo", "store", "prime", "singleton")
            .expect("attach role");

        assert_eq!(updated.role.display, "demo.store");
        assert_eq!(updated.role.topology, "prime/store");
        assert!(
            updated
                .source
                .contains("[subnets.\"prime\".canisters.\"store\"]")
        );
        assert!(updated.source.contains("kind = \"singleton\""));

        let lifecycle =
            configured_role_lifecycle_from_source(&updated.source).expect("role lifecycle");
        let store = lifecycle
            .iter()
            .find(|role| role.role == "store")
            .expect("store row");
        assert_eq!(store.state, "attached");
        assert_eq!(store.topology.as_deref(), Some("prime/store"));
    }

    #[test]
    fn attach_fleet_role_preserves_explicit_supported_kind() {
        let config = r#"
controllers = []
app_index = []

[fleet]
name = "demo"

[roles.root]
kind = "root"
package = "root"

[roles.worker]
kind = "canister"
package = "worker"

[subnets.prime.canisters.root]
kind = "root"
"#;
        let updated = attach_fleet_role_source(config, "demo", "worker", "prime", "replica")
            .expect("attach role");

        assert_eq!(updated.role.kind, "replica");
        assert_eq!(updated.role.topology, "prime/worker");
        assert!(updated.source.contains("kind = \"replica\""));
    }

    #[test]
    fn attach_fleet_role_rejects_missing_duplicate_root_and_unknown_kind() {
        let missing_err =
            attach_fleet_role_source(REAL_CONFIG, "demo", "missing", "prime", "singleton")
                .expect_err("missing role should fail")
                .to_string();
        assert!(missing_err.contains("is not declared"));

        let duplicate_err =
            attach_fleet_role_source(REAL_CONFIG, "demo", "user_hub", "prime", "singleton")
                .expect_err("duplicate attachment should fail")
                .to_string();
        assert!(duplicate_err.contains("already attached"));

        let root_err = attach_fleet_role_source(REAL_CONFIG, "demo", "root", "prime", "singleton")
            .expect_err("root attachment should fail")
            .to_string();
        assert!(root_err.contains("root role must already be attached"));

        let kind_err = attach_fleet_role_source(REAL_CONFIG, "demo", "minimal", "prime", "service")
            .expect_err("unknown kind should fail")
            .to_string();
        assert!(kind_err.contains("kind must be one of"));
    }

    #[test]
    fn rename_fleet_role_updates_declaration_topology_and_package_metadata() {
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
fleet = "demo"
role = "hub"
"#,
        )
        .expect("write manifest");
        let config = r#"
controllers = []
app_index = ["hub"]

[fleet]
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

[subnets.prime.canisters.root]
kind = "root"

[subnets.prime.canisters.hub]
kind = "singleton"

[subnets.prime.canisters.hub.sharding.pools.primary]
canister_role = "worker"

[subnets.prime.canisters.worker]
kind = "shard"
"#;
        let updated = rename_fleet_role_source(config, &config_path, "demo", "hub", "router")
            .expect("rename role");

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
        assert!(
            updated
                .source
                .contains("[\"subnets\".\"prime\".\"canisters\".\"router\"]")
        );
        assert!(updated.source.contains(
            "[\"subnets\".\"prime\".\"canisters\".\"router\".\"sharding\".\"pools\".\"primary\"]"
        ));
        assert!(updated.source.contains("app_index = [\"router\"]"));
        assert!(!updated.source.contains("[roles.hub]"));
        assert!(
            updated
                .package_source
                .as_deref()
                .is_some_and(|source| source.contains("role = \"router\""))
        );

        let lifecycle =
            configured_role_lifecycle_from_source(&updated.source).expect("role lifecycle");
        assert!(lifecycle.iter().any(|role| role.role == "router"));
        assert!(!lifecycle.iter().any(|role| role.role == "hub"));
    }

    #[test]
    fn rename_fleet_role_updates_role_bearing_references() {
        let config = r#"
controllers = []
app_index = ["hub"]

[fleet]
name = "demo"

[roles.root]
kind = "root"

[roles.hub]
kind = "canister"

[roles.worker]
kind = "canister"

[subnets.prime.canisters.root]
kind = "root"

[subnets.prime.canisters.hub]
kind = "singleton"

[subnets.prime.canisters.hub.sharding.pools.primary]
canister_role = "worker"

[subnets.prime.canisters.worker]
kind = "shard"
"#;
        let config_path = Path::new("canic.toml");
        let updated = rename_fleet_role_source(config, config_path, "demo", "worker", "worker_v2")
            .expect("rename role");

        assert!(updated.source.contains("canister_role = \"worker_v2\""));
        assert!(updated.source.contains("[\"roles\".\"worker_v2\"]"));
        assert!(
            updated
                .source
                .contains("[\"subnets\".\"prime\".\"canisters\".\"worker_v2\"]")
        );
    }

    #[test]
    fn rename_fleet_role_rejects_root_missing_duplicate_and_same_role() {
        let duplicate_err = rename_fleet_role_source(
            REAL_CONFIG,
            Path::new("canic.toml"),
            "demo",
            "user_hub",
            "scale_hub",
        )
        .expect_err("duplicate rename should fail")
        .to_string();
        assert!(duplicate_err.contains("already declared"));

        let missing_err = rename_fleet_role_source(
            REAL_CONFIG,
            Path::new("canic.toml"),
            "demo",
            "missing",
            "renamed",
        )
        .expect_err("missing rename should fail")
        .to_string();
        assert!(missing_err.contains("is not declared"));

        let root_err =
            rename_fleet_role_source(REAL_CONFIG, Path::new("canic.toml"), "demo", "root", "app")
                .expect_err("root rename should fail")
                .to_string();
        assert!(root_err.contains("root role cannot be renamed"));

        let same_err = rename_fleet_role_source(
            REAL_CONFIG,
            Path::new("canic.toml"),
            "demo",
            "user_hub",
            "user_hub",
        )
        .expect_err("same rename should fail")
        .to_string();
        assert!(same_err.contains("must differ"));
    }

    #[test]
    fn configured_role_capabilities_lists_enabled_role_features() {
        let config = r#"
controllers = []
app_index = ["user_hub", "scale_hub"]

[fleet]
name = "demo"

[roles.root]
kind = "root"

[roles.app]
kind = "canister"

[roles.user_hub]
kind = "canister"

[roles.user_shard]
kind = "canister"

[roles.project_instance]
kind = "canister"

[roles.scale_hub]
kind = "canister"

[roles.scale_replica]
kind = "canister"

[roles.minimal]
kind = "canister"

[app]
init_mode = "enabled"
[app.whitelist]

[subnets.prime.canisters.root]
kind = "root"

[subnets.prime.canisters.user_hub]
kind = "singleton"

[subnets.prime.canisters.user_hub.sharding.pools.user_shards]
canister_role = "user_shard"
policy.capacity = 100
policy.max_shards = 4

[subnets.prime.canisters.user_shard]
kind = "shard"

[subnets.prime.canisters.user_shard.auth]
delegated_token_signer = true

[subnets.prime.canisters.scale_hub]
kind = "singleton"

[subnets.prime.canisters.scale_hub.scaling.pools.scales]
canister_role = "scale_replica"

[subnets.prime.canisters.scale_replica]
kind = "replica"
"#;
        let capabilities =
            configured_role_capabilities_from_source(config).expect("role capabilities");

        assert_eq!(
            capabilities.get("user_hub"),
            Some(&vec!["sharding".to_string()])
        );
        assert_eq!(
            capabilities.get("user_shard"),
            Some(&vec!["auth".to_string()])
        );
        assert_eq!(
            capabilities.get("scale_hub"),
            Some(&vec!["scaling".to_string()])
        );
        assert!(!capabilities.contains_key("root"));
    }

    #[test]
    fn configured_pool_expectations_lists_root_subnet_pools() {
        let config = r#"
controllers = []
app_index = []

[fleet]
name = "demo"

[roles.root]
kind = "root"

[roles.app]
kind = "canister"

[roles.user_hub]
kind = "canister"

[roles.user_shard]
kind = "canister"

[roles.project_instance]
kind = "canister"

[roles.scale_hub]
kind = "canister"

[roles.scale_replica]
kind = "canister"

[roles.minimal]
kind = "canister"

[app]
init_mode = "enabled"
[app.whitelist]

[subnets.prime.canisters.root]
kind = "root"

[subnets.prime.canisters.user_hub]
kind = "singleton"

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
kind = "singleton"

[subnets.prime.canisters.scale_hub.scaling.pools.scales]
canister_role = "scale_replica"

[subnets.prime.canisters.scale_replica]
kind = "replica"
"#;
        let pools = configured_pool_expectations_from_source(config).expect("pool expectations");

        assert_eq!(pools.len(), 3);
        assert!(
            pools
                .iter()
                .any(|pool| { pool.pool == "user_shards" && pool.canister_role == "user_shard" })
        );
        assert!(
            pools.iter().any(|pool| {
                pool.pool == "projects" && pool.canister_role == "project_instance"
            })
        );
        assert!(
            pools
                .iter()
                .any(|pool| { pool.pool == "scales" && pool.canister_role == "scale_replica" })
        );
    }

    #[test]
    fn configured_role_metrics_profiles_lists_resolved_profiles() {
        let config = r#"
controllers = []
app_index = []

[fleet]
name = "demo"

[roles.root]
kind = "root"

[roles.app]
kind = "canister"

[roles.user_hub]
kind = "canister"

[roles.user_shard]
kind = "canister"

[roles.project_instance]
kind = "canister"

[roles.scale_hub]
kind = "canister"

[roles.scale_replica]
kind = "canister"

[roles.minimal]
kind = "canister"

[subnets.prime.canisters.root]
kind = "root"

[subnets.prime.canisters.user_hub]
kind = "singleton"

[subnets.prime.canisters.user_hub.sharding.pools.user_shards]
canister_role = "user_shard"

[subnets.prime.canisters.user_shard]
kind = "shard"

[subnets.prime.canisters.scale_replica]
kind = "replica"

[subnets.prime.canisters.scale_replica.metrics]
profile = "full"
"#;
        let profiles =
            configured_role_metrics_profiles_from_source(config).expect("metrics profiles");

        assert_eq!(profiles.get("root").map(String::as_str), Some("root"));
        assert_eq!(profiles.get("user_hub").map(String::as_str), Some("hub"));
        assert_eq!(profiles.get("user_shard").map(String::as_str), Some("leaf"));
        assert_eq!(
            profiles.get("scale_replica").map(String::as_str),
            Some("full")
        );
    }

    #[test]
    fn configured_role_details_lists_verbose_config_features() {
        let config = r#"
controllers = []
app_index = ["user_hub", "scale_hub"]

[fleet]
name = "demo"

[roles.root]
kind = "root"

[roles.app]
kind = "canister"

[roles.user_hub]
kind = "canister"

[roles.user_shard]
kind = "canister"

[roles.project_instance]
kind = "canister"

[roles.scale_hub]
kind = "canister"

[roles.scale_replica]
kind = "canister"

[roles.minimal]
kind = "canister"

[app]
init_mode = "enabled"
[app.whitelist]

[subnets.prime.canisters.root]
kind = "root"

[subnets.prime.canisters.user_hub]
kind = "singleton"
topup.threshold = "10T"
topup.amount = "4T"

[subnets.prime.canisters.user_hub.sharding.pools.user_shards]
canister_role = "user_shard"
policy.capacity = 100
policy.max_shards = 4

[subnets.prime.canisters.user_shard]
kind = "shard"
randomness.enabled = false

[subnets.prime.canisters.user_shard.auth]
delegated_token_signer = true
role_attestation_cache = true

[subnets.prime.canisters.scale_hub]
kind = "singleton"

[subnets.prime.canisters.scale_hub.scaling.pools.scales]
canister_role = "scale_replica"
policy.initial_workers = 2
policy.min_workers = 2

[subnets.prime.canisters.scale_replica]
kind = "replica"

[subnets.prime.canisters.scale_replica.metrics]
profile = "full"
"#;
        let details = configured_role_details_from_source(config).expect("role details");

        assert!(
            details
                .get("user_hub")
                .is_some_and(|details| details.contains(&"app_index".to_string()))
        );
        assert!(details.get("user_hub").is_some_and(|details| {
            details
                .iter()
                .any(|detail| detail == "sharding user_shards->user_shard cap=100 initial=1 max=4")
        }));
        assert!(
            details
                .get("user_shard")
                .is_some_and(|details| details.contains(&"auth delegated-token-signer".to_string()))
        );
        assert!(details.get("scale_hub").is_some_and(|details| {
            details.contains(&"scaling scales->scale_replica initial=2 min=2 max=32".to_string())
        }));
        assert!(details.get("user_hub").is_some_and(|details| {
            details.contains(
                &"metrics profile=hub tiers=core,placement,runtime,security (inferred)".to_string(),
            )
        }));
        assert!(details.get("scale_replica").is_some_and(|details| {
            details.contains(
                &"metrics profile=full tiers=core,placement,platform,runtime,security,storage (configured)"
                    .to_string()
            )
        }));
    }

    #[test]
    fn configured_role_topups_lists_configured_policy_summaries() {
        let config = r#"
controllers = []
app_index = []

[fleet]
name = "demo"

[roles.root]
kind = "root"

[roles.app]
kind = "canister"

[roles.user_hub]
kind = "canister"

[roles.user_shard]
kind = "canister"

[roles.project_instance]
kind = "canister"

[roles.scale_hub]
kind = "canister"

[roles.scale_replica]
kind = "canister"

[roles.minimal]
kind = "canister"

[app]
init_mode = "enabled"
[app.whitelist]

[subnets.prime.canisters.root]
kind = "root"

[subnets.prime.canisters.scale_hub]
kind = "singleton"
topup.threshold = "10T"
topup.amount = "4T"
"#;
        let topups = configured_role_topups_from_source(config).expect("role topups");

        assert_eq!(
            topups.get("scale_hub").map(String::as_str),
            Some("4.00 TC @ 10.00 TC")
        );
        assert!(!topups.contains_key("root"));
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

[roles.app]
kind = "canister"

[roles.user_hub]
kind = "canister"

[roles.user_shard]
kind = "canister"

[roles.project_instance]
kind = "canister"

[roles.scale_hub]
kind = "canister"

[roles.scale_replica]
kind = "canister"

[roles.minimal]
kind = "canister"

[subnets.prime]
pool.minimum_size = 2

[subnets.prime.canisters.root]
kind = "root"

[subnets.prime.canisters.app]
kind = "singleton"
initial_cycles = "7T"

[subnets.prime.canisters.user_hub]
kind = "singleton"
"#;

        let cycles = configured_local_root_create_cycles_from_source(config).expect("cycles");

        assert_eq!(cycles, 127_000_000_000_000);
    }

    #[test]
    fn configured_role_auto_create_lists_derived_singleton_roles() {
        let config = r#"
controllers = []
app_index = []

[fleet]
name = "demo"

[roles.root]
kind = "root"

[roles.app]
kind = "canister"

[roles.user_hub]
kind = "canister"

[roles.user_shard]
kind = "canister"

[roles.project_instance]
kind = "canister"

[roles.scale_hub]
kind = "canister"

[roles.scale_replica]
kind = "canister"

[roles.minimal]
kind = "canister"

[app]
init_mode = "enabled"
[app.whitelist]

[subnets.prime.canisters.root]
kind = "root"

[subnets.prime.canisters.app]
kind = "singleton"

[subnets.prime.canisters.user_hub]
kind = "singleton"
"#;
        let auto_create =
            configured_role_auto_create_from_source(config).expect("auto create roles");

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

[roles.app]
kind = "canister"

[roles.user_hub]
kind = "canister"

[roles.user_shard]
kind = "canister"

[roles.project_instance]
kind = "canister"

[roles.scale_hub]
kind = "canister"

[roles.scale_replica]
kind = "canister"

[roles.minimal]
kind = "canister"

[app]
init_mode = "enabled"
[app.whitelist]

[subnets.prime.canisters.root]
kind = "root"

[subnets.prime.canisters.app]
kind = "singleton"

[subnets.prime.canisters.user_hub]
kind = "singleton"

[subnets.prime.canisters.user_hub.sharding.pools.user_shards]
canister_role = "user_shard"
policy.capacity = 100
policy.initial_shards = 1
policy.max_shards = 4

[subnets.prime.canisters.user_shard]
kind = "shard"

[subnets.prime.canisters.minimal]
kind = "replica"
"#;
        let roles = configured_bootstrap_roles_from_source(config).expect("bootstrap roles");

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

    #[test]
    fn configured_fleet_name_reads_required_config_identity() {
        let name = configured_fleet_name_from_source(REAL_CONFIG).expect("fleet name");

        assert_eq!(name, "demo");
    }

    #[test]
    fn configured_fleet_name_rejects_missing_config_identity() {
        let err = configured_fleet_name_from_source(
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
        .unwrap_err();

        assert!(
            err.to_string()
                .contains("missing required [fleet].name in canic.toml"),
            "unexpected error: {err}"
        );
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

[roles.app]
kind = "canister"

[roles.user_hub]
kind = "canister"

[roles.user_shard]
kind = "canister"

[roles.project_instance]
kind = "canister"

[roles.scale_hub]
kind = "canister"

[roles.scale_replica]
kind = "canister"

[roles.minimal]
kind = "canister"

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

    #[test]
    fn configured_release_roles_rejects_multiple_root_subnets() {
        let err = configured_release_roles_from_source(MULTI_ROOT_CONFIG).unwrap_err();
        assert!(
            err.to_string()
                .contains("root kind must be unique globally"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn configured_release_roles_rejects_missing_root() {
        let err = configured_release_roles_from_source(NO_ROOT_CONFIG).unwrap_err();
        assert!(
            err.to_string().contains("root role declaration missing"),
            "unexpected error: {err}"
        );
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

    #[test]
    fn canisters_root_follows_config_parent_when_manifest_metadata_is_unavailable() {
        with_guarded_env(|| {
            let temp = TempWorkspace::new();
            let workspace_root = temp.path();
            let config_dir = workspace_root.join("custom");
            fs::create_dir_all(&config_dir).expect("create config dir");
            let config_file = config_dir.join("override.toml");
            fs::write(&config_file, "").expect("write config");

            let previous = std::env::var_os("CANIC_CONFIG_PATH");
            unsafe {
                std::env::set_var("CANIC_CONFIG_PATH", &config_file);
            }
            let result = canisters_root(workspace_root);
            unsafe {
                if let Some(value) = previous {
                    std::env::set_var("CANIC_CONFIG_PATH", value);
                } else {
                    std::env::remove_var("CANIC_CONFIG_PATH");
                }
            }

            assert_eq!(result, config_dir);
        });
    }

    #[test]
    fn config_path_defaults_under_fleets_root() {
        with_guarded_env(|| {
            let temp = TempWorkspace::new();
            let workspace_root = temp.path();
            let fleets_dir = workspace_root.join("fleets");
            fs::create_dir_all(&fleets_dir).expect("create fleets dir");
            let expected = fleets_dir.join("canic.toml");

            let previous = std::env::var_os("CANIC_CONFIG_PATH");
            unsafe {
                std::env::remove_var("CANIC_CONFIG_PATH");
            }
            let result = config_path(workspace_root);
            unsafe {
                if let Some(value) = previous {
                    std::env::set_var("CANIC_CONFIG_PATH", value);
                }
            }

            assert_eq!(result, expected);
        });
    }

    #[test]
    fn root_manifest_path_prefers_canister_manifest_metadata() {
        with_guarded_env(|| {
            let temp = TempWorkspace::new();
            let workspace_root = temp.path();
            fs::create_dir_all(workspace_root.join("fleets/test/root")).expect("create root dir");
            fs::create_dir_all(workspace_root.join("fleets/test/root/src"))
                .expect("create root src dir");
            fs::write(
                workspace_root.join("Cargo.toml"),
                "[workspace]\nmembers = [\"fleets/test/root\"]\n",
            )
            .expect("write workspace manifest");
            fs::write(
                workspace_root.join("fleets/test/root/Cargo.toml"),
                r#"[package]
name = "canister_root"
version = "0.1.0"
edition = "2024"

[package.metadata.canic]
role = "root"
"#,
            )
            .expect("write root manifest");
            fs::write(workspace_root.join("fleets/test/root/src/lib.rs"), "")
                .expect("write root lib");

            let previous_config = std::env::var_os("CANIC_CONFIG_PATH");
            let previous_root = std::env::var_os("CANIC_CANISTERS_ROOT");
            unsafe {
                std::env::remove_var("CANIC_CONFIG_PATH");
                std::env::remove_var("CANIC_CANISTERS_ROOT");
            }
            let result = root_manifest_path(workspace_root).expect("root manifest path");
            restore_env("CANIC_CONFIG_PATH", previous_config);
            restore_env("CANIC_CANISTERS_ROOT", previous_root);

            assert_eq!(result, workspace_root.join("fleets/test/root/Cargo.toml"));
        });
    }

    #[test]
    fn canister_manifest_path_prefers_canister_manifest_metadata() {
        with_guarded_env(|| {
            let temp = TempWorkspace::new();
            let workspace_root = temp.path();
            fs::create_dir_all(workspace_root.join("fleets/test/user_hub"))
                .expect("create user hub dir");
            fs::create_dir_all(workspace_root.join("fleets/test/user_hub/src"))
                .expect("create user hub src dir");
            fs::write(
                workspace_root.join("Cargo.toml"),
                "[workspace]\nmembers = [\"fleets/test/user_hub\"]\n",
            )
            .expect("write workspace manifest");
            fs::write(
                workspace_root.join("fleets/test/user_hub/Cargo.toml"),
                r#"[package]
name = "canister_user_hub"
version = "0.1.0"
edition = "2024"

[package.metadata.canic]
role = "user_hub"
"#,
            )
            .expect("write user hub manifest");
            fs::write(workspace_root.join("fleets/test/user_hub/src/lib.rs"), "")
                .expect("write user hub lib");

            let previous_config = std::env::var_os("CANIC_CONFIG_PATH");
            let previous_root = std::env::var_os("CANIC_CANISTERS_ROOT");
            unsafe {
                std::env::remove_var("CANIC_CONFIG_PATH");
                std::env::remove_var("CANIC_CANISTERS_ROOT");
            }
            let result =
                canister_manifest_path(workspace_root, "user_hub").expect("user hub manifest path");
            restore_env("CANIC_CONFIG_PATH", previous_config);
            restore_env("CANIC_CANISTERS_ROOT", previous_root);

            assert_eq!(
                result,
                workspace_root.join("fleets/test/user_hub/Cargo.toml")
            );
        });
    }

    #[test]
    fn canister_manifest_path_uses_declared_canic_role_metadata() {
        with_guarded_env(|| {
            let temp = TempWorkspace::new();
            let workspace_root = temp.path();
            fs::create_dir_all(workspace_root.join("fleets/test/scale")).expect("create scale dir");
            fs::create_dir_all(workspace_root.join("fleets/test/scale/src"))
                .expect("create scale src dir");
            fs::write(
                workspace_root.join("Cargo.toml"),
                "[workspace]\nmembers = [\"fleets/test/scale\"]\n",
            )
            .expect("write workspace manifest");
            fs::write(
                workspace_root.join("fleets/test/scale/Cargo.toml"),
                r#"[package]
name = "canister_scale"
version = "0.1.0"
edition = "2024"

[package.metadata.canic]
role = "scale_replica"
"#,
            )
            .expect("write scale manifest");
            fs::write(workspace_root.join("fleets/test/scale/src/lib.rs"), "")
                .expect("write scale lib");

            let previous_config = std::env::var_os("CANIC_CONFIG_PATH");
            let previous_root = std::env::var_os("CANIC_CANISTERS_ROOT");
            unsafe {
                std::env::remove_var("CANIC_CONFIG_PATH");
                std::env::remove_var("CANIC_CANISTERS_ROOT");
            }
            let result = canister_manifest_path(workspace_root, "scale_replica")
                .expect("scale manifest path");
            restore_env("CANIC_CONFIG_PATH", previous_config);
            restore_env("CANIC_CANISTERS_ROOT", previous_root);

            assert_eq!(result, workspace_root.join("fleets/test/scale/Cargo.toml"));
        });
    }

    #[test]
    fn canister_manifest_path_prefers_scoped_role_metadata() {
        with_guarded_env(|| {
            let temp = TempWorkspace::new();
            let workspace_root = temp.path();
            let audit_root = workspace_root.join("canisters/audit/root_probe");
            let fleet_root = workspace_root.join("fleets/test/root");

            fs::create_dir_all(audit_root.join("src")).expect("create audit root dir");
            fs::create_dir_all(fleet_root.join("src")).expect("create fleet root dir");
            fs::write(
                workspace_root.join("Cargo.toml"),
                "[workspace]\nmembers = [\"canisters/audit/root_probe\", \"fleets/test/root\"]\n",
            )
            .expect("write workspace manifest");
            fs::write(
                audit_root.join("Cargo.toml"),
                r#"[package]
name = "root_probe"
version = "0.1.0"
edition = "2024"

[package.metadata.canic]
role = "root"
"#,
            )
            .expect("write audit root manifest");
            fs::write(audit_root.join("src/lib.rs"), "").expect("write audit root lib");
            fs::write(
                fleet_root.join("Cargo.toml"),
                r#"[package]
name = "canister_root"
version = "0.1.0"
edition = "2024"

[package.metadata.canic]
role = "root"
"#,
            )
            .expect("write fleet root manifest");
            fs::write(fleet_root.join("src/lib.rs"), "").expect("write fleet root lib");

            let previous_config = std::env::var_os("CANIC_CONFIG_PATH");
            let previous_root = std::env::var_os("CANIC_CANISTERS_ROOT");
            unsafe {
                std::env::remove_var("CANIC_CONFIG_PATH");
                std::env::set_var("CANIC_CANISTERS_ROOT", workspace_root.join("fleets/test"));
            }
            let result =
                canister_manifest_path(workspace_root, "root").expect("scoped root manifest path");
            restore_env("CANIC_CONFIG_PATH", previous_config);
            restore_env("CANIC_CANISTERS_ROOT", previous_root);

            assert_eq!(result, fleet_root.join("Cargo.toml"));
        });
    }

    #[test]
    fn read_release_artifact_accepts_gzip_wasm() {
        let temp = TempWorkspace::new();
        let path = temp.path().join("artifact.wasm.gz");
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder
            .write_all(&[0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00])
            .expect("write wasm bytes");
        fs::write(&path, encoder.finish().expect("finish encoder")).expect("write artifact");

        let artifact = read_release_artifact(&path).expect("read artifact");

        assert!(!artifact.is_empty());
    }

    #[test]
    fn read_release_artifact_rejects_plain_wasm() {
        let temp = TempWorkspace::new();
        let path = temp.path().join("artifact.wasm");
        fs::write(&path, [0x00, 0x61, 0x73, 0x6d]).expect("write plain wasm");

        let err = read_release_artifact(&path).unwrap_err();

        assert!(
            err.to_string().contains("not gzip-compressed"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn read_release_artifact_rejects_non_wasm_payload() {
        let temp = TempWorkspace::new();
        let path = temp.path().join("artifact.bin.gz");
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(b"not wasm").expect("write payload");
        fs::write(&path, encoder.finish().expect("finish encoder")).expect("write artifact");

        let err = read_release_artifact(&path).unwrap_err();

        assert!(
            err.to_string()
                .contains("does not decompress to a wasm module"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn canister_manifest_path_requires_declared_role_metadata() {
        with_guarded_env(|| {
            let temp = TempWorkspace::new();
            let workspace_root = temp.path();
            fs::create_dir_all(workspace_root.join("fleets")).expect("create fleets dir");
            fs::write(
                workspace_root.join("Cargo.toml"),
                "[workspace]\nmembers = []\n",
            )
            .expect("write workspace manifest");

            let previous_config = std::env::var_os("CANIC_CONFIG_PATH");
            let previous_root = std::env::var_os("CANIC_CANISTERS_ROOT");
            unsafe {
                std::env::remove_var("CANIC_CONFIG_PATH");
                std::env::remove_var("CANIC_CANISTERS_ROOT");
            }
            let err = canister_manifest_path(workspace_root, "user_hub")
                .expect_err("missing role metadata must fail");
            restore_env("CANIC_CONFIG_PATH", previous_config);
            restore_env("CANIC_CANISTERS_ROOT", previous_root);

            assert!(
                err.to_string()
                    .contains("[package.metadata.canic] role = \"user_hub\""),
                "unexpected error: {err}"
            );
        });
    }

    #[test]
    fn canisters_root_defaults_to_workspace_fleets_dir() {
        with_guarded_env(|| {
            let temp = TempWorkspace::new();
            let workspace_root = temp.path();
            let previous_config = std::env::var_os("CANIC_CONFIG_PATH");
            let previous_root = std::env::var_os("CANIC_CANISTERS_ROOT");
            unsafe {
                std::env::remove_var("CANIC_CONFIG_PATH");
                std::env::remove_var("CANIC_CANISTERS_ROOT");
            }
            let result = canisters_root(workspace_root);
            restore_env("CANIC_CONFIG_PATH", previous_config);
            restore_env("CANIC_CANISTERS_ROOT", previous_root);

            assert_eq!(result, workspace_root.join("fleets"));
        });
    }

    #[test]
    fn config_path_override_is_normalized_against_workspace_root() {
        with_guarded_env(|| {
            let temp = TempWorkspace::new();
            let workspace_root = temp.path();
            let relative = Path::new("configs/canic.toml");
            let previous = std::env::var_os("CANIC_CONFIG_PATH");
            unsafe {
                std::env::set_var("CANIC_CONFIG_PATH", relative);
            }
            let result = config_path(workspace_root);
            unsafe {
                if let Some(value) = previous {
                    std::env::set_var("CANIC_CONFIG_PATH", value);
                } else {
                    std::env::remove_var("CANIC_CONFIG_PATH");
                }
            }

            assert_eq!(result, workspace_root.join(relative));
        });
    }
}
