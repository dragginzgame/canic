use canic_core::bootstrap::parse_config_model;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    fs,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

mod paths;
mod stage;

pub use paths::{
    canister_manifest_path, canisters_root, config_path, dfx_root, load_root_package_version,
    load_workspace_package_version, resolve_artifact_root, root_manifest_path,
    root_release_set_manifest_path, workspace_manifest_path, workspace_root,
};
use stage::build_release_set_entry;
pub use stage::{
    dfx_call, idl_blob, idl_text, json_u64, resume_root_bootstrap, stage_root_release_set,
    wasm_hash, wasm_hash_hex,
};

#[cfg(test)]
use stage::read_release_artifact;

pub(super) const CANISTERS_ROOT_RELATIVE: &str = "canisters";
pub(super) const ROOT_CONFIG_FILE: &str = "canic.toml";
pub(super) const WORKSPACE_MANIFEST_RELATIVE: &str = "Cargo.toml";
pub const ROOT_RELEASE_SET_MANIFEST_FILE: &str = "root.release-set.json";
pub(super) const GZIP_MAGIC: [u8; 2] = [0x1f, 0x8b];
pub(super) const WASM_MAGIC: [u8; 4] = [0x00, 0x61, 0x73, 0x6d];

///
/// RootReleaseSetManifest
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootReleaseSetManifest {
    pub release_version: String,
    pub entries: Vec<ReleaseSetEntry>,
}

///
/// ReleaseSetEntry
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ReleaseSetEntry {
    pub role: String,
    pub template_id: String,
    pub artifact_relative_path: String,
    pub payload_size_bytes: u64,
    pub payload_sha256_hex: String,
    pub chunk_size_bytes: u64,
    pub chunk_sha256_hex: Vec<String>,
}

// Build and persist the current root release-set manifest from built `.wasm.gz` artifacts.
pub fn emit_root_release_set_manifest(
    workspace_root: &Path,
    dfx_root: &Path,
    network: &str,
) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    let artifact_root = resolve_artifact_root(dfx_root, network)?;
    let config_path = config_path(workspace_root);
    let manifest_path = root_release_set_manifest_path(&artifact_root)?;
    let release_version = load_root_package_version(
        &root_manifest_path(workspace_root),
        &workspace_manifest_path(workspace_root),
    )?;
    let entries = configured_release_roles(&config_path)?
        .into_iter()
        .map(|role_name| build_release_set_entry(dfx_root, &artifact_root, &role_name))
        .collect::<Result<Vec<_>, _>>()?;
    let manifest = RootReleaseSetManifest {
        release_version,
        entries,
    };

    fs::write(&manifest_path, serde_json::to_vec_pretty(&manifest)?)?;
    Ok(manifest_path)
}

// Emit the root release-set manifest only once every required ordinary artifact exists.
pub fn emit_root_release_set_manifest_if_ready(
    workspace_root: &Path,
    dfx_root: &Path,
    network: &str,
) -> Result<Option<std::path::PathBuf>, Box<dyn std::error::Error>> {
    let artifact_root = resolve_artifact_root(dfx_root, network)?;
    let roles = configured_release_roles(&config_path(workspace_root))?;

    for role_name in roles {
        let artifact_path = artifact_root
            .join(&role_name)
            .join(format!("{role_name}.wasm.gz"));
        if !artifact_path.is_file() {
            return Ok(None);
        }
    }

    emit_root_release_set_manifest(workspace_root, dfx_root, network).map(Some)
}

// Load one previously emitted root release-set manifest from disk.
pub fn load_root_release_set_manifest(
    manifest_path: &Path,
) -> Result<RootReleaseSetManifest, Box<dyn std::error::Error>> {
    let source = fs::read(manifest_path)?;
    Ok(serde_json::from_slice(&source)?)
}

// Enumerate the configured ordinary roles that root must publish before bootstrap resumes.
pub fn configured_release_roles(
    config_path: &Path,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let config_source = fs::read_to_string(config_path)?;
    configured_release_roles_from_source(&config_source)
        .map_err(|err| format!("invalid {}: {err}", config_path.display()).into())
}

// Enumerate the local install targets: root plus the ordinary roles owned by its subnet.
pub fn configured_install_targets(
    config_path: &Path,
    root_canister: &str,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut targets = vec![root_canister.to_string()];
    targets.extend(configured_release_roles(config_path)?);
    Ok(targets)
}

// Enumerate the configured ordinary roles for the single subnet that owns `root`.
fn configured_release_roles_from_source(
    config_source: &str,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let config = parse_config_model(config_source).map_err(|err| err.to_string())?;
    let mut roles = BTreeSet::new();
    let mut root_subnet_roles = None;

    for (subnet_role, subnet) in &config.subnets {
        if !subnet
            .canisters
            .keys()
            .any(canic::ids::CanisterRole::is_root)
        {
            continue;
        }

        if root_subnet_roles.is_some() {
            return Err(format!(
                "multiple subnets define a root canister; release-set staging requires exactly one root subnet (found at least '{subnet_role}')"
            )
            .into());
        }

        root_subnet_roles = Some(
            subnet
                .canisters
                .keys()
                .filter(|role| !role.is_root() && !role.is_wasm_store())
                .map(|role| role.as_str().to_string())
                .collect::<Vec<_>>(),
        );
    }

    let root_subnet_roles = root_subnet_roles.ok_or_else(|| {
        "no subnet defines a root canister; release-set staging requires exactly one root subnet"
            .to_string()
    })?;

    for role in root_subnet_roles {
        roles.insert(role);
    }

    Ok(roles.into_iter().collect())
}

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
        canister_manifest_path, canisters_root, config_path, configured_install_targets,
        configured_release_roles_from_source, read_release_artifact, root_manifest_path,
    };
    use flate2::{Compression, write::GzEncoder};
    use std::{
        env, fs,
        io::Write,
        path::{Path, PathBuf},
        sync::{Mutex, OnceLock},
        time::{SystemTime, UNIX_EPOCH},
    };

    static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    const REAL_CONFIG: &str = r#"
controllers = []
app_directory = ["user_hub", "scale_hub"]

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
app_directory = []

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
app_directory = []

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
            let unique = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |duration| duration.as_nanos());
            let path = env::temp_dir().join(format!(
                "canic-installer-release-set-tests-{}-{unique}",
                std::process::id()
            ));
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
app_directory = []

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
    fn config_path_defaults_under_canisters_root() {
        with_guarded_env(|| {
            let temp = TempWorkspace::new();
            let workspace_root = temp.path();
            let canisters_dir = workspace_root.join("canisters");
            fs::create_dir_all(&canisters_dir).expect("create canisters dir");
            let expected = canisters_dir.join("canic.toml");

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
        fs::create_dir_all(workspace_root.join("canisters/root")).expect("create root dir");
        fs::write(
            workspace_root.join("Cargo.toml"),
            "[workspace]\nmembers = []\n",
        )
        .expect("write workspace manifest");
        fs::write(
            workspace_root.join("canisters/root/Cargo.toml"),
            "[package]\nname = \"canister_root\"\nversion = \"0.1.0\"\n",
        )
        .expect("write root manifest");

        assert_eq!(
            root_manifest_path(workspace_root),
            workspace_root.join("canisters/root/Cargo.toml")
        );
    }

    #[test]
    fn canister_manifest_path_prefers_canister_manifest_metadata() {
        let temp = TempWorkspace::new();
        let workspace_root = temp.path();
        fs::create_dir_all(workspace_root.join("canisters/user_hub")).expect("create user hub dir");
        fs::write(
            workspace_root.join("Cargo.toml"),
            "[workspace]\nmembers = []\n",
        )
        .expect("write workspace manifest");
        fs::write(
            workspace_root.join("canisters/user_hub/Cargo.toml"),
            "[package]\nname = \"canister_user_hub\"\nversion = \"0.1.0\"\n",
        )
        .expect("write user hub manifest");

        assert_eq!(
            canister_manifest_path(workspace_root, "user_hub"),
            workspace_root.join("canisters/user_hub/Cargo.toml")
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
    fn canister_manifest_path_falls_back_to_canisters_root() {
        let temp = TempWorkspace::new();
        let workspace_root = temp.path();
        fs::create_dir_all(workspace_root.join("canisters")).expect("create canisters dir");

        assert_eq!(
            canister_manifest_path(workspace_root, "user_hub"),
            workspace_root.join("canisters/user_hub/Cargo.toml")
        );
    }

    #[test]
    fn canisters_root_defaults_to_workspace_canisters_dir() {
        let temp = TempWorkspace::new();
        let workspace_root = temp.path();

        assert_eq!(
            canisters_root(workspace_root),
            workspace_root.join("canisters")
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
