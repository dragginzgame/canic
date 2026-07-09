use super::*;

#[test]
fn install_truth_gate_persists_machine_readable_receipt() {
    let root = temp_dir("canic-install-truth-receipt-json");
    let config_path = root.join("fleets/demo/canic.toml");
    fs::create_dir_all(config_path.parent().expect("config parent")).expect("create config dir");
    fs::write(
        &config_path,
        demo_config_source(
            r#"
[subnets.prime.canisters.root]
kind = "root"
"#,
        ),
    )
    .expect("write config");
    write_wasm_gz_artifact(&root, "root", b"root-artifact");

    let options = local_demo_install_options(&root);

    let check = current_install_deployment_truth_check_at(
        &options,
        &root,
        &root,
        &config_path,
        "demo",
        "2026-05-22T00:00:00Z".to_string(),
    )
    .expect("deployment truth check");
    let receipt = install_deployment_truth_gate_receipt(
        &check,
        "unix:1770000000".to_string(),
        vec![artifact_gate_phase_receipt(
            &check,
            "unix:1770000000",
            Some("unix:1770000001".to_string()),
        )],
        artifact_gate_role_phase_receipts(&check),
    );

    let path = write_install_deployment_truth_receipt(&root, "local", "demo", &receipt)
        .expect("write deployment truth receipt");
    let expected_path = install_deployment_truth_receipt_path(&root, "local", "demo", &receipt)
        .expect("receipt path");

    assert_eq!(path, expected_path);
    assert_eq!(
        path.parent()
            .and_then(Path::file_name)
            .and_then(|name| name.to_str()),
        Some("demo")
    );
    assert!(
        path.file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| {
                !name.contains(':')
                    && Path::new(name)
                        .extension()
                        .is_some_and(|ext| ext.eq_ignore_ascii_case("json"))
            }),
        "unexpected receipt path: {}",
        path.display()
    );
    let decoded: DeploymentReceiptV1 =
        serde_json::from_slice(&fs::read(&path).expect("read receipt")).expect("decode receipt");
    assert_eq!(decoded, receipt);

    fs::remove_dir_all(root).expect("clean temp dir");
}

#[test]
fn install_truth_phase_receipt_records_emit_manifest_evidence() {
    let root = temp_dir("canic-install-truth-emit-manifest-receipt");
    let config_path = root.join("fleets/demo/canic.toml");
    fs::create_dir_all(config_path.parent().expect("config parent")).expect("create config dir");
    fs::write(
        &config_path,
        r#"
controllers = []
app_index = []

[fleet]
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

[app]
init_mode = "enabled"
[app.whitelist]

[subnets.prime.canisters.root]
kind = "root"
"#,
    )
    .expect("write config");
    write_wasm_gz_artifact(&root, "root", b"root-artifact");

    let options = InstallRootOptions {
        root_canister: "root".to_string(),
        root_build_target: "root".to_string(),
        network: "local".to_string(),
        deployment_name: None,
        icp_root: Some(root.clone()),
        build_profile: Some(CanisterBuildProfile::Fast),
        ready_timeout_seconds: 30,
        config_path: Some("fleets/demo/canic.toml".to_string()),
        expected_fleet: Some("demo".to_string()),
        interactive_config_selection: false,
        deployment_plan_override: None,
        artifact_promotion_plan_override: None,
    };
    let check = current_install_deployment_truth_check_at(
        &options,
        &root,
        &root,
        &config_path,
        "demo",
        "2026-05-22T00:00:00Z".to_string(),
    )
    .expect("deployment truth check");

    let receipt = install_deployment_truth_phase_receipt(
        &check,
        InstallPhaseLabel::EMIT_MANIFEST,
        "unix:1770000002".to_string(),
        Some("unix:1770000003".to_string()),
        "emit root release-set manifest",
        ObservationStatusV1::Observed,
        vec!["manifest_path:/tmp/manifest.json".to_string()],
    );

    assert_eq!(
        receipt.operation_status,
        DeploymentExecutionStatusV1::Complete
    );
    assert_eq!(receipt.operation_id, "local:local:demo:check:emit_manifest");
    assert_eq!(receipt.phase_receipts.len(), 1);
    assert_eq!(receipt.phase_receipts[0].phase, "emit_manifest");
    assert_eq!(
        receipt.phase_receipts[0].verified_postcondition.status,
        ObservationStatusV1::Observed
    );
    assert_eq!(
        receipt.phase_receipts[0].verified_postcondition.evidence,
        vec!["manifest_path:/tmp/manifest.json".to_string()]
    );

    fs::remove_dir_all(root).expect("clean temp dir");
}

#[test]
fn install_truth_completed_phase_receipt_records_pre_gate_evidence() {
    let (root, check) = demo_install_deployment_truth_check("canic-install-truth-pre-gate-phase");
    let execution_context = current_install_execution_context(&root, &root, "local");
    let scope = InstallReceiptScope {
        icp_root: &root,
        network: "local",
        deployment_name: "demo",
        check: &check,
        execution_context: Some(&execution_context),
    };

    let path = write_completed_install_phase_receipt(
        scope,
        CompletedInstallPhase {
            phase: InstallPhaseLabel::BUILD_ARTIFACTS,
            attempted_action: "build configured install targets",
            started_at: "unix:1770000004".to_string(),
            finished_at: Some("unix:1770000005".to_string()),
            evidence: vec!["build_target:root".to_string()],
            role_names: vec!["root".to_string()],
        },
    )
    .expect("write completed phase receipt");
    let receipt: DeploymentReceiptV1 =
        serde_json::from_slice(&fs::read(path).expect("read receipt")).expect("decode receipt");

    assert_eq!(
        receipt.operation_id,
        "local:local:demo:check:build_artifacts"
    );
    assert_eq!(
        receipt.operation_status,
        DeploymentExecutionStatusV1::Complete
    );
    assert_eq!(receipt.phase_receipts[0].phase, "build_artifacts");
    assert_eq!(
        receipt.phase_receipts[0].verified_postcondition.evidence,
        vec!["build_target:root".to_string()]
    );
    assert_eq!(receipt.role_phase_receipts.len(), 1);
    assert_eq!(receipt.role_phase_receipts[0].role, "root");
    assert_eq!(receipt.role_phase_receipts[0].phase, "build_artifacts");
    assert_eq!(
        receipt.role_phase_receipts[0].result,
        crate::deployment_truth::RolePhaseResultV1::Applied
    );
    let execution_context = receipt
        .execution_context
        .expect("completed phase receipt should include execution context");
    assert_eq!(
        execution_context.backend,
        crate::deployment_truth::DeploymentExecutorBackendV1::CurrentCli
    );
    assert!(
        execution_context
            .artifact_roots
            .iter()
            .any(|root| { root.ends_with(".icp/local/canisters") })
    );

    fs::remove_dir_all(root).expect("clean temp dir");
}

#[test]
fn install_truth_receipted_phase_records_success_and_failure() {
    let (root, check) = demo_install_deployment_truth_check("canic-install-truth-receipted-phase");
    let execution_context = current_install_execution_context(&root, &root, "local");
    let scope = InstallReceiptScope {
        icp_root: &root,
        network: "local",
        deployment_name: "demo",
        check: &check,
        execution_context: Some(&execution_context),
    };

    scope
        .run_phase(
            InstallPhaseLabel::INSTALL_ROOT,
            "install root wasm",
            vec!["root_canister:aaaaa-aa".to_string()],
            || Ok(()),
        )
        .expect("successful phase should record");
    let err = scope
        .run_phase(
            InstallPhaseLabel::STAGE_RELEASE_SET,
            "stage root release set",
            vec!["manifest_path:/tmp/release-set.json".to_string()],
            || Err::<(), Box<dyn std::error::Error>>("stage failed".into()),
        )
        .expect_err("failed phase should return original error");
    scope
        .run_phase(
            InstallPhaseLabel::WAIT_READY,
            "wait for root bootstrap readiness",
            vec!["timeout_seconds:30".to_string()],
            || Ok(()),
        )
        .expect("wait-ready phase should record");

    assert_eq!(err.to_string(), "stage failed");

    let receipt_dir = root.join(".canic/local/deployment-receipts/demo");
    let receipts = fs::read_dir(&receipt_dir)
        .expect("read receipts")
        .map(|entry| {
            let path = entry.expect("receipt entry").path();
            serde_json::from_slice::<DeploymentReceiptV1>(
                &fs::read(path).expect("read receipt JSON"),
            )
            .expect("decode receipt")
        })
        .collect::<Vec<_>>();
    let install = receipts
        .iter()
        .find(|receipt| receipt.operation_id.ends_with(":install_root"))
        .expect("install receipt");
    let stage = receipts
        .iter()
        .find(|receipt| receipt.operation_id.ends_with(":stage_release_set"))
        .expect("stage receipt");
    let wait = receipts
        .iter()
        .find(|receipt| receipt.operation_id.ends_with(":wait_ready"))
        .expect("wait-ready receipt");

    assert_eq!(
        install.operation_status,
        DeploymentExecutionStatusV1::Complete
    );
    assert_eq!(
        install.phase_receipts[0].verified_postcondition.status,
        ObservationStatusV1::Observed
    );
    assert_eq!(
        stage.operation_status,
        DeploymentExecutionStatusV1::FailedAfterMutation
    );
    assert_eq!(
        stage.phase_receipts[0].verified_postcondition.status,
        ObservationStatusV1::Inconclusive
    );
    assert_eq!(wait.operation_status, DeploymentExecutionStatusV1::Complete);
    assert_eq!(
        wait.phase_receipts[0].verified_postcondition.status,
        ObservationStatusV1::Observed
    );

    fs::remove_dir_all(root).expect("clean temp dir");
}

#[test]
fn install_truth_latest_receipt_uses_newest_persisted_receipt() {
    let root = temp_dir("canic-install-truth-latest-receipt");
    let receipt_dir = root.join(".canic/local/deployment-receipts/demo");
    fs::create_dir_all(&receipt_dir).expect("create receipt dir");
    let older = receipt_dir.join("unix_100-local_demo_check_materialize_artifacts.json");
    let newer = receipt_dir.join("unix_200-local_demo_check_materialize_artifacts.json");
    let ignored = receipt_dir.join("unix_300-local_demo_check_materialize_artifacts.txt");
    fs::write(&older, "{}").expect("write older receipt");
    fs::write(&newer, "{}").expect("write newer receipt");
    fs::write(ignored, "{}").expect("write ignored file");

    let latest = latest_deployment_truth_receipt_path_from_root(&root, "local", "demo")
        .expect("latest receipt")
        .expect("receipt exists");

    assert_eq!(latest, newer);

    fs::remove_dir_all(root).expect("clean temp dir");
}
