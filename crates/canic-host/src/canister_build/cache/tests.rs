//! Focused checks for canister artifact cache behavior.

use super::*;
use crate::test_support::temp_dir;
use std::{sync::mpsc, thread, time::Duration};

#[test]
fn heartbeat_distinguishes_sequential_batches_and_bounds_role_labels() {
    let phase_started = Instant::now();
    let first = CargoBuildProgress::batch(["hub"], 1, 3, phase_started);
    let second = CargoBuildProgress::batch(["shard"], 2, 3, phase_started);
    assert_eq!(first.phase_started, second.phase_started);
    let first_message = first.message(
        "declaration",
        Duration::from_secs(30),
        Duration::from_secs(30),
    );
    let second_message = second.message(
        "declaration",
        Duration::from_secs(30),
        Duration::from_secs(90),
    );
    assert!(first_message.contains("batch 1/3 roles [hub]; child 30s, phase 30s"));
    assert!(second_message.contains("batch 2/3 roles [shard]; child 30s, phase 90s"));
    let unsafe_role = format!("role\n{}", "x".repeat(1000));
    let bounded = CargoBuildProgress::batch(
        std::iter::repeat_n(unsafe_role.as_str(), 100),
        1,
        100,
        phase_started,
    );
    let message = bounded.message("runtime", Duration::ZERO, Duration::ZERO);
    assert!(!message.contains('\n'));
    assert!(message.len() < 1000);
    assert!(message.contains("..."));
}

#[test]
fn release_declarations_disable_lto_without_changing_runtime_profile() {
    let context = crate::canister_build::WorkspaceBuildContext {
        role: "app".into(),
        profile: crate::canister_build::CanisterBuildProfile::Release,
        environment: "local".into(),
        build_network: canic_contracts::ids::BuildNetwork::Local,
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
    crate::test_support::generate_fixture_lockfile(&root);
    let context = crate::canister_build::WorkspaceBuildContext {
        role: "app".into(),
        profile: crate::canister_build::CanisterBuildProfile::Release,
        environment: "local".into(),
        build_network: canic_contracts::ids::BuildNetwork::Local,
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
#[cfg(unix)]
fn cargo_keeps_explicit_wrapper_and_returns_its_failure_without_retry() {
    for selected in ["", "configured-wrapper"] {
        let root = temp_dir("explicit-compiler-wrapper");
        fs::create_dir_all(&root).unwrap();
        let mut cargo = Command::new("/bin/sh");
        cargo.current_dir(&root).args([
            "-c",
            "printf 'run\\n' >> calls; printf '%s' \"$RUSTC_WRAPPER\"; printf 'compiler failure' >&2; exit 7",
        ]);
        cargo.env("RUSTC_WRAPPER", selected);
        configure_canister_cargo_command(&mut cargo, &root);
        let output =
            output_canister_cargo_command(&mut cargo, CargoBuildProgress::single("fixture"))
                .unwrap();
        assert_eq!(output.status.code(), Some(7));
        assert_eq!(output.stdout, selected.as_bytes());
        assert_eq!(output.stderr, b"compiler failure");
        assert_eq!(fs::read(root.join("calls")).unwrap(), b"run\n");
        fs::remove_dir_all(root).unwrap();
    }
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
