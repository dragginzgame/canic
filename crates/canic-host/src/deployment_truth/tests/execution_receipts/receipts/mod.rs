use super::super::*;

#[test]
fn artifact_gate_receipt_records_materialized_artifact_evidence() {
    let mut plan = sample_plan();
    plan.expected_verifier_readiness.required = false;
    let inventory = DeploymentInventoryV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        inventory_id: "inventory-1".to_string(),
        observed_at: "2026-05-22T00:00:00Z".to_string(),
        observed_identity: Some(sample_identity()),
        observed_root: None,
        local_config: LocalDeploymentConfigV1 {
            config_path: Some("canic.toml".to_string()),
            raw_config_sha256: None,
            canonical_embedded_config_sha256: Some("runtime".to_string()),
        },
        observed_canisters: vec![ObservedCanisterV1 {
            canister_id: "aaaaa-aa".to_string(),
            role: Some("root".to_string()),
            control_class: CanisterControlClassV1::DeploymentControlled,
            controllers: vec!["aaaaa-aa".to_string()],
            module_hash: None,
            status: None,
            root_trust_anchor: Some("aaaaa-aa".to_string()),
            canonical_embedded_config_digest: None,
            role_assignment_source: Some(
                RoleAssignmentSourceV1::LocalInstallState
                    .label()
                    .to_string(),
            ),
        }],
        observed_pool: Vec::new(),
        observed_artifacts: vec![ObservedArtifactV1 {
            role: "root".to_string(),
            artifact_path: "root.wasm.gz".to_string(),
            file_sha256: Some("file".to_string()),
            file_sha256_source: Some(ArtifactDigestSourceV1::ObservedFileDigest),
            payload_sha256: Some("gzip".to_string()),
            payload_size_bytes: Some(10),
            source: ArtifactSourceV1::LocalBuild,
        }],
        observed_verifier_readiness: VerifierReadinessObservationV1 {
            status: ObservationStatusV1::NotObserved,
            role_epochs: Vec::new(),
        },
        unresolved_observations: Vec::new(),
    };
    let diff = compare_plan_to_inventory(&plan, &inventory);
    let report = safety_report_from_diff("report-1", Some("diff-1".to_string()), &diff);
    let check = DeploymentCheckV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        check_id: "check-1".to_string(),
        plan,
        inventory,
        diff,
        report,
    };

    let phase = artifact_gate_phase_receipt(
        &check,
        "2026-05-22T00:00:00Z",
        Some("2026-05-22T00:00:01Z".to_string()),
    );
    let role_receipts = artifact_gate_role_phase_receipts(&check);
    let receipt = deployment_receipt_from_check(
        &check,
        "operation-1",
        "2026-05-22T00:00:00Z",
        Some("2026-05-22T00:00:01Z".to_string()),
        vec![phase.clone()],
        role_receipts.clone(),
        DeploymentCommandResultV1::Succeeded,
    );

    assert_eq!(phase.phase, "materialize_artifacts");
    assert_eq!(
        phase.verified_postcondition.status,
        ObservationStatusV1::Observed
    );
    assert_eq!(
        phase.verified_postcondition.evidence,
        vec!["artifact:root:sha256:file"]
    );
    assert_eq!(receipt.plan_id, "plan-local-root");
    assert_eq!(
        receipt.operation_status,
        DeploymentExecutionStatusV1::Complete
    );
    assert_eq!(receipt.final_inventory_id.as_deref(), Some("inventory-1"));
    assert_eq!(role_receipts.len(), 1);
    assert_eq!(role_receipts[0].role, "root");
    assert_eq!(
        role_receipts[0].result,
        RolePhaseResultV1::VerifiedAlreadyApplied
    );
    assert_eq!(role_receipts[0].artifact_digest.as_deref(), Some("file"));
    assert_eq!(receipt.role_phase_receipts, role_receipts);
    assert_eq!(receipt.phase_receipts, vec![phase]);
}

#[test]
fn execution_status_classifier_marks_failed_before_mutation_without_applied_roles() {
    let status = deployment_execution_status_for_receipt_parts(
        &DeploymentCommandResultV1::Failed {
            code: "preflight_blocked".to_string(),
            message: "blocked before mutation".to_string(),
        },
        &[sample_role_phase_receipt(RolePhaseResultV1::NotAttempted)],
    );

    assert_eq!(status, DeploymentExecutionStatusV1::FailedBeforeMutation);
}

#[test]
fn execution_status_classifier_marks_failed_after_mutation_with_applied_role() {
    let status = deployment_execution_status_for_receipt_parts(
        &DeploymentCommandResultV1::Failed {
            code: "post_install_failed".to_string(),
            message: "failed after one role applied".to_string(),
        },
        &[sample_role_phase_receipt(RolePhaseResultV1::Applied)],
    );

    assert_eq!(status, DeploymentExecutionStatusV1::FailedAfterMutation);
}

#[test]
fn execution_status_classifier_marks_partially_applied_with_applied_and_failed_roles() {
    let role_phase_receipts = vec![
        sample_role_phase_receipt(RolePhaseResultV1::Applied),
        RolePhaseReceiptV1 {
            role: "user_hub".to_string(),
            ..sample_role_phase_receipt(RolePhaseResultV1::Failed)
        },
    ];
    let status = deployment_execution_status_for_receipt_parts(
        &DeploymentCommandResultV1::Failed {
            code: "multi_role_install_failed".to_string(),
            message: "one role applied and another failed".to_string(),
        },
        &role_phase_receipts,
    );

    assert_eq!(status, DeploymentExecutionStatusV1::PartiallyApplied);
}

#[test]
fn deployment_receipt_from_check_derives_partial_status_from_role_receipts() {
    let check = sample_check(sample_plan(), sample_matching_inventory());
    let receipt = deployment_receipt_from_check(
        &check,
        "operation-1",
        "2026-05-22T00:00:00Z",
        Some("2026-05-22T00:00:01Z".to_string()),
        Vec::new(),
        vec![
            sample_role_phase_receipt(RolePhaseResultV1::Applied),
            RolePhaseReceiptV1 {
                role: "user_hub".to_string(),
                ..sample_role_phase_receipt(RolePhaseResultV1::Failed)
            },
        ],
        DeploymentCommandResultV1::Failed {
            code: "partial".to_string(),
            message: "partial execution".to_string(),
        },
    );

    assert_eq!(
        receipt.operation_status,
        DeploymentExecutionStatusV1::PartiallyApplied
    );
}

#[test]
fn artifact_gate_receipt_records_missing_artifact_postcondition() {
    let temp = TempWorkspace::new("canic-host-artifact-gate-receipt");
    let workspace_root = temp.path().join("workspace");
    let icp_root = temp.path().join("icp");
    let config_dir = workspace_root.join("fleets");
    fs::create_dir_all(&config_dir).expect("create config dir");
    fs::write(config_dir.join("canic.toml"), SAMPLE_CONFIG).expect("write config");
    write_artifact(&icp_root, "root", b"root-artifact");

    let check = check_local_deployment(&LocalDeploymentCheckRequest {
        deployment_name: "demo".to_string(),
        network: "local".to_string(),
        workspace_root,
        icp_root,
        config_path: None,
        observed_at: "2026-05-22T00:00:00Z".to_string(),
        runtime_variant: "local".to_string(),
        build_profile: "fast".to_string(),
    })
    .expect("check local deployment");

    let phase = artifact_gate_phase_receipt(&check, "start", Some("finish".to_string()));
    let role_receipts = artifact_gate_role_phase_receipts(&check);

    assert_eq!(
        phase.verified_postcondition.status,
        ObservationStatusV1::Missing
    );
    assert!(
        phase
            .verified_postcondition
            .evidence
            .iter()
            .any(|evidence| evidence == "artifact:user_hub:missing")
    );
    assert!(role_receipts.iter().any(|receipt| {
        receipt.role == "user_hub" && receipt.result == RolePhaseResultV1::Failed
    }));
}
