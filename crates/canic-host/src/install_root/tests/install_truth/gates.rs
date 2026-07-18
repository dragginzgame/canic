use super::*;

#[test]
fn normal_named_network_install_checks_fresh_local_build_artifacts() {
    let root = temp_dir("canic-install-truth-named-environment-artifacts");
    let config_path = root.join("fleets/demo/canic.toml");
    write_demo_root_only_config(&config_path);
    write_wasm_gz_artifact(&root, "root", b"root-artifact");
    write_wasm_gz_artifact(&root, "wasm_store", b"wasm-store-artifact");
    let mut options = local_demo_install_options(&root);
    options.network = "staging".to_string();

    let check = current_install_deployment_truth_check_at(
        &options,
        &root,
        &root,
        &config_path,
        "demo",
        "2026-07-18T00:00:00Z".to_string(),
    )
    .expect("deployment truth check");

    assert_eq!(check.plan.deployment_identity.network, "staging");
    assert!(check.plan.role_artifacts.iter().all(|artifact| {
        artifact
            .wasm_gz_path
            .as_deref()
            .is_some_and(|path| path.contains(".icp/local/canisters"))
    }));
    assert_eq!(check.inventory.observed_artifacts.len(), 2);
    assert!(
        check
            .report
            .hard_failures
            .iter()
            .all(|finding| finding.code != "artifact_missing")
    );
    assert!(
        check
            .inventory
            .unresolved_observations
            .iter()
            .all(|gap| gap.key != "local_artifacts.root")
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
    let blocked = err
        .downcast_ref::<InstallRootBlockedError>()
        .expect("deployment-truth gate should retain its typed reason");
    assert_eq!(blocked.kind(), InstallRootBlockKind::DeploymentTruth);

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
    let blocked = err
        .downcast_ref::<InstallRootBlockedError>()
        .expect("deployment-truth gate should retain its typed reason");
    assert_eq!(blocked.kind(), InstallRootBlockKind::DeploymentTruth);

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
    let blocked = err
        .downcast_ref::<InstallRootBlockedError>()
        .expect("deployment-truth gate should retain its typed reason");
    assert_eq!(blocked.kind(), InstallRootBlockKind::DeploymentTruth);

    fs::remove_dir_all(root).expect("clean temp dir");
}
