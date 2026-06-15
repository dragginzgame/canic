use super::*;

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
