use super::*;

#[test]
fn deployment_execution_preflight_accepts_safe_plan_and_capable_executor() {
    let check = sample_check(sample_plan(), sample_matching_inventory());
    let authority = build_authority_reconciliation_plan(&check);
    let executor = CurrentCliDeploymentExecutor::new(
        Some("/workspace/canic".to_string()),
        Some("/workspace/canic/.icp".to_string()),
        vec!["/workspace/canic/.icp/local/canisters".to_string()],
    );

    let preflight = deployment_execution_preflight(
        &check.plan,
        &check.report,
        &authority,
        &executor,
        CURRENT_CLI_EXECUTOR_CAPABILITIES,
    );

    assert_eq!(preflight.plan_id, check.plan.plan_id);
    assert_eq!(preflight.safety_report_id, check.report.report_id);
    assert_eq!(preflight.authority_plan_id, authority.plan_id);
    assert_eq!(
        preflight.status,
        DeploymentExecutionPreflightStatusV1::Ready
    );
    assert!(preflight.blockers.is_empty());
    assert!(preflight.missing_capabilities.is_empty());
    assert_eq!(
        preflight.planned_phases,
        vec![
            "resolve_root_canister",
            "build_artifacts",
            "materialize_artifacts",
            "execution_preflight",
            "emit_manifest",
            "install_root",
            "fund_root_pre_bootstrap",
            "stage_release_set",
            "resume_bootstrap",
            "wait_ready",
            "fund_root_post_ready",
            "write_install_state",
        ]
    );
    assert_json_round_trip(&preflight);
}

#[test]
fn deployment_execution_preflight_from_check_derives_authority_plan() {
    let check = sample_check(sample_plan(), sample_matching_inventory());
    let executor = CurrentCliDeploymentExecutor::new(
        Some("/workspace/canic".to_string()),
        Some("/workspace/canic/.icp".to_string()),
        vec!["/workspace/canic/.icp/local/canisters".to_string()],
    );

    let from_check = deployment_execution_preflight_from_check(
        &check,
        &executor,
        CURRENT_CLI_EXECUTOR_CAPABILITIES,
    );
    let authority = build_authority_reconciliation_plan(&check);
    let explicit = deployment_execution_preflight(
        &check.plan,
        &check.report,
        &authority,
        &executor,
        CURRENT_CLI_EXECUTOR_CAPABILITIES,
    );

    assert_eq!(from_check, explicit);
}

#[test]
fn deployment_execution_preflight_validation_accepts_check_derived_artifact() {
    let check = sample_check(sample_plan(), sample_matching_inventory());
    let executor = CurrentCliDeploymentExecutor::new(
        Some("/workspace/canic".to_string()),
        Some("/workspace/canic/.icp".to_string()),
        vec!["/workspace/canic/.icp/local/canisters".to_string()],
    );
    let preflight = deployment_execution_preflight_from_check(
        &check,
        &executor,
        CURRENT_CLI_EXECUTOR_CAPABILITIES,
    );

    validate_deployment_execution_preflight(&preflight).expect("preflight should validate");
    validate_deployment_execution_preflight_for_check(&check, &preflight)
        .expect("preflight should match source check");
}

#[test]
fn deployment_execution_preflight_validation_rejects_mutated_status() {
    let check = sample_unknown_unsafe_check();
    let executor = CurrentCliDeploymentExecutor::new(
        Some("/workspace/canic".to_string()),
        Some("/workspace/canic/.icp".to_string()),
        vec!["/workspace/canic/.icp/local/canisters".to_string()],
    );
    let mut preflight = deployment_execution_preflight_from_check(
        &check,
        &executor,
        CURRENT_CLI_EXECUTOR_CAPABILITIES,
    );
    preflight.status = DeploymentExecutionPreflightStatusV1::Ready;

    let err = validate_deployment_execution_preflight(&preflight)
        .expect_err("ready status with blockers should fail");

    std::assert_matches!(
        err,
        DeploymentExecutionPreflightError::StatusBlockerMismatch { .. }
    );
}

#[test]
fn deployment_execution_preflight_validation_rejects_source_check_mismatch() {
    let check = sample_check(sample_plan(), sample_matching_inventory());
    let executor = CurrentCliDeploymentExecutor::new(
        Some("/workspace/canic".to_string()),
        Some("/workspace/canic/.icp".to_string()),
        vec!["/workspace/canic/.icp/local/canisters".to_string()],
    );
    let mut preflight = deployment_execution_preflight_from_check(
        &check,
        &executor,
        CURRENT_CLI_EXECUTOR_CAPABILITIES,
    );
    preflight.plan_id = "other-plan".to_string();

    let err = validate_deployment_execution_preflight_for_check(&check, &preflight)
        .expect_err("preflight from another plan should fail");

    std::assert_matches!(
        err,
        DeploymentExecutionPreflightError::SourceCheckMismatch {
            field: "plan_id",
            ..
        }
    );
}

#[test]
fn deployment_execution_preflight_validation_rejects_capability_inconsistency() {
    let check = sample_unknown_unsafe_check();
    let executor = LimitedExecutor {
        context: DeploymentExecutionContextV1 {
            workspace_root: Some("/workspace/canic".to_string()),
            icp_root: Some("/workspace/canic/.icp".to_string()),
            artifact_roots: Vec::new(),
            backend: DeploymentExecutorBackendV1::Other {
                name: "limited-test-backend".to_string(),
            },
            backend_capabilities: vec![DeploymentExecutorCapabilityV1::CanisterStatus],
        },
    };
    let mut preflight = deployment_execution_preflight_from_check(
        &check,
        &executor,
        &[DeploymentExecutorCapabilityV1::CanisterStatus],
    );
    preflight
        .missing_capabilities
        .push(DeploymentExecutorCapabilityV1::InstallCode);

    let err = validate_deployment_execution_preflight(&preflight)
        .expect_err("missing non-required capability should fail");

    std::assert_matches!(
        err,
        DeploymentExecutionPreflightError::MissingCapabilityNotRequired {
            capability: DeploymentExecutorCapabilityV1::InstallCode
        }
    );
}

#[test]
fn deployment_execution_preflight_v1_json_schema_shape_is_stable() {
    let check = sample_unknown_unsafe_check();
    let executor = LimitedExecutor {
        context: DeploymentExecutionContextV1 {
            workspace_root: Some("/workspace/canic".to_string()),
            icp_root: Some("/workspace/canic/.icp".to_string()),
            artifact_roots: Vec::new(),
            backend: DeploymentExecutorBackendV1::Other {
                name: "limited-test-backend".to_string(),
            },
            backend_capabilities: vec![DeploymentExecutorCapabilityV1::CanisterStatus],
        },
    };
    let preflight = deployment_execution_preflight_from_check(
        &check,
        &executor,
        &[
            DeploymentExecutorCapabilityV1::CanisterStatus,
            DeploymentExecutorCapabilityV1::StageArtifact,
        ],
    );
    let value = serde_json::to_value(&preflight).expect("encode execution preflight");

    assert_object_keys(
        &value,
        &[
            "schema_version",
            "plan_id",
            "safety_report_id",
            "authority_plan_id",
            "backend",
            "status",
            "planned_phases",
            "required_capabilities",
            "missing_capabilities",
            "blockers",
        ],
    );
    assert_eq!(value["schema_version"], DEPLOYMENT_TRUTH_SCHEMA_VERSION);
    assert_eq!(value["plan_id"], "plan-local-root");
    assert_eq!(value["safety_report_id"], "report-1");
    assert_eq!(value["authority_plan_id"], "plan-local-root");
    assert_eq!(value["backend"]["Other"]["name"], "limited-test-backend");
    assert_eq!(value["status"], "Blocked");
    assert_eq!(value["required_capabilities"][0], "CanisterStatus");
    assert_eq!(value["required_capabilities"][1], "StageArtifact");
    assert_eq!(value["missing_capabilities"][0], "StageArtifact");
    assert_eq!(
        value["blockers"]
            .as_array()
            .expect("blockers should be array")
            .iter()
            .filter(|finding| finding["code"] == "executor_capability_missing")
            .count(),
        1
    );
}

#[test]
fn deployment_execution_preflight_text_reports_passive_readiness() {
    let check = sample_check(sample_plan(), sample_matching_inventory());
    let executor = CurrentCliDeploymentExecutor::new(
        Some("/workspace/canic".to_string()),
        Some("/workspace/canic/.icp".to_string()),
        vec!["/workspace/canic/.icp/local/canisters".to_string()],
    );
    let preflight = deployment_execution_preflight_from_check(
        &check,
        &executor,
        CURRENT_CLI_EXECUTOR_CAPABILITIES,
    );

    let text = deployment_execution_preflight_text(&preflight);

    assert!(text.contains("Deployment execution preflight"));
    assert!(text.contains("mode: passive"));
    assert!(text.contains("status: ready"));
    assert!(text.contains("plan_id: plan-local-root"));
    assert!(text.contains("backend: CurrentCli"));
    assert!(text.contains("planned_phases:"));
    assert!(text.contains("  - install_root"));
    assert!(text.contains("required_capabilities:"));
    assert!(text.contains("  - StageArtifact"));
}

#[test]
fn deployment_execution_preflight_blocks_safety_authority_and_capability_gaps() {
    let mut check = sample_unknown_unsafe_check();
    check.report.status = SafetyStatusV1::Blocked;
    check.report.hard_failures.push(SafetyFindingV1 {
        code: "deployment_artifact_missing".to_string(),
        message: "planned artifact was not observed".to_string(),
        severity: SafetySeverityV1::HardFailure,
        subject: Some("root".to_string()),
    });
    let authority = build_authority_reconciliation_plan(&check);
    let executor = LimitedExecutor {
        context: DeploymentExecutionContextV1 {
            workspace_root: Some("/workspace/canic".to_string()),
            icp_root: Some("/workspace/canic/.icp".to_string()),
            artifact_roots: Vec::new(),
            backend: DeploymentExecutorBackendV1::Other {
                name: "limited-test-backend".to_string(),
            },
            backend_capabilities: vec![DeploymentExecutorCapabilityV1::CanisterStatus],
        },
    };

    let preflight = deployment_execution_preflight(
        &check.plan,
        &check.report,
        &authority,
        &executor,
        &[
            DeploymentExecutorCapabilityV1::CanisterStatus,
            DeploymentExecutorCapabilityV1::StageArtifact,
        ],
    );

    assert_eq!(
        preflight.status,
        DeploymentExecutionPreflightStatusV1::Blocked
    );
    assert_eq!(
        preflight.missing_capabilities,
        vec![DeploymentExecutorCapabilityV1::StageArtifact]
    );
    assert!(preflight.blockers.iter().any(|finding| {
        finding.code == "deployment_safety_blocked"
            && finding.subject.as_deref() == Some("report-1")
    }));
    assert!(
        preflight
            .blockers
            .iter()
            .any(|finding| finding.code == "deployment_artifact_missing")
    );
    assert!(
        preflight
            .blockers
            .iter()
            .any(|finding| finding.code == "authority_unsafe_blocked")
    );
    assert!(preflight.blockers.iter().any(|finding| {
        finding.code == "executor_capability_missing"
            && finding.subject.as_deref() == Some("StageArtifact")
    }));
}

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
            role_assignment_source: Some("local_install_state".to_string()),
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
        receipt.role == "user_hub"
            && receipt.result == RolePhaseResultV1::Failed
            && receipt
                .error
                .as_deref()
                .is_some_and(|error| error.contains("artifact_missing"))
    }));
}

#[test]
fn receipt_aware_diff_marks_verified_phase_resumable() {
    let plan = sample_plan();
    let inventory = sample_matching_inventory();
    let receipt = sample_receipt_with_phase(
        "plan-local-root",
        Some("aaaaa-aa"),
        ObservationStatusV1::Observed,
        RolePhaseResultV1::VerifiedAlreadyApplied,
    );

    let diff = compare_plan_inventory_and_receipt(&plan, &inventory, &receipt);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Safe);
    assert_eq!(
        diff.resumable_phases,
        vec!["materialize_artifacts".to_string()]
    );
    assert_eq!(
        diff.resume_safety.reasons,
        vec!["no blocking deployment truth differences were found".to_string()]
    );
}

#[test]
fn receipt_aware_diff_blocks_plan_mismatch_resume() {
    let plan = sample_plan();
    let inventory = sample_matching_inventory();
    let receipt = sample_receipt_with_phase(
        "old-plan",
        Some("aaaaa-aa"),
        ObservationStatusV1::Observed,
        RolePhaseResultV1::VerifiedAlreadyApplied,
    );

    let diff = compare_plan_inventory_and_receipt(&plan, &inventory, &receipt);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Blocked);
    assert!(diff.resumable_phases.is_empty());
    assert!(
        diff.hard_failures
            .iter()
            .any(|finding| finding.code == "receipt_plan_mismatch")
    );
}

#[test]
fn receipt_aware_diff_does_not_resume_unverified_phase() {
    let plan = sample_plan();
    let inventory = sample_matching_inventory();
    let receipt = sample_receipt_with_phase(
        "plan-local-root",
        Some("aaaaa-aa"),
        ObservationStatusV1::Missing,
        RolePhaseResultV1::Failed,
    );

    let diff = compare_plan_inventory_and_receipt(&plan, &inventory, &receipt);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Blocked);
    assert!(diff.resumable_phases.is_empty());
    assert!(
        diff.hard_failures
            .iter()
            .any(|finding| finding.code == "receipt_postcondition_unverified")
    );
}

#[test]
fn receipt_aware_diff_blocks_execution_status_mismatch() {
    let plan = sample_plan();
    let inventory = sample_matching_inventory();
    let mut receipt = sample_receipt_with_phase(
        "plan-local-root",
        Some("aaaaa-aa"),
        ObservationStatusV1::Observed,
        RolePhaseResultV1::VerifiedAlreadyApplied,
    );
    receipt.command_result = DeploymentCommandResultV1::Failed {
        code: "partial".to_string(),
        message: "role application failed".to_string(),
    };
    receipt.operation_status = DeploymentExecutionStatusV1::PartiallyApplied;
    receipt.role_phase_receipts = vec![
        sample_role_phase_receipt(RolePhaseResultV1::Applied),
        RolePhaseReceiptV1 {
            role: "user_hub".to_string(),
            ..sample_role_phase_receipt(RolePhaseResultV1::NotAttempted)
        },
    ];

    let diff = compare_plan_inventory_and_receipt(&plan, &inventory, &receipt);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Blocked);
    assert!(diff.resumable_phases.is_empty());
    let has_status_mismatch = diff.hard_failures.iter().any(|finding| {
        finding.code == "receipt_execution_status_mismatch"
            && finding.subject.as_deref() == Some("receipt.operation_status")
    });
    assert!(has_status_mismatch);
}

#[test]
fn receipt_aware_diff_blocks_conflicting_duplicate_phase_receipt() {
    let plan = sample_plan();
    let inventory = sample_matching_inventory();
    let mut receipt = sample_receipt_with_phase(
        "plan-local-root",
        Some("aaaaa-aa"),
        ObservationStatusV1::Observed,
        RolePhaseResultV1::VerifiedAlreadyApplied,
    );
    let mut conflicting = receipt.phase_receipts[0].clone();
    conflicting.verified_postcondition.status = ObservationStatusV1::Missing;
    receipt.phase_receipts.push(conflicting);

    let diff = compare_plan_inventory_and_receipt(&plan, &inventory, &receipt);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Blocked);
    assert!(diff.resumable_phases.is_empty());
    assert!(
        diff.hard_failures
            .iter()
            .any(|finding| finding.code == "receipt_phase_conflict"
                && finding.subject.as_deref() == Some("materialize_artifacts"))
    );
}

#[test]
fn receipt_aware_diff_warns_for_duplicate_identical_phase_receipt() {
    let plan = sample_plan();
    let inventory = sample_matching_inventory();
    let mut receipt = sample_receipt_with_phase(
        "plan-local-root",
        Some("aaaaa-aa"),
        ObservationStatusV1::Observed,
        RolePhaseResultV1::VerifiedAlreadyApplied,
    );
    receipt
        .phase_receipts
        .push(receipt.phase_receipts[0].clone());

    let diff = compare_plan_inventory_and_receipt(&plan, &inventory, &receipt);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Warning);
    assert_eq!(
        diff.resumable_phases,
        vec!["materialize_artifacts".to_string()]
    );
    assert!(
        diff.warnings
            .iter()
            .any(|finding| finding.code == "duplicate_receipt_phase"
                && finding.subject.as_deref() == Some("materialize_artifacts"))
    );
}

#[test]
fn receipt_aware_diff_blocks_conflicting_duplicate_role_phase_receipt() {
    let plan = sample_plan();
    let inventory = sample_matching_inventory();
    let mut receipt = sample_receipt_with_phase(
        "plan-local-root",
        Some("aaaaa-aa"),
        ObservationStatusV1::Observed,
        RolePhaseResultV1::VerifiedAlreadyApplied,
    );
    receipt.role_phase_receipts[0].phase = "materialize_artifacts".to_string();
    let mut conflicting = receipt.role_phase_receipts[0].clone();
    conflicting.result = RolePhaseResultV1::Failed;
    conflicting.error = Some("artifact_missing".to_string());
    receipt.role_phase_receipts.push(conflicting);

    let diff = compare_plan_inventory_and_receipt(&plan, &inventory, &receipt);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Blocked);
    assert!(diff.resumable_phases.is_empty());
    assert!(
        diff.hard_failures
            .iter()
            .any(|finding| finding.code == "receipt_role_phase_conflict"
                && finding.subject.as_deref() == Some("root:materialize_artifacts"))
    );
}

#[test]
fn receipt_aware_diff_warns_for_duplicate_identical_role_phase_receipt() {
    let plan = sample_plan();
    let inventory = sample_matching_inventory();
    let mut receipt = sample_receipt_with_phase(
        "plan-local-root",
        Some("aaaaa-aa"),
        ObservationStatusV1::Observed,
        RolePhaseResultV1::VerifiedAlreadyApplied,
    );
    receipt
        .role_phase_receipts
        .push(receipt.role_phase_receipts[0].clone());

    let diff = compare_plan_inventory_and_receipt(&plan, &inventory, &receipt);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Warning);
    assert_eq!(
        diff.resumable_phases,
        vec!["materialize_artifacts".to_string()]
    );
    assert!(
        diff.warnings
            .iter()
            .any(|finding| finding.code == "duplicate_receipt_role_phase"
                && finding.subject.as_deref() == Some("root:materialize_artifacts"))
    );
}
