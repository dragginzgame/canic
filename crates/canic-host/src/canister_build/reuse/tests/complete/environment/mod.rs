//! Qualify credential-independent reuse with real Cargo inputs and synthetic release manifests.

use super::*;
use crate::icp::CANIC_ICP_IDENTITY_PASSWORD_FILE_ENV;

const CHILD_ROOT: &str = "CANIC_TEST_BUILD_ENVIRONMENT_ROOT";
const BUILD_INPUT: &str = "CANIC_TEST_BUILD_INPUT";

#[test]
fn deployment_credentials_preserve_reuse_while_build_inputs_invalidate() {
    if let Some(root) = env::var_os(CHILD_ROOT) {
        observe_build_environment(Path::new(&root));
        return;
    }
    let (root, context) = infrastructure_build_fixture();
    fs::write(
        context.workspace_root.join("build.rs"),
        r#"
fn main() {
    assert!(std::env::var_os("CANIC_ICP_IDENTITY_PASSWORD_FILE").is_none());
    let input = std::env::var("CANIC_TEST_BUILD_INPUT").unwrap();
    println!("cargo:rustc-env=CANIC_TEST_COMPILED_INPUT={input}");
    println!("cargo:rerun-if-env-changed=CANIC_TEST_BUILD_INPUT");
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
#[unsafe(no_mangle)]
pub extern "C" fn fixture_value() -> u8 {
    env!("CANIC_TEST_COMPILED_INPUT").as_bytes()[0] + canic::value()
}
"#,
    )
    .unwrap();
    let thread = std::thread::current();
    let invoke = |credential: Option<&str>, input: &str| {
        let mut command = Command::new(env::current_exe().unwrap());
        command
            .args(["--exact", thread.name().unwrap()])
            .env(CHILD_ROOT, &root)
            .env(BUILD_INPUT, input)
            .env("CARGO_TARGET_DIR", root.join("target"))
            .env_remove("CARGO_BUILD_BUILD_DIR")
            .env_remove(CANIC_ICP_IDENTITY_PASSWORD_FILE_ENV);
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
    };
    let baseline = invoke(Some("fixture-credential-a"), "alpha");
    assert_eq!(baseline["reused"], false);
    for credential in [
        Some("fixture-credential-b"),
        None,
        Some("fixture-credential-a"),
    ] {
        let repeat = invoke(credential, "alpha");
        assert_eq!(repeat["reused"], true);
        for key in ["inputs", "release", "wasm"] {
            assert_eq!(repeat[key], baseline[key], "unchanged {key}");
        }
    }
    let changed = invoke(Some("fixture-credential-a"), "beta");
    assert_eq!(changed["reused"], false);
    assert_ne!(changed["inputs"], baseline["inputs"]);
    assert_ne!(changed["wasm"], baseline["wasm"]);
    assert!(
        changed["miss_reason"]
            .as_str()
            .unwrap()
            .contains("changed-value keys, up to 8: CANIC_TEST_BUILD_INPUT")
    );
    let repeat = invoke(Some("fixture-credential-b"), "beta");
    assert_eq!(repeat["reused"], true);
    assert_eq!(repeat["inputs"], changed["inputs"]);
    assert_eq!(repeat["release"], changed["release"]);
    let dependency = root.join("upstream/canic/src/lib.rs");
    let source = fs::read_to_string(&dependency).unwrap();
    fs::write(&dependency, source.replace("{ 1 }", "{ 2 }")).unwrap();
    let changed_source = invoke(Some("fixture-credential-a"), "beta");
    assert_eq!(changed_source["reused"], false);
    assert_ne!(changed_source["inputs"], changed["inputs"]);
    assert_ne!(changed_source["wasm"], changed["wasm"]);
    let config = fs::read_to_string(&context.config_path).unwrap();
    fs::write(
        &context.config_path,
        config.replace("maximum_instances = 1", "maximum_instances = 2"),
    )
    .unwrap();
    let changed_config = invoke(Some("fixture-credential-a"), "beta");
    assert_eq!(changed_config["reused"], false);
    assert_ne!(changed_config["inputs"], changed_source["inputs"]);
    fs::remove_dir_all(root).unwrap();
}

fn observe_build_environment(root: &Path) {
    let context = infrastructure_build_context(root.join("app"));
    let before = prepared_reuse(&context)
        .load()
        .unwrap()
        .map(|hit| hit.release_build_id);
    // Cargo/build.rs and rustc both assert the credential is absent. This real
    // compiler output is measured separately from the synthetic sealed manifest.
    compile_infrastructure_fixture(&context);
    let reuse = prepared_reuse(&context);
    let hit = reuse.load().unwrap();
    assert_eq!(before, hit.as_ref().map(|hit| hit.release_build_id));
    let reused = hit.is_some();
    let miss_reason = reuse.miss_reason();
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
    assert!(!diagnostics.contains(CANIC_ICP_IDENTITY_PASSWORD_FILE_ENV));
    fs::write(
        root.join("build-observation.json"),
        serde_json::to_vec(&serde_json::json!({
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
