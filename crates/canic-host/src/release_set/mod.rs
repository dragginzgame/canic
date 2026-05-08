//! Release-set discovery, manifest emission, and staging helpers.

use std::time::{SystemTime, UNIX_EPOCH};

mod config;
mod manifest;
mod paths;
mod stage;

pub use config::{
    configured_fleet_name, configured_fleet_roles, configured_install_targets,
    configured_release_roles, configured_role_auto_create, configured_role_capabilities,
    configured_role_details, configured_role_kinds, configured_role_topups,
};
pub use manifest::{
    ReleaseSetEntry, RootReleaseSetManifest, emit_root_release_set_manifest,
    emit_root_release_set_manifest_if_ready, emit_root_release_set_manifest_with_config,
    load_root_release_set_manifest,
};
pub use paths::{
    canister_manifest_path, canisters_root, config_path, icp_root, load_root_package_version,
    load_workspace_package_version, resolve_artifact_root, root_manifest_path,
    root_release_set_manifest_path, workspace_manifest_path, workspace_root,
};
use stage::build_release_set_entry;
pub(crate) use stage::icp_call_on_network;
pub use stage::{resume_root_bootstrap, stage_root_release_set};

#[cfg(test)]
use stage::read_release_artifact;

#[cfg(test)]
use config::{
    configured_fleet_name_from_source, configured_fleet_roles_from_source,
    configured_release_roles_from_source, configured_role_auto_create_from_source,
    configured_role_capabilities_from_source, configured_role_details_from_source,
    configured_role_kinds_from_source, configured_role_topups_from_source,
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
        canister_manifest_path, canisters_root, config_path, configured_fleet_name_from_source,
        configured_fleet_roles_from_source, configured_install_targets,
        configured_release_roles_from_source, configured_role_auto_create_from_source,
        configured_role_capabilities_from_source, configured_role_details_from_source,
        configured_role_kinds_from_source, configured_role_topups_from_source,
        read_release_artifact, root_manifest_path,
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

[app]
init_mode = "enabled"
[app.whitelist]

[auth.delegated_tokens]
enabled = true
ecdsa_key_name = "test_key_1"

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
    fn configured_fleet_roles_include_root_first() {
        let roles = configured_fleet_roles_from_source(REAL_CONFIG).expect("fleet roles");

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
    fn configured_role_capabilities_lists_enabled_role_features() {
        let config = r#"
controllers = []
app_index = ["user_hub", "scale_hub"]

[fleet]
name = "demo"

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
canister_role = "scale"

[subnets.prime.canisters.scale]
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
    fn configured_role_details_lists_verbose_config_features() {
        let config = r#"
controllers = []
app_index = ["user_hub", "scale_hub"]

[fleet]
name = "demo"

[app]
init_mode = "enabled"
[app.whitelist]

[subnets.prime]
auto_create = ["user_hub"]
subnet_index = ["scale_hub"]

[subnets.prime.canisters.root]
kind = "root"

[subnets.prime.canisters.user_hub]
kind = "singleton"
topup_policy.threshold = "10T"
topup_policy.amount = "4T"

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
canister_role = "scale"
policy.initial_workers = 2
policy.min_workers = 2

[subnets.prime.canisters.scale]
kind = "replica"
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
            details.contains(&"scaling scales->scale initial=2 min=2 max=32".to_string())
        }));
    }

    #[test]
    fn configured_role_topups_lists_configured_policy_summaries() {
        let config = r#"
controllers = []
app_index = []

[fleet]
name = "demo"

[app]
init_mode = "enabled"
[app.whitelist]

[subnets.prime.canisters.root]
kind = "root"

[subnets.prime.canisters.scale_hub]
kind = "singleton"
topup_policy.threshold = "10T"
topup_policy.amount = "4T"
"#;
        let topups = configured_role_topups_from_source(config).expect("role topups");

        assert_eq!(
            topups.get("scale_hub").map(String::as_str),
            Some("4.0TC @ 10.0TC")
        );
        assert!(!topups.contains_key("root"));
    }

    #[test]
    fn configured_role_auto_create_lists_subnet_auto_create_roles() {
        let config = r#"
controllers = []
app_index = []

[fleet]
name = "demo"

[app]
init_mode = "enabled"
[app.whitelist]

[subnets.prime]
auto_create = ["app", "user_hub"]

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
            err.to_string().contains("root canister not defined"),
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
            "[package]\nname = \"canister_root\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
        )
        .expect("write root manifest");
        fs::write(workspace_root.join("fleets/test/root/src/lib.rs"), "").expect("write root lib");

        assert_eq!(
            root_manifest_path(workspace_root),
            workspace_root.join("fleets/test/root/Cargo.toml")
        );
    }

    #[test]
    fn canister_manifest_path_prefers_canister_manifest_metadata() {
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
            "[package]\nname = \"canister_user_hub\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
        )
        .expect("write user hub manifest");
        fs::write(workspace_root.join("fleets/test/user_hub/src/lib.rs"), "")
            .expect("write user hub lib");

        assert_eq!(
            canister_manifest_path(workspace_root, "user_hub"),
            workspace_root.join("fleets/test/user_hub/Cargo.toml")
        );
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
    fn canister_manifest_path_falls_back_to_fleets_root() {
        let temp = TempWorkspace::new();
        let workspace_root = temp.path();
        fs::create_dir_all(workspace_root.join("fleets")).expect("create fleets dir");

        assert_eq!(
            canister_manifest_path(workspace_root, "user_hub"),
            workspace_root.join("fleets/user_hub/Cargo.toml")
        );
    }

    #[test]
    fn canisters_root_defaults_to_workspace_fleets_dir() {
        let temp = TempWorkspace::new();
        let workspace_root = temp.path();

        assert_eq!(
            canisters_root(workspace_root),
            workspace_root.join("fleets")
        );
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
