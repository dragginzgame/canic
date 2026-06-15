use super::*;

#[test]
fn install_truth_preflight_uses_current_install_inputs_without_mutation() {
    with_guarded_env(|| {
        let root = temp_dir("canic-install-truth-preflight");
        fs::create_dir_all(root.join("fleets/demo")).expect("create config dir");
        fs::write(
            root.join("fleets/demo/canic.toml"),
            demo_config_source(
                r#"
[subnets.prime.canisters.root]
kind = "root"

[subnets.prime.canisters.user_hub]
kind = "service"
"#,
            ),
        )
        .expect("write config");
        write_wasm_gz_artifact(&root, "root", b"root-artifact");
        write_wasm_gz_artifact(&root, "wasm_store", b"wasm-store-artifact");
        write_wasm_gz_artifact(&root, "user_hub", b"user-hub-artifact");
        let previous_workspace_root = env::var_os("CANIC_WORKSPACE_ROOT");
        unsafe {
            env::set_var("CANIC_WORKSPACE_ROOT", &root);
        }

        let options = local_demo_install_options(&root);

        let check = check_install_deployment_truth(&options, "2026-05-22T00:00:00Z")
            .expect("install truth preflight");
        let execution_preflight =
            check_install_execution_preflight(&options, "2026-05-22T00:00:01Z")
                .expect("install execution preflight");

        assert_eq!(check.check_id, "local:local:demo:check");
        assert_eq!(check.plan.fleet_template, "demo");
        assert_eq!(
            check
                .plan
                .role_artifacts
                .iter()
                .map(|artifact| artifact.build_profile.as_str())
                .collect::<Vec<_>>(),
            vec!["fast", "fast", "fast"]
        );
        assert_eq!(check.inventory.observed_artifacts.len(), 3);
        enforce_install_deployment_truth_gate(&check)
            .expect("complete local artifacts should pass gate");
        assert_eq!(execution_preflight.plan_id, check.plan.plan_id);
        assert_eq!(
            execution_preflight.backend,
            DeploymentExecutorBackendV1::CurrentCli
        );
        assert!(execution_preflight.missing_capabilities.is_empty());
        assert_eq!(
            execution_preflight.status,
            DeploymentExecutionPreflightStatusV1::Ready
        );
        assert!(execution_preflight.blockers.is_empty());
        assert!(!root.join(".canic").exists());

        restore_env_var("CANIC_WORKSPACE_ROOT", previous_workspace_root);
        fs::remove_dir_all(root).expect("clean temp dir");
    });
}

#[test]
fn install_truth_execution_preflight_receipt_records_ready_state() {
    let (root, mut check) =
        demo_install_deployment_truth_check("canic-install-truth-execution-preflight-ready");
    check.plan.expected_canisters.clear();
    check.report.status = SafetyStatusV1::Safe;
    check.report.summary = "deployment inventory matches plan".to_string();
    check.report.hard_failures.clear();
    let execution_context = current_install_execution_context(&root, &root, "local");

    let path = write_current_install_execution_preflight_receipt(
        &root,
        "local",
        "demo",
        &check,
        &execution_context,
    )
    .expect("write execution preflight receipt");
    let receipt: DeploymentReceiptV1 =
        serde_json::from_slice(&fs::read(path).expect("read receipt")).expect("decode receipt");

    assert_eq!(
        receipt.operation_id,
        "local:local:demo:check:execution_preflight"
    );
    assert_eq!(
        receipt.operation_status,
        DeploymentExecutionStatusV1::Complete
    );
    assert_eq!(receipt.phase_receipts[0].phase, "execution_preflight");
    assert!(
        receipt.phase_receipts[0]
            .verified_postcondition
            .evidence
            .contains(&"execution_preflight_status:Ready".to_string())
    );
    assert!(
        receipt.phase_receipts[0]
            .verified_postcondition
            .evidence
            .contains(&"blockers:0".to_string())
    );
    assert!(receipt.execution_context.is_some());

    fs::remove_dir_all(root).expect("clean temp dir");
}

#[test]
fn install_truth_execution_preflight_receipt_records_blocked_state_before_error() {
    let (root, mut check) =
        demo_install_deployment_truth_check("canic-install-truth-execution-preflight-blocked");
    check.report.status = SafetyStatusV1::Blocked;
    check.report.hard_failures.push(SafetyFindingV1 {
        code: "deployment_artifact_missing".to_string(),
        message: "planned artifact was not observed".to_string(),
        severity: SafetySeverityV1::HardFailure,
        subject: Some("root".to_string()),
    });
    let execution_context = current_install_execution_context(&root, &root, "local");

    let err = write_current_install_execution_preflight_receipt(
        &root,
        "local",
        "demo",
        &check,
        &execution_context,
    )
    .expect_err("blocked execution preflight should stop install");

    assert!(
        err.to_string()
            .contains("deployment execution preflight blocked install")
    );
    let path = latest_deployment_truth_receipt_path_from_root(&root, "local", "demo")
        .expect("find latest receipt")
        .expect("blocked preflight receipt should be written");
    let receipt: DeploymentReceiptV1 =
        serde_json::from_slice(&fs::read(path).expect("read receipt")).expect("decode receipt");
    assert_eq!(
        receipt.operation_status,
        DeploymentExecutionStatusV1::FailedBeforeMutation
    );
    assert!(
        receipt.phase_receipts[0]
            .verified_postcondition
            .evidence
            .contains(&"execution_preflight_status:Blocked".to_string())
    );
    assert!(
        receipt.phase_receipts[0]
            .verified_postcondition
            .evidence
            .iter()
            .any(|line| line.starts_with("blocker:deployment_artifact_missing:"))
    );

    fs::remove_dir_all(root).expect("clean temp dir");
}

#[test]
fn install_truth_reports_executor_missing_required_capabilities() {
    let context = DeploymentExecutionContextV1 {
        workspace_root: Some("/workspace/canic".to_string()),
        icp_root: Some("/workspace/canic/.icp".to_string()),
        artifact_roots: vec!["/workspace/canic/.icp/local/canisters".to_string()],
        backend: DeploymentExecutorBackendV1::Other {
            name: "limited-test-backend".to_string(),
        },
        backend_capabilities: vec![DeploymentExecutorCapabilityV1::CanisterStatus],
    };

    assert_eq!(
        current_install_executor_missing_capabilities(&context),
        vec![
            DeploymentExecutorCapabilityV1::CreateCanister,
            DeploymentExecutorCapabilityV1::InstallCode,
            DeploymentExecutorCapabilityV1::Call,
            DeploymentExecutorCapabilityV1::Query,
            DeploymentExecutorCapabilityV1::StageArtifact,
        ],
    );
}
