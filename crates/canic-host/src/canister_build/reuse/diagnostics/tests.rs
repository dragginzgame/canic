use super::*;
use crate::test_support::temp_dir;
use std::{collections::BTreeMap, env, fs};

#[test]
fn isolated_invocations_report_a_synthetic_environment_change_and_repeat_exactly() {
    const CHILD_ROOT: &str = "CANIC_TEST_DIAGNOSTIC_CHILD_ROOT";
    const INPUT_KEY: &str = "CANIC_TEST_SYNTHETIC_BUILD_INPUT";
    if let Some(root) = env::var_os(CHILD_ROOT) {
        let root = PathBuf::from(root);
        let (environment, environment_keys) =
            environment_evidence(crate::build_environment::inputs());
        InputDiagnostics {
            schema_version: 1,
            environment,
            environment_keys,
            source: "fixed-source".into(),
            configuration: "fixed-configuration".into(),
        }
        .retain(&root);
        return;
    }
    let root = temp_dir("reuse-isolated-environment");
    let thread = std::thread::current();
    let invoke = |value: Option<&str>| {
        let mut command = std::process::Command::new(env::current_exe().unwrap());
        command
            .args(["--exact", thread.name().unwrap()])
            .env(CHILD_ROOT, &root)
            .env_remove(INPUT_KEY);
        if let Some(value) = value {
            command.env(INPUT_KEY, value);
        }
        let output = command.output().unwrap();
        assert!(output.status.success());
        let bytes = fs::read(root.join("last-input-diagnostics.json")).unwrap();
        assert!(!String::from_utf8_lossy(&bytes).contains("synthetic-build-setting"));
        serde_json::from_slice::<InputDiagnostics>(&bytes).unwrap()
    };
    let baseline = invoke(None);
    let candidate = invoke(Some("synthetic-build-setting"));
    let reason = candidate.compare(&baseline);
    assert!(reason.contains(INPUT_KEY));
    assert!(reason.contains("environment changed"));
    assert!(!reason.contains("synthetic-build-setting"));
    let repeat = invoke(Some("synthetic-build-setting"));
    assert_eq!(candidate.environment, repeat.environment);
    assert_eq!(candidate.environment_keys, repeat.environment_keys);
    let changed_value = invoke(Some("changed-synthetic-build-setting"));
    assert!(
        changed_value
            .compare(&candidate)
            .contains("key attribution unavailable")
    );
    fs::remove_dir_all(root).unwrap();
}

fn evidence(environment: &[(&str, &str)]) -> InputDiagnostics {
    let (environment, environment_keys) = environment_evidence(
        environment
            .iter()
            .map(|(key, value)| (OsString::from(key), OsString::from(value)))
            .collect(),
    );
    InputDiagnostics {
        schema_version: 1,
        environment,
        environment_keys,
        source: "source".into(),
        configuration: "configuration".into(),
    }
}

#[test]
fn environment_comparison_names_added_keys_without_storing_values() {
    let root = temp_dir("reuse-diagnostics-environment");
    let baseline = evidence(&[("PATH", "synthetic-path")]);
    baseline.retain(&root);
    let changed = evidence(&[
        ("PATH", "synthetic-path"),
        ("CANIC_TEST_BUILD_INPUT", "synthetic-value"),
    ]);
    let reason = changed.explain_miss(&root);
    assert!(reason.contains("environment changed"));
    assert!(reason.contains("CANIC_TEST_BUILD_INPUT"));
    assert!(!reason.contains("synthetic-value"));
    let bytes = serde_json::to_string(&changed).unwrap();
    assert!(!bytes.contains("synthetic-value"));
    assert!(!bytes.contains("synthetic-path"));
    let value_changed = evidence(&[("PATH", "another-path")]);
    assert!(
        value_changed
            .compare(&baseline)
            .contains("key attribution unavailable")
    );
    assert_ne!(value_changed.environment, baseline.environment);
    assert_eq!(
        evidence(&[("PATH", "synthetic-path")]).environment,
        baseline.environment
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn missing_corrupt_and_unsafe_diagnostics_remain_bounded_and_optional() {
    let root = temp_dir("reuse-diagnostics-invalid");
    let current = evidence(&[("SAFE_KEY", "value")]);
    assert_eq!(
        current.explain_miss(&root),
        "no comparable prior input evidence"
    );
    write_bytes(&root.join("last-input-diagnostics.json"), b"invalid").unwrap();
    assert_eq!(
        current.explain_miss(&root),
        "no comparable prior input evidence"
    );
    let mut previous = evidence(&[]);
    previous.environment_keys.insert("BAD\nKEY".into());
    previous.environment_keys.insert("x".repeat(1000));
    previous
        .environment_keys
        .extend((0..100).map(|i| format!("KEY_{i}")));
    let reason = current.compare(&previous);
    assert!(!reason.contains('\n'));
    assert!(reason.len() < 1000);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn source_and_configuration_changes_are_separately_attributed() {
    let root = PathBuf::from("/synthetic-reuse");
    let context = WorkspaceBuildContext {
        role: "root".into(),
        profile: crate::canister_build::CanisterBuildProfile::Fast,
        environment: "local".into(),
        build_network: canic_core::ids::BuildNetwork::Local,
        workspace_root: root.clone(),
        icp_root: root.clone(),
        config_path: root.join("canic.toml"),
        local_replica: None,
        refresh_canonical_infrastructure_did: false,
        release_build_id: None,
    };
    let mut inputs = BuildInputSnapshot {
        identity: "unchanged-authority".into(),
        files: BTreeMap::from([
            ("/synthetic-reuse/src/lib.rs".into(), "old-source".into()),
            ("/synthetic-reuse/canic.toml".into(), "old-config".into()),
        ]),
    };
    let previous = InputDiagnostics::capture(&context, &[], &inputs);
    inputs
        .files
        .insert("/synthetic-reuse/src/lib.rs".into(), "new-source".into());
    let source = InputDiagnostics::capture(&context, &[], &inputs);
    assert!(
        source
            .compare(&previous)
            .contains("source/dependency inputs changed")
    );
    assert!(
        !source
            .compare(&previous)
            .contains("toolchain/configuration")
    );
    inputs
        .files
        .insert("/synthetic-reuse/canic.toml".into(), "new-config".into());
    let configuration = InputDiagnostics::capture(&context, &[], &inputs);
    assert!(
        configuration
            .compare(&source)
            .contains("toolchain/configuration inputs changed")
    );
    assert!(!configuration.compare(&source).contains("source/dependency"));
}
