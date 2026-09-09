//! Module: canic_host::canister_build::cache
//!
//! Responsibility: isolate reusable Cargo state created by canister artifact builds.
//! Does not own: canonical `.icp` artifacts, build profiles, or deployment orchestration.
//! Boundary: resolves one dedicated Wasm target directory while respecting explicit Cargo input.

use std::{
    env,
    ffi::OsStr,
    fs, io,
    path::{Path, PathBuf},
    process::Command,
};

use crate::durable_io::{RegularFileLockError, lock_regular_file_with_parents};

const DEFAULT_WASM_TARGET_RELATIVE: &str = "target/canic-wasm";
const CANISTER_BUILD_LOCK_RELATIVE: &str = ".canic/locks/canister-artifact-build.lock";

pub fn configure_canister_cargo_command(command: &mut Command, workspace_root: &Path) {
    command.env("CARGO_INCREMENTAL", "0").env(
        "CARGO_TARGET_DIR",
        canister_build_target_root(workspace_root),
    );
    configure_implicit_sccache(
        command,
        env::var_os("RUSTC_WRAPPER").as_deref(),
        env::var_os("PATH").as_deref(),
    );
}

/// Declaration passes retain runtime cfg/profile semantics without paying for LTO.
pub fn configure_declaration_command(
    command: &mut Command,
    context: &crate::canister_build::WorkspaceBuildContext,
) {
    let profile = match context.profile {
        crate::canister_build::CanisterBuildProfile::Debug => "DEV",
        crate::canister_build::CanisterBuildProfile::Fast => "FAST",
        crate::canister_build::CanisterBuildProfile::Release => "RELEASE",
    };
    command
        .env(canic_core::role_contract::CANONICAL_CANDID_BUILD_ENV, "1")
        .env_remove(canic_core::ids::RELEASE_BUILD_ID_ENV)
        .env_remove(canic_core::role_contract::PROTOCOL_PROFILE_DIGEST_ENV)
        .env_remove(canic_core::role_contract::build_context::PROTOCOL_BUILD_CONTEXT_ENV)
        .env(format!("CARGO_PROFILE_{profile}_LTO"), "off")
        .env(format!("CARGO_PROFILE_{profile}_OPT_LEVEL"), "0")
        .env(format!("CARGO_PROFILE_{profile}_CODEGEN_UNITS"), "16")
        .env(
            "CARGO_TARGET_DIR",
            declaration_target_root(&context.workspace_root),
        );
}

pub fn declaration_target_root(workspace_root: &Path) -> PathBuf {
    canister_build_target_root(workspace_root).join("declarations")
}

#[must_use]
pub fn canister_build_target_root(workspace_root: &Path) -> PathBuf {
    resolve_canister_build_target_root(
        workspace_root,
        env::var_os("CARGO_TARGET_DIR").map(PathBuf::from),
    )
}

/// Lock the complete shared Cargo-target build and artifact-materialization boundary.
pub fn lock_canister_build_target(workspace_root: &Path) -> io::Result<fs::File> {
    let path = workspace_root.join(CANISTER_BUILD_LOCK_RELATIVE);
    lock_regular_file_with_parents(&path).map_err(|error| match error {
        RegularFileLockError::NotRegular => io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "Canic artifact-build lock is not a regular file: {}",
                path.display()
            ),
        ),
        RegularFileLockError::Io(source) => io::Error::new(
            source.kind(),
            format!(
                "failed to lock Canic artifact-build target {}: {source}",
                path.display()
            ),
        ),
        #[cfg(windows)]
        RegularFileLockError::UnsupportedPlatform => io::Error::new(
            io::ErrorKind::Unsupported,
            "Canic artifact-build locking is unsupported on Windows",
        ),
    })
}

fn resolve_canister_build_target_root(
    workspace_root: &Path,
    configured_target: Option<PathBuf>,
) -> PathBuf {
    configured_target.map_or_else(
        || workspace_root.join(DEFAULT_WASM_TARGET_RELATIVE),
        |path| {
            if path.is_absolute() {
                path
            } else {
                workspace_root.join(path)
            }
        },
    )
}

fn resolve_implicit_sccache_wrapper(
    explicit_wrapper: Option<&OsStr>,
    search_path: Option<&OsStr>,
) -> Option<PathBuf> {
    if explicit_wrapper.is_some() {
        return None;
    }
    search_path.and_then(|search_path| {
        env::split_paths(search_path)
            .map(|directory| directory.join(sccache_executable_name()))
            .find(|candidate| is_executable_file(candidate))
    })
}

fn configure_implicit_sccache(
    command: &mut Command,
    explicit_wrapper: Option<&OsStr>,
    search_path: Option<&OsStr>,
) {
    if let Some(sccache) = resolve_implicit_sccache_wrapper(explicit_wrapper, search_path) {
        command.env("RUSTC_WRAPPER", sccache);
    }
}

#[cfg(windows)]
const fn sccache_executable_name() -> &'static str {
    "sccache.exe"
}

#[cfg(not(windows))]
const fn sccache_executable_name() -> &'static str {
    "sccache"
}

fn is_executable_file(path: &Path) -> bool {
    let Ok(metadata) = fs::metadata(path) else {
        return false;
    };
    if !metadata.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;

        metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::temp_dir;
    use std::{ffi::OsString, sync::mpsc, thread, time::Duration};

    #[test]
    fn release_declarations_disable_lto_without_changing_runtime_profile() {
        let context = crate::canister_build::WorkspaceBuildContext {
            role: "app".into(),
            profile: crate::canister_build::CanisterBuildProfile::Release,
            environment: "local".into(),
            build_network: canic_core::ids::BuildNetwork::Local,
            workspace_root: "/workspace".into(),
            icp_root: "/workspace".into(),
            config_path: "/workspace/canic.toml".into(),
            local_replica: None,
            refresh_canonical_infrastructure_did: false,
            release_build_id: None,
        };
        let mut command = Command::new("cargo");
        configure_declaration_command(&mut command, &context);
        let environment = command
            .get_envs()
            .collect::<std::collections::BTreeMap<_, _>>();
        assert_eq!(
            environment[OsStr::new("CARGO_PROFILE_RELEASE_LTO")],
            Some(OsStr::new("off"))
        );
        assert_eq!(
            environment[OsStr::new("CARGO_PROFILE_RELEASE_CODEGEN_UNITS")],
            Some(OsStr::new("16"))
        );
        assert_eq!(
            environment[OsStr::new("CARGO_TARGET_DIR")],
            Some(declaration_target_root(&context.workspace_root).as_os_str())
        );
        assert_eq!(
            environment[OsStr::new("CARGO_PROFILE_RELEASE_OPT_LEVEL")],
            Some(OsStr::new("0"))
        );
        assert_eq!(
            context.profile,
            crate::canister_build::CanisterBuildProfile::Release
        );
    }

    #[test]
    fn declaration_and_runtime_preserve_cfg_with_distinct_final_outputs() {
        let root = temp_dir("declaration-profile-cfg");
        fs::create_dir_all(root.join("helper/src")).unwrap();
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("Cargo.toml"), "[workspace]\n[package]\nname=\"cache_probe\"\nversion=\"0.1.0\"\nedition=\"2024\"\n[build-dependencies]\ncache_helper={path=\"helper\"}\n[profile.release]\nlto=true\ncodegen-units=1\n").unwrap();
        fs::write(
            root.join("helper/Cargo.toml"),
            "[package]\nname=\"cache_helper\"\nversion=\"0.1.0\"\nedition=\"2024\"\n",
        )
        .unwrap();
        fs::write(root.join("helper/src/lib.rs"), "pub fn mode() -> &'static str { if std::env::var(\"CANIC_INTERNAL_CANDID_BUILD\").as_deref() == Ok(\"1\") { \"declaration\" } else { \"runtime\" } }\n").unwrap();
        fs::write(root.join("build.rs"), "fn main() { println!(\"cargo:rerun-if-env-changed=CANIC_INTERNAL_CANDID_BUILD\"); println!(\"cargo:rustc-env=MODE={}\", cache_helper::mode()); }\n").unwrap();
        fs::write(
            root.join("src/main.rs"),
            "fn main() { println!(\"{} {}\", env!(\"MODE\"), cfg!(debug_assertions)); }\n",
        )
        .unwrap();
        fs::write(root.join("Cargo.lock"), "version=4\n[[package]]\nname=\"cache_helper\"\nversion=\"0.1.0\"\n[[package]]\nname=\"cache_probe\"\nversion=\"0.1.0\"\ndependencies=[\"cache_helper\"]\n").unwrap();
        let context = crate::canister_build::WorkspaceBuildContext {
            role: "app".into(),
            profile: crate::canister_build::CanisterBuildProfile::Release,
            environment: "local".into(),
            build_network: canic_core::ids::BuildNetwork::Local,
            workspace_root: root.clone(),
            icp_root: root.clone(),
            config_path: root.join("Cargo.toml"),
            local_replica: None,
            refresh_canonical_infrastructure_did: false,
            release_build_id: None,
        };
        let declaration = run_intermediate_probe(&context, true);
        let runtime = run_intermediate_probe(&context, false);
        assert_ne!(declaration, runtime);
        assert_eq!(
            Command::new(declaration).output().unwrap().stdout,
            b"declaration false\n"
        );
        assert_eq!(
            Command::new(runtime).output().unwrap().stdout,
            b"runtime false\n"
        );
        fs::remove_dir_all(root).unwrap();
    }

    fn run_intermediate_probe(
        context: &crate::canister_build::WorkspaceBuildContext,
        declaration: bool,
    ) -> PathBuf {
        let mut command = crate::cargo_command();
        command.current_dir(&context.workspace_root).args([
            "build",
            "--locked",
            "--offline",
            "--release",
            "--message-format=json",
        ]);
        context.apply_to_command(&mut command);
        configure_canister_cargo_command(&mut command, &context.workspace_root);
        if declaration {
            configure_declaration_command(&mut command, context);
        }
        let host = Command::new("rustc")
            .args(["--print", "host-tuple"])
            .output()
            .unwrap();
        assert!(host.status.success());
        let host = String::from_utf8(host.stdout).unwrap();
        command.arg("--target").arg(host.trim());
        // Keep nested Cargo out of any target selected for the enclosing test runner.
        command.env_remove("CARGO_BUILD_BUILD_DIR");
        command.env(
            "CARGO_TARGET_DIR",
            context.workspace_root.join(if declaration {
                "declarations"
            } else {
                "runtime"
            }),
        );
        let output = command.output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let records = String::from_utf8(output.stdout)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
            .collect::<Vec<_>>();
        let binary = records
            .iter()
            .find(|value| {
                value["reason"] == "compiler-artifact" && value["target"]["name"] == "cache_probe"
            })
            .unwrap();
        PathBuf::from(binary["executable"].as_str().unwrap())
    }

    #[test]
    fn default_target_is_a_dedicated_reusable_workspace_cache() {
        let root = Path::new("/workspace");

        assert_eq!(
            resolve_canister_build_target_root(root, None),
            Path::new("/workspace/target/canic-wasm")
        );
    }

    #[test]
    fn configured_relative_target_remains_workspace_relative() {
        let root = Path::new("/workspace");

        assert_eq!(
            resolve_canister_build_target_root(root, Some(PathBuf::from("custom-target"))),
            Path::new("/workspace/custom-target")
        );
    }

    #[test]
    fn install_build_discovers_sccache_without_overriding_explicit_wrapper() {
        let root = temp_dir("canister-build-sccache");
        let bin = root.join("bin");
        fs::create_dir_all(&bin).expect("create cache bin directory");
        let sccache = bin.join(sccache_executable_name());
        fs::write(&sccache, b"cache").expect("write cache executable");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;

            fs::set_permissions(&sccache, fs::Permissions::from_mode(0o755))
                .expect("make cache executable");
        }
        let search_path = env::join_paths([&bin]).expect("join cache search path");

        let mut discovered = Command::new("cargo");
        configure_implicit_sccache(&mut discovered, None, Some(&search_path));
        assert_eq!(
            discovered
                .get_envs()
                .find(|(name, _)| *name == "RUSTC_WRAPPER")
                .and_then(|(_, value)| value),
            Some(sccache.as_os_str())
        );

        let explicit_wrapper = OsString::from("custom-wrapper");
        let mut explicit = Command::new("cargo");
        explicit.env("RUSTC_WRAPPER", &explicit_wrapper);
        configure_implicit_sccache(
            &mut explicit,
            Some(explicit_wrapper.as_os_str()),
            Some(&search_path),
        );
        assert_eq!(
            explicit
                .get_envs()
                .find(|(name, _)| *name == "RUSTC_WRAPPER")
                .and_then(|(_, value)| value),
            Some(explicit_wrapper.as_os_str())
        );

        fs::remove_dir_all(root).expect("remove cache test root");
    }

    #[test]
    fn artifact_materialization_lock_excludes_a_second_builder() {
        let root = temp_dir("canic-artifact-build-lock");
        let _ = fs::remove_dir_all(&root);
        let lock_path = root.join(CANISTER_BUILD_LOCK_RELATIVE);
        assert!(lock_path.starts_with(root.join(".canic")));
        assert!(!lock_path.starts_with(root.join("target")));
        let first = lock_canister_build_target(&root).expect("acquire first build lock");
        let contender_root = root.clone();
        let (attempted_tx, attempted_rx) = mpsc::channel();
        let (acquired_tx, acquired_rx) = mpsc::channel();
        let contender = thread::spawn(move || {
            attempted_tx.send(()).expect("report lock attempt");
            let acquired = lock_canister_build_target(&contender_root).is_ok();
            acquired_tx.send(acquired).expect("report lock result");
        });

        attempted_rx.recv().expect("observe contender attempt");
        assert!(
            acquired_rx
                .recv_timeout(Duration::from_millis(100))
                .is_err()
        );
        drop(first);
        assert_eq!(acquired_rx.recv_timeout(Duration::from_secs(2)), Ok(true));
        contender.join().expect("join lock contender");
        fs::remove_dir_all(root).expect("clean build-lock fixture");
    }
}
