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
fn install_truth_artifact_gate_blocks_missing_built_artifacts() {
    let root = temp_dir("canic-install-truth-artifact-gate");
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

[subnets.prime.canisters.user_hub]
kind = "service"
"#,
    )
    .expect("write config");
    write_wasm_gz_artifact(&root, "root", b"root-artifact");
    write_wasm_gz_artifact(&root, "wasm_store", b"wasm-store-artifact");

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

    assert!(
        check
            .report
            .hard_failures
            .iter()
            .any(|finding| finding.code == "artifact_missing"
                && finding.subject.as_deref() == Some("user_hub"))
    );
    assert!(enforce_install_deployment_truth_gate(&check).is_err());

    fs::remove_dir_all(root).expect("clean temp dir");
}

#[test]
fn install_truth_check_uses_supplied_deployment_plan_override() {
    let (root, mut check) = demo_install_deployment_truth_check(
        "canic-install-truth-supplied-deployment-plan-override",
    );
    check.plan.plan_id = "promoted-plan-1".to_string();
    let config_path = root.join("fleets/demo/canic.toml");
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
        deployment_plan_override: Some(check.plan),
        artifact_promotion_plan_override: None,
    };

    let supplied_check = current_install_deployment_truth_check_at(
        &options,
        &root,
        &root,
        &config_path,
        "demo",
        "2026-05-22T00:00:00Z".to_string(),
    )
    .expect("deployment truth check");

    assert_eq!(supplied_check.plan.plan_id, "promoted-plan-1");
    fs::remove_dir_all(root).expect("clean temp dir");
}

#[test]
fn install_truth_check_rejects_supplied_plan_network_mismatch() {
    let (root, mut check) =
        demo_install_deployment_truth_check("canic-install-truth-plan-network-mismatch");
    check.plan.deployment_identity.network = "ic".to_string();
    let config_path = root.join("fleets/demo/canic.toml");
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
        deployment_plan_override: Some(check.plan),
        artifact_promotion_plan_override: None,
    };

    current_install_deployment_truth_check_at(
        &options,
        &root,
        &root,
        &config_path,
        "demo",
        "2026-05-22T00:00:00Z".to_string(),
    )
    .expect_err("network mismatch should fail");

    fs::remove_dir_all(root).expect("clean temp dir");
}

#[test]
fn install_truth_check_rejects_supplied_plan_deployment_target_mismatch() {
    let (root, mut check) =
        demo_install_deployment_truth_check("canic-install-truth-plan-target-mismatch");
    check.plan.deployment_identity.deployment_name = "prod".to_string();
    let config_path = root.join("fleets/demo/canic.toml");
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
        deployment_plan_override: Some(check.plan),
        artifact_promotion_plan_override: None,
    };

    current_install_deployment_truth_check_at(
        &options,
        &root,
        &root,
        &config_path,
        "demo",
        "2026-05-22T00:00:00Z".to_string(),
    )
    .expect_err("deployment target mismatch should fail");

    fs::remove_dir_all(root).expect("clean temp dir");
}

#[test]
fn install_plan_artifact_validation_rejects_missing_root_wasm_before_mutation() {
    let (root, check) = demo_install_deployment_truth_check("canic-install-plan-missing-root-wasm");

    let Err(err) = validate_plan_artifacts_with_phase(&check.plan, &root, "local") else {
        panic!("missing root wasm should fail before install mutation");
    };

    assert!(
        err.to_string()
            .contains("deployment plan root wasm artifact does not exist")
    );
    fs::remove_dir_all(root).expect("clean temp dir");
}

#[test]
fn install_writes_artifact_promotion_execution_receipt_for_promotion_plan() {
    let (root, mut check) =
        demo_install_deployment_truth_check("canic-install-promotion-execution-receipt");
    let artifact = check
        .plan
        .role_artifacts
        .iter_mut()
        .find(|artifact| artifact.role == "root")
        .expect("root artifact");
    artifact.wasm_sha256 = Some(sample_sha256("d"));
    artifact.wasm_gz_sha256 = Some(sample_sha256("a"));
    artifact.observed_wasm_gz_file_sha256 = Some(sample_sha256("a"));
    artifact.canonical_embedded_config_sha256 = Some(sample_sha256("c"));
    let promotion_plan = sample_artifact_promotion_plan_for_install(&check);
    let execution_context = current_install_execution_context(&root, &root, "local");
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
        deployment_plan_override: Some(check.plan.clone()),
        artifact_promotion_plan_override: Some(promotion_plan.clone()),
    };
    let latest_deployment_receipt_before =
        latest_deployment_truth_receipt_path_from_root(&root, "local", "demo").ok();

    let path = write_artifact_promotion_execution_receipt_for_install(
        &options,
        &root,
        "local",
        "demo",
        &check,
        &execution_context,
    )
    .expect("promotion execution receipt write")
    .expect("promotion execution receipt path");
    let receipt: ArtifactPromotionExecutionReceiptV1 =
        serde_json::from_slice(&fs::read(&path).expect("read promotion receipt"))
            .expect("decode promotion receipt");

    assert!(
        path.display()
            .to_string()
            .contains("artifact-promotion-execution-receipts")
    );
    assert_eq!(receipt.artifact_promotion_plan_id, promotion_plan.plan_id);
    assert_eq!(receipt.promoted_plan_id, check.plan.plan_id);
    assert_eq!(receipt.deployment_receipt.plan_id, check.plan.plan_id);
    assert_eq!(receipt.roles.len(), 1);
    assert_eq!(receipt.roles[0].role, "root");
    assert_eq!(
        latest_deployment_truth_receipt_path_from_root(&root, "local", "demo").ok(),
        latest_deployment_receipt_before,
        "promotion wrapper emission must not update ordinary deployment receipt discovery"
    );
    fs::remove_dir_all(root).expect("clean temp dir");
}

#[test]
fn install_truth_artifact_gate_blocks_materialized_digest_drift() {
    let root = temp_dir("canic-install-truth-artifact-digest-gate");
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

    let mut check = current_install_deployment_truth_check_at(
        &options,
        &root,
        &root,
        &config_path,
        "demo",
        "2026-05-22T00:00:00Z".to_string(),
    )
    .expect("deployment truth check");
    check.plan.role_artifacts[0].observed_wasm_gz_file_sha256 =
        Some("different-observed-file-digest".to_string());
    check.diff = compare_plan_to_inventory(&check.plan, &check.inventory);
    check.report = safety_report_from_diff(
        "local:local:demo:report",
        Some("local:local:demo:diff".to_string()),
        &check.diff,
    );

    assert!(
        check
            .report
            .hard_failures
            .iter()
            .any(|finding| finding.code == "artifact_file_digest_mismatch")
    );
    assert!(enforce_install_deployment_truth_gate(&check).is_err());

    fs::remove_dir_all(root).expect("clean temp dir");
}

#[test]
fn install_truth_gate_blocks_observed_controller_drift() {
    let root = temp_dir("canic-install-truth-controller-gate");
    let config_path = root.join("fleets/demo/canic.toml");
    write_demo_root_only_config(&config_path);
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

    let mut check = current_install_deployment_truth_check_at(
        &options,
        &root,
        &root,
        &config_path,
        "demo",
        "2026-05-22T00:00:00Z".to_string(),
    )
    .expect("deployment truth check");
    check.plan.authority_profile.expected_controllers = vec!["aaaaa-aa".to_string()];
    check.inventory.observed_canisters = vec![ObservedCanisterV1 {
        canister_id: "aaaaa-aa".to_string(),
        role: Some("root".to_string()),
        control_class: CanisterControlClassV1::DeploymentControlled,
        controllers: vec!["external-controller".to_string()],
        module_hash: None,
        status: Some("running".to_string()),
        root_trust_anchor: Some("aaaaa-aa".to_string()),
        canonical_embedded_config_digest: None,
        role_assignment_source: Some("icp_canister_status".to_string()),
    }];
    check.diff = compare_plan_to_inventory(&check.plan, &check.inventory);
    check.report = safety_report_from_diff(
        "local:local:demo:report",
        Some("local:local:demo:diff".to_string()),
        &check.diff,
    );

    assert!(
        check
            .report
            .hard_failures
            .iter()
            .any(|finding| finding.code == "expected_controller_missing")
    );
    assert!(enforce_install_deployment_truth_gate(&check).is_err());
    let receipt = install_deployment_truth_gate_receipt(
        &check,
        "start".to_string(),
        vec![artifact_gate_phase_receipt(
            &check,
            "start",
            Some("finish".into()),
        )],
        artifact_gate_role_phase_receipts(&check),
    );
    let lines = install_deployment_truth_gate_lines(&check, &receipt);
    assert!(
        lines
            .iter()
            .any(|line| line.contains("Deployment truth blocker: diff:expected_controller_missing"))
    );
    assert!(lines.iter().any(|line| {
        line.contains("Deployment truth receipt:") && line.contains("status=FailedBeforeMutation")
    }));
    let err = enforce_install_deployment_truth_gate(&check).unwrap_err();
    assert!(
        err.to_string()
            .contains("diff:expected_controller_missing:"),
        "unexpected error: {err}"
    );

    fs::remove_dir_all(root).expect("clean temp dir");
}

#[test]
fn install_truth_gate_blocks_missing_expected_root_canister() {
    let root = temp_dir("canic-install-truth-missing-root-gate");
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

    let mut check = current_install_deployment_truth_check_at(
        &options,
        &root,
        &root,
        &config_path,
        "demo",
        "2026-05-22T00:00:00Z".to_string(),
    )
    .expect("deployment truth check");
    check.plan.expected_canisters[0].canister_id = Some("aaaaa-aa".to_string());
    check.inventory.observed_canisters = vec![ObservedCanisterV1 {
        canister_id: "different-root".to_string(),
        role: Some("root".to_string()),
        control_class: CanisterControlClassV1::DeploymentControlled,
        controllers: vec!["aaaaa-aa".to_string()],
        module_hash: None,
        status: Some("running".to_string()),
        root_trust_anchor: Some("different-root".to_string()),
        canonical_embedded_config_digest: None,
        role_assignment_source: Some("icp_canister_status".to_string()),
    }];
    check.diff = compare_plan_to_inventory(&check.plan, &check.inventory);
    check.report = safety_report_from_diff(
        "local:local:demo:report",
        Some("local:local:demo:diff".to_string()),
        &check.diff,
    );

    assert!(
        check
            .report
            .hard_failures
            .iter()
            .any(|finding| finding.code == "canister_missing")
    );
    let err = enforce_install_deployment_truth_gate(&check).unwrap_err();
    assert!(
        err.to_string().contains("canister_missing:"),
        "unexpected error: {err}"
    );

    fs::remove_dir_all(root).expect("clean temp dir");
}

#[test]
fn install_truth_gate_blocks_all_safety_report_hard_failures() {
    let root = temp_dir("canic-install-truth-all-hard-failures");
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

    let mut check = current_install_deployment_truth_check_at(
        &options,
        &root,
        &root,
        &config_path,
        "demo",
        "2026-05-22T00:00:00Z".to_string(),
    )
    .expect("deployment truth check");
    check.report.hard_failures.push(SafetyFindingV1 {
        code: "future_hard_failure".to_string(),
        message: "future deployment truth blocker".to_string(),
        severity: SafetySeverityV1::HardFailure,
        subject: Some("future.subject".to_string()),
    });

    let err = enforce_install_deployment_truth_gate(&check).unwrap_err();

    assert!(
        err.to_string().contains("future_hard_failure:"),
        "unexpected error: {err}"
    );

    fs::remove_dir_all(root).expect("clean temp dir");
}

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
        "emit_manifest",
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
            phase: "build_artifacts",
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
fn current_install_staging_evidence_records_release_set_transport_facts() {
    let manifest = RootReleaseSetManifest {
        release_version: "0.43.4".to_string(),
        entries: vec![ReleaseSetEntry {
            role: "user_hub".to_string(),
            template_id: "embedded:user_hub".to_string(),
            artifact_relative_path: "local/canisters/user_hub/user_hub.wasm.gz".to_string(),
            payload_size_bytes: 42,
            payload_sha256_hex: "payload-hash".to_string(),
            chunk_size_bytes: 1_048_576,
            chunk_sha256_hex: vec!["chunk-a".to_string(), "chunk-b".to_string()],
        }],
    };

    let evidence = current_install_staging_evidence(
        "aaaaa-aa",
        Path::new("/workspace/.icp/local/canisters/root.release-set.json"),
        &manifest,
    );

    assert!(evidence.contains(&"root_canister:aaaaa-aa".to_string()));
    assert!(evidence.contains(&"release_version:0.43.4".to_string()));
    assert!(evidence.contains(&"staging_receipts:1".to_string()));
    assert!(evidence.contains(&"staging_role:user_hub".to_string()));
    assert!(evidence.contains(&"staging_transport:WasmStore".to_string()));
    assert!(evidence.contains(&"staging_chunks_prepared:2".to_string()));
    assert!(evidence.contains(&"staging_chunks_published:2".to_string()));
    assert!(evidence.contains(&"staging_postcondition:Observed".to_string()));
    assert!(evidence.contains(&"staging_wasm_store:root:aaaaa-aa:bootstrap".to_string()));
}

#[test]
fn resolve_root_canister_operation_owns_current_install_evidence() {
    let operation = ResolveRootCanisterOperation::new(
        Path::new("/workspace/.icp"),
        "local",
        "root",
        Path::new("/workspace/fleets/demo/canic.toml"),
    );

    let evidence = operation.evidence("aaaaa-aa");

    assert_eq!(evidence, ["root_target:root", "root_canister:aaaaa-aa"]);
}

#[test]
fn build_install_targets_operation_owns_current_install_evidence() {
    let operation = BuildInstallTargetsOperation::new(
        "local",
        vec!["root".to_string(), "wasm_store".to_string()],
        Some(CanisterBuildProfile::Fast),
        Path::new("/workspace/fleets/demo/canic.toml"),
        Path::new("/workspace/.icp"),
    );

    assert_eq!(
        operation.evidence(),
        ["build_target:root", "build_target:wasm_store"]
    );
    assert_eq!(operation.role_names(), ["root", "wasm_store"]);
}

#[test]
fn emit_root_manifest_operation_owns_current_install_evidence() {
    let _operation = EmitRootManifestOperation::new(
        Path::new("/workspace"),
        Path::new("/workspace/.icp"),
        "local",
        Path::new("/workspace/fleets/demo/canic.toml"),
    );

    let evidence = EmitRootManifestOperation::evidence(Path::new(
        "/workspace/.icp/local/canisters/root.release-set.json",
    ));

    assert_eq!(
        evidence,
        ["manifest_path:/workspace/.icp/local/canisters/root.release-set.json"]
    );
}

#[test]
fn stage_release_set_operation_owns_current_install_staging_evidence() {
    let manifest = RootReleaseSetManifest {
        release_version: "0.43.6".to_string(),
        entries: vec![ReleaseSetEntry {
            role: "root".to_string(),
            template_id: "embedded:root".to_string(),
            artifact_relative_path: "local/canisters/root/root.wasm.gz".to_string(),
            payload_size_bytes: 84,
            payload_sha256_hex: "payload-hash".to_string(),
            chunk_size_bytes: 1_048_576,
            chunk_sha256_hex: vec!["chunk-a".to_string()],
        }],
    };
    let operation = StageReleaseSetOperation::new(
        Path::new("/workspace/.icp"),
        "local",
        "aaaaa-aa",
        Path::new("/workspace/.icp/local/canisters/root.release-set.json"),
        manifest,
    );

    let evidence = operation.evidence();

    assert!(evidence.contains(&"root_canister:aaaaa-aa".to_string()));
    assert!(evidence.contains(&"release_version:0.43.6".to_string()));
    assert!(evidence.contains(&"staging_role:root".to_string()));
    assert!(evidence.contains(&"staging_transport:WasmStore".to_string()));
    assert!(evidence.contains(&"staging_chunks_prepared:1".to_string()));
    assert!(evidence.contains(&"staging_chunks_published:1".to_string()));
}

#[test]
fn install_root_wasm_operation_owns_current_install_evidence() {
    let operation = InstallRootWasmOperation::new(
        Path::new("/workspace/.icp"),
        "local",
        "aaaaa-aa",
        PathBuf::from("/workspace/.icp/local/canisters/root/root.wasm"),
    );

    let evidence = operation.evidence();

    assert!(evidence.contains(&"root_canister:aaaaa-aa".to_string()));
    assert!(
        evidence.contains(&"root_wasm:/workspace/.icp/local/canisters/root/root.wasm".to_string())
    );
}

#[test]
fn ensure_root_cycles_operation_owns_current_install_evidence() {
    let operation = EnsureRootCyclesOperation::new(
        Path::new("/workspace/.icp"),
        "local",
        "aaaaa-aa",
        "fund_root_pre_bootstrap",
        "ensure local root minimum cycles before bootstrap",
        "pre-bootstrap",
    );

    let evidence = operation.evidence();

    assert!(evidence.contains(&"root_canister:aaaaa-aa".to_string()));
    assert!(evidence.contains(&"minimum_cycles:100000000000000".to_string()));
    assert!(evidence.contains(&"funding_phase:pre-bootstrap".to_string()));
}

#[test]
fn resume_bootstrap_operation_owns_current_install_evidence() {
    let operation = ResumeBootstrapOperation::new("local", "aaaaa-aa");

    let evidence = operation.evidence();

    assert_eq!(evidence, ["root_canister:aaaaa-aa"]);
}

#[test]
fn wait_root_ready_operation_owns_current_install_evidence() {
    let operation = WaitRootReadyOperation::new("local", "aaaaa-aa", 30);

    let evidence = operation.evidence();

    assert!(evidence.contains(&"root_canister:aaaaa-aa".to_string()));
    assert!(evidence.contains(&"timeout_seconds:30".to_string()));
}

#[test]
fn current_install_activation_phases_use_operation_runner() {
    let source = include_str!("../mod.rs");
    let activation = source_section(
        source,
        "fn run_root_activation_phases(",
        "#[derive(Clone, Copy)]",
    );

    for operation in [
        "install_operation",
        "pre_bootstrap_funding",
        "stage_operation",
        "resume_operation",
        "wait_ready_operation",
        "post_ready_funding",
    ] {
        assert!(
            activation.contains(&format!("run_operation(&{operation})")),
            "activation phase must run through operation runner: {operation}"
        );
    }
    assert!(
        !activation.contains("run_phase("),
        "activation phases must not manually wire receipt_scope.run_phase"
    );
}

#[test]
fn current_install_records_gates_before_activation_mutation() {
    let source = include_str!("../mod.rs");
    let install = source_section(
        source,
        "pub fn install_root(",
        "struct PreparedInstallTruth",
    );
    assert_before(
        install,
        "prepare_install_deployment_truth(",
        "run_root_activation_phases(",
    );

    let prepare = source_section(
        source,
        "fn prepare_install_deployment_truth(",
        "fn resolve_root_canister_with_phase(",
    );
    assert_before(
        prepare,
        "ensure_current_install_executor_capabilities(execution_context)?",
        "run_install_deployment_truth_safety_gate(",
    );

    let gate = source_section(
        source,
        "fn run_install_deployment_truth_safety_gate(",
        "fn enforce_install_deployment_truth_gate(",
    );
    assert_before(
        gate,
        "enforce_install_deployment_truth_gate(&deployment_truth_check)?",
        "write_current_install_execution_preflight_receipt(",
    );
    assert_before(
        gate,
        "write_current_install_execution_preflight_receipt(",
        "Ok(deployment_truth_check)",
    );
}

#[test]
fn current_install_check_paths_do_not_write_or_mutate_state() {
    let source = include_str!("../mod.rs");
    let check_paths = source_section(
        source,
        "pub fn check_install_deployment_truth(",
        "fn resolve_current_install_truth_inputs(",
    );

    for forbidden in [
        "write_install_state(",
        "write_install_state_with_deployment_truth_receipt(",
        "write_install_deployment_truth_receipt(",
        "write_current_install_execution_preflight_receipt(",
        "register_deployment_state(",
        "run_root_activation_phases(",
        "install_root(",
    ] {
        assert!(
            !check_paths.contains(forbidden),
            "read-only install check/preflight paths must not contain {forbidden}"
        );
    }
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
            "install_root",
            "install root wasm",
            vec!["root_canister:aaaaa-aa".to_string()],
            || Ok(()),
        )
        .expect("successful phase should record");
    let err = scope
        .run_phase(
            "stage_release_set",
            "stage root release set",
            vec!["manifest_path:/tmp/release-set.json".to_string()],
            || Err::<(), Box<dyn std::error::Error>>("stage failed".into()),
        )
        .expect_err("failed phase should return original error");
    scope
        .run_phase(
            "wait_ready",
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

#[test]
fn install_truth_gate_lines_include_warning_codes() {
    let root = temp_dir("canic-install-truth-warning-lines");
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
    write_wasm_gz_artifact(&root, "wasm_store", b"wasm-store-artifact");

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

    let mut check = current_install_deployment_truth_check_at(
        &options,
        &root,
        &root,
        &config_path,
        "demo",
        "2026-05-22T00:00:00Z".to_string(),
    )
    .expect("deployment truth check");
    check.report.warnings.push(SafetyFindingV1 {
        code: "observation_gap".to_string(),
        message: "live root status was not observed".to_string(),
        severity: SafetySeverityV1::Warning,
        subject: Some("live_canister_status.root".to_string()),
    });

    let receipt = install_deployment_truth_gate_receipt(
        &check,
        "start".to_string(),
        vec![artifact_gate_phase_receipt(
            &check,
            "start",
            Some("finish".into()),
        )],
        artifact_gate_role_phase_receipts(&check),
    );
    let lines = install_deployment_truth_gate_lines(&check, &receipt);

    assert!(lines.iter().any(|line| {
        line.contains("Deployment truth receipt:") && line.contains("status=Complete")
    }));
    assert!(lines.iter().any(|line| line.contains(
        "Deployment truth warning: inventory:observation_gap:live_canister_status.root"
    )));
    assert!(lines.iter().any(|line| {
        line.contains("Deployment truth role receipt: phase=materialize_artifacts role=root")
    }));

    fs::remove_dir_all(root).expect("clean temp dir");
}

#[test]
fn install_truth_gate_lines_distinguish_plan_assumptions() {
    let root = temp_dir("canic-install-truth-plan-assumption-lines");
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
    let receipt = install_deployment_truth_gate_receipt(
        &check,
        "start".to_string(),
        vec![artifact_gate_phase_receipt(
            &check,
            "start",
            Some("finish".into()),
        )],
        artifact_gate_role_phase_receipts(&check),
    );
    let lines = install_deployment_truth_gate_lines(&check, &receipt);

    assert!(lines.iter().any(|line| {
        line.contains("Deployment truth warning: plan:plan_assumption:local_state.root_canister_id")
    }));

    fs::remove_dir_all(root).expect("clean temp dir");
}
