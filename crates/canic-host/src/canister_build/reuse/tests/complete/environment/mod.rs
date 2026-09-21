//! Qualify build-environment normalization with real Cargo inputs and synthetic release manifests.

use super::*;
use crate::icp::CANIC_ICP_IDENTITY_PASSWORD_FILE_ENV;

const CHILD_ROOT: &str = "CANIC_TEST_BUILD_ENVIRONMENT_ROOT";
const BUILD_INPUT: &str = "CANIC_TEST_BUILD_INPUT";

#[test]
fn launcher_shell_depth_and_credentials_preserve_reuse_while_build_inputs_invalidate() {
    if let Some(root) = env::var_os(CHILD_ROOT) {
        observe_build_environment(Path::new(&root));
        return;
    }
    let (root, context) = infrastructure_build_fixture();
    write_environment_probe(&context);
    let thread = std::thread::current();
    let invoke = |credential: Option<&str>,
                  input: &str,
                  shell_depth: Option<&str>,
                  session: Option<(&str, &str)>| {
        invoke_environment(
            &root,
            thread.name().unwrap(),
            credential,
            input,
            shell_depth,
            session,
        )
    };
    let baseline = invoke(Some("fixture-credential-a"), "alpha", Some("1"), None);
    assert_eq!(baseline["reused"], false);
    for credential in [
        Some("fixture-credential-b"),
        None,
        Some("fixture-credential-a"),
    ] {
        let repeat = invoke(credential, "alpha", Some("2"), None);
        assert_eq!(repeat["reused"], true);
        for key in ["inputs", "release", "wasm"] {
            assert_eq!(repeat[key], baseline[key], "unchanged {key}");
        }
    }
    for depth in [None, Some("0"), Some("7"), Some("invalid")] {
        let repeat = invoke(None, "alpha", depth, None);
        assert_eq!(repeat["reused"], true);
        for key in ["inputs", "release", "wasm"] {
            assert_eq!(repeat[key], baseline[key], "unchanged {key}");
        }
    }
    for session in [
        Some(("session-a", "thread-a")),
        Some(("session-b", "thread-b")),
    ] {
        let repeat = invoke(None, "alpha", Some("1"), session);
        assert_eq!(repeat["reused"], true);
        for key in ["inputs", "release", "wasm"] {
            assert_eq!(repeat[key], baseline[key], "unchanged {key}");
        }
    }
    let changed = invoke(Some("fixture-credential-a"), "beta", Some("1"), None);
    assert_eq!(changed["reused"], false);
    assert_ne!(changed["inputs"], baseline["inputs"]);
    assert_ne!(changed["wasm"], baseline["wasm"]);
    assert!(
        changed["miss_reason"]
            .as_str()
            .unwrap()
            .contains("changed-value keys, up to 8: CANIC_TEST_BUILD_INPUT")
    );
    assert!(
        changed["miss_reason"]
            .as_str()
            .unwrap()
            .contains("CODEX_BUILD_FIXTURE_INPUT")
    );
    let repeat = invoke(Some("fixture-credential-b"), "beta", None, None);
    assert_eq!(repeat["reused"], true);
    assert_eq!(repeat["inputs"], changed["inputs"]);
    assert_eq!(repeat["release"], changed["release"]);
    let dependency = root.join("upstream/canic/src/lib.rs");
    let source = fs::read_to_string(&dependency).unwrap();
    fs::write(&dependency, source.replace("{ 1 }", "{ 2 }")).unwrap();
    let changed_source = invoke(Some("fixture-credential-a"), "beta", Some("1"), None);
    assert_eq!(changed_source["reused"], false);
    assert_ne!(changed_source["inputs"], changed["inputs"]);
    assert_ne!(changed_source["wasm"], changed["wasm"]);
    let config = fs::read_to_string(&context.config_path).unwrap();
    fs::write(
        &context.config_path,
        config.replace("maximum_instances = 1", "maximum_instances = 2"),
    )
    .unwrap();
    let changed_config = invoke(Some("fixture-credential-a"), "beta", Some("1"), None);
    assert_eq!(changed_config["reused"], false);
    assert_ne!(changed_config["inputs"], changed_source["inputs"]);
    fs::remove_dir_all(root).unwrap();
}

fn invoke_environment(
    root: &Path,
    test_name: &str,
    credential: Option<&str>,
    input: &str,
    shell_depth: Option<&str>,
    session: Option<(&str, &str)>,
) -> serde_json::Value {
    let mut command = Command::new(env::current_exe().unwrap());
    command
        .args(["--exact", test_name, "--include-ignored"])
        .env(CHILD_ROOT, root)
        .env(BUILD_INPUT, input)
        .env("CODEX_BUILD_FIXTURE_INPUT", input)
        .env("CARGO_TARGET_DIR", root.join("target"))
        .env_remove("CARGO_BUILD_BUILD_DIR")
        .env_remove(CANIC_ICP_IDENTITY_PASSWORD_FILE_ENV)
        .env_remove("SHLVL")
        .env_remove("CODEX_SESSION_ID")
        .env_remove("CODEX_THREAD_ID");
    if let Some((session, thread)) = session {
        command
            .env("CODEX_SESSION_ID", session)
            .env("CODEX_THREAD_ID", thread);
    }
    if let Some(depth) = shell_depth {
        command.env("SHLVL", depth);
    }
    if let Some(value) = credential {
        command.env(CANIC_ICP_IDENTITY_PASSWORD_FILE_ENV, value);
    }
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice::<serde_json::Value>(
        &fs::read(root.join("build-observation.json")).unwrap(),
    )
    .unwrap()
}

fn write_environment_probe(context: &WorkspaceBuildContext) {
    fs::write(
        context.workspace_root.join("build.rs"),
        r#"
fn main() {
    assert!(std::env::var_os("CANIC_ICP_IDENTITY_PASSWORD_FILE").is_none());
    assert!(std::env::var_os("CODEX_SESSION_ID").is_none());
    assert!(std::env::var_os("CODEX_THREAD_ID").is_none());
    assert_eq!(std::env::var("SHLVL").as_deref(), Ok("0"));
    let input = std::env::var("CANIC_TEST_BUILD_INPUT").unwrap();
    assert_eq!(std::env::var("CODEX_BUILD_FIXTURE_INPUT").unwrap(), input);
    println!("cargo:rustc-env=CANIC_TEST_COMPILED_INPUT={input}");
    println!("cargo:rerun-if-env-changed=CANIC_TEST_BUILD_INPUT");
    println!("cargo:rerun-if-env-changed=CODEX_BUILD_FIXTURE_INPUT");
    println!("cargo:rerun-if-env-changed=CANIC_ICP_IDENTITY_PASSWORD_FILE");
    println!("cargo:rerun-if-changed=build.rs");
}
"#,
    )
    .unwrap();
    fs::write(
        context.workspace_root.join("src/lib.rs"),
        r#"
const _: () = assert!(option_env!("CANIC_ICP_IDENTITY_PASSWORD_FILE").is_none());
const _: () = assert!(option_env!("CODEX_SESSION_ID").is_none());
const _: () = assert!(option_env!("CODEX_THREAD_ID").is_none());
const _: () = assert!(option_env!("CODEX_BUILD_FIXTURE_INPUT").is_some());
const _: () = assert!(env!("SHLVL").as_bytes()[0] == b'0');
#[unsafe(no_mangle)]
pub extern "C" fn fixture_value() -> u8 {
    env!("CANIC_TEST_COMPILED_INPUT").as_bytes()[0] + canic::value()
}
"#,
    )
    .unwrap();
}

fn observe_build_environment(root: &Path) {
    let context = infrastructure_build_context(root.join("app"));
    let started = std::time::Instant::now();
    let before = prepared_reuse(&context)
        .load()
        .unwrap()
        .map(|hit| hit.release_build_id);
    // Cargo/build.rs and rustc assert credentials/session IDs are absent. This real
    // compiler output is measured separately from the synthetic sealed manifest.
    let lookup_micros = started.elapsed().as_micros();
    let compile_started = std::time::Instant::now();
    compile_infrastructure_fixture(&context);
    let cargo_micros = compile_started.elapsed().as_micros();
    let sealing_started = std::time::Instant::now();
    let reuse = prepared_reuse(&context);
    let hit = reuse.load().unwrap();
    assert_eq!(before, hit.as_ref().map(|hit| hit.release_build_id));
    let reused = hit.is_some();
    let miss_reason = (!reused).then(|| reuse.miss_reason());
    let release = hit.map_or_else(
        || {
            let release = finalize_fixture(&context);
            reuse
                .record(
                    release,
                    vec![
                        "app".into(),
                        "root".into(),
                        "fleet_coordinator".into(),
                        "wasm_store".into(),
                    ],
                )
                .unwrap();
            release
        },
        |hit| hit.release_build_id,
    );
    let diagnostics = fs::read_to_string(
        context
            .icp_root
            .join(".canic/build-reuse/last-input-diagnostics.json"),
    )
    .unwrap();
    for key in [
        CANIC_ICP_IDENTITY_PASSWORD_FILE_ENV,
        "CODEX_SESSION_ID",
        "CODEX_THREAD_ID",
    ] {
        assert!(!diagnostics.contains(key));
    }
    fs::write(
        root.join("build-observation.json"),
        serde_json::to_vec(&serde_json::json!({
            "lookup_micros": lookup_micros,
            "cargo_probe_micros": cargo_micros,
            "synthetic_sealing_micros": sealing_started.elapsed().as_micros(),
            "elapsed_micros": started.elapsed().as_micros(),
            "inputs": reuse.inputs.digest(),
            "release": release,
            "reused": reused,
            "miss_reason": miss_reason,
            "wasm": file_hash(&root.join("target/wasm32-unknown-unknown/fast/reuse_app.wasm")).unwrap(),
        }))
        .unwrap(),
    )
    .unwrap();
}

#[test]
#[ignore = "opt-in isolated real-Cargo reuse matrix; synthetic release sealing"]
fn frozen_relocated_and_changed_tree_measurement() {
    if let Some(root) = env::var_os(CHILD_ROOT) {
        observe_build_environment(Path::new(&root));
        return;
    }
    let (root, context) = infrastructure_build_fixture();
    write_environment_probe(&context);
    let thread = std::thread::current();
    let observe = |root: &Path, label: &str| {
        let result =
            invoke_environment(root, thread.name().unwrap(), None, "alpha", Some("1"), None);
        eprintln!(
            "[CANIC-BUILD-MATRIX] {}",
            serde_json::json!({"case": label, "measurement": result})
        );
        result
    };
    let cold = observe(&root, "original_cold");
    let warm = observe(&root, "unchanged_warm");
    assert_eq!(cold["reused"], false);
    assert_eq!(warm["reused"], true);
    assert_eq!(cold["inputs"], warm["inputs"]);
    assert_eq!(cold["wasm"], warm["wasm"]);
    let relocated = crate::test_support::temp_dir("reuse-relocated-frozen");
    copy_frozen(&root, &relocated);
    let moved = observe(&relocated, "relocated_frozen_cold");
    assert_eq!(moved["reused"], false);
    assert_ne!(moved["inputs"], cold["inputs"]);
    assert_eq!(observe(&relocated, "relocated_frozen_warm")["reused"], true);
    fs::create_dir_all(context.workspace_root.join("docs")).unwrap();
    fs::write(
        context.workspace_root.join("docs/qualification.md"),
        b"local qualification evidence\n",
    )
    .unwrap();
    let qualification = observe(&root, "qualification_only");
    // Package trees are deliberately complete inputs: a build script may read any file.
    assert_eq!(qualification["reused"], false);
    assert_eq!(qualification["wasm"], warm["wasm"]);
    let source = context.workspace_root.join("src/lib.rs");
    let original = fs::read_to_string(&source).unwrap();
    fs::write(
        &source,
        original.replace("+ canic::value()", "+ canic::value() + 1"),
    )
    .unwrap();
    let runtime = observe(&root, "runtime_source");
    assert_eq!(runtime["reused"], false);
    assert_ne!(runtime["wasm"], qualification["wasm"]);
    let dependency = root.join("upstream/canic/src/lib.rs");
    let original = fs::read_to_string(&dependency).unwrap();
    fs::write(&dependency, original.replace("{ 1 }", "{ 2 }")).unwrap();
    let changed_dependency = observe(&root, "dependency_source");
    assert_eq!(changed_dependency["reused"], false);
    assert_ne!(changed_dependency["wasm"], runtime["wasm"]);
    fs::remove_dir_all(root).unwrap();
    fs::remove_dir_all(relocated).unwrap();
}

fn copy_frozen(source: &Path, target: &Path) {
    fs::create_dir_all(target).unwrap();
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        if entry.file_name() == "target" {
            continue;
        }
        let destination = target.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_frozen(&entry.path(), &destination);
        } else {
            fs::copy(entry.path(), destination).unwrap();
        }
    }
}
