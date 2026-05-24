use super::*;
use crate::deployment_truth::observe::{
    apply_canister_control_to_observed_pool, apply_live_status_to_registry_observation,
    observed_root_from_status, registry_entries_to_observed_canisters,
    registry_entries_to_observed_pool,
};
use crate::icp::{IcpCanisterStatusReport, IcpCanisterStatusSettings};
use crate::install_root::InstallState;
use crate::registry::RegistryEntry;
use crate::release_set::{ConfiguredPoolExpectation, ROOT_RELEASE_SET_MANIFEST_FILE};
use crate::test_support::temp_dir;
use serde::Serialize;
use std::{fs, path::Path};

struct LimitedExecutor {
    context: DeploymentExecutionContextV1,
}

impl DeploymentExecutor for LimitedExecutor {
    fn execution_context(&self) -> DeploymentExecutionContextV1 {
        self.context.clone()
    }
}

#[test]
fn plan_round_trips_through_json() {
    let plan = DeploymentPlanV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        plan_id: "plan-local-root".to_string(),
        deployment_identity: sample_identity(),
        trust_domain: TrustDomainV1 {
            root_trust_anchor: Some("aaaaa-aa".to_string()),
            migration_from: None,
        },
        fleet_template: "root".to_string(),
        runtime_variant: "local".to_string(),
        authority_profile: AuthorityProfileV1 {
            profile_id: "local-default".to_string(),
            expected_controllers: vec!["aaaaa-aa".to_string()],
            staging_controllers: Vec::new(),
            emergency_controllers: Vec::new(),
        },
        role_artifacts: vec![sample_role_artifact()],
        expected_canisters: vec![ExpectedCanisterV1 {
            role: "root".to_string(),
            canister_id: Some("aaaaa-aa".to_string()),
            control_class: CanisterControlClassV1::DeploymentControlled,
        }],
        expected_pool: Vec::new(),
        expected_verifier_readiness: VerifierReadinessExpectationV1 {
            required: true,
            expected_role_epochs: vec![RoleEpochExpectationV1 {
                role: "root".to_string(),
                minimum_epoch: 1,
            }],
        },
        unresolved_assumptions: Vec::new(),
    };

    let encoded = serde_json::to_string(&plan).expect("plan should encode");
    let decoded = serde_json::from_str::<DeploymentPlanV1>(&encoded).expect("plan should decode");

    assert_eq!(decoded, plan);
}

#[test]
fn inventory_round_trips_through_json() {
    let inventory = DeploymentInventoryV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        inventory_id: "inventory-1".to_string(),
        observed_at: "2026-05-21T00:00:00Z".to_string(),
        observed_identity: Some(sample_identity()),
        local_config: LocalDeploymentConfigV1 {
            config_path: Some("icp.yml".to_string()),
            raw_config_sha256: Some("raw".to_string()),
            canonical_embedded_config_sha256: Some("canonical".to_string()),
        },
        observed_canisters: vec![ObservedCanisterV1 {
            canister_id: "aaaaa-aa".to_string(),
            role: Some("root".to_string()),
            control_class: CanisterControlClassV1::DeploymentControlled,
            controllers: vec!["aaaaa-aa".to_string()],
            module_hash: Some("module".to_string()),
            status: Some("running".to_string()),
            root_trust_anchor: Some("aaaaa-aa".to_string()),
            canonical_embedded_config_digest: Some("canonical".to_string()),
            role_assignment_source: Some("registry".to_string()),
        }],
        observed_pool: Vec::new(),
        observed_artifacts: vec![ObservedArtifactV1 {
            role: "root".to_string(),
            artifact_path: ".icp/local/canisters/root/root.wasm.gz".to_string(),
            file_sha256: Some("artifact-file".to_string()),
            file_sha256_source: Some(ArtifactDigestSourceV1::ObservedFileDigest),
            payload_sha256: Some("artifact".to_string()),
            payload_size_bytes: Some(42),
            source: ArtifactSourceV1::LocalBuild,
        }],
        observed_verifier_readiness: VerifierReadinessObservationV1 {
            status: ObservationStatusV1::Observed,
            role_epochs: vec![RoleEpochObservationV1 {
                role: "root".to_string(),
                observed_epoch: Some(1),
                status: ObservationStatusV1::Observed,
            }],
        },
        unresolved_observations: Vec::new(),
    };

    let encoded = serde_json::to_string_pretty(&inventory).expect("inventory should encode");
    let decoded =
        serde_json::from_str::<DeploymentInventoryV1>(&encoded).expect("inventory should decode");

    assert_eq!(decoded, inventory);
}

#[test]
fn receipt_diff_and_safety_report_support_not_evaluated_state() {
    let receipt = DeploymentReceiptV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        operation_id: "operation-1".to_string(),
        plan_id: "plan-local-root".to_string(),
        execution_context: None,
        operation_status: DeploymentExecutionStatusV1::InProgress,
        started_at: "2026-05-21T00:00:00Z".to_string(),
        finished_at: None,
        operator_principal: None,
        root_principal: Some("aaaaa-aa".to_string()),
        previous_observed_deployment_epoch: None,
        phase_receipts: vec![PhaseReceiptV1 {
            phase: "build_artifacts".to_string(),
            started_at: "2026-05-21T00:00:00Z".to_string(),
            finished_at: None,
            attempted_action: "build root artifact".to_string(),
            verified_postcondition: VerifiedPostconditionV1 {
                status: ObservationStatusV1::NotObserved,
                evidence: Vec::new(),
            },
        }],
        role_phase_receipts: vec![RolePhaseReceiptV1 {
            role: "root".to_string(),
            phase: "install_root".to_string(),
            result: RolePhaseResultV1::NotAttempted,
            previous_module_hash: None,
            target_module_hash: Some("module".to_string()),
            observed_module_hash_after: None,
            artifact_digest: Some("artifact".to_string()),
            canonical_embedded_config_sha256: None,
            error: None,
        }],
        final_inventory_id: None,
        command_result: DeploymentCommandResultV1::NotFinished,
    };
    let diff = DeploymentDiffV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        plan_identity: sample_identity(),
        observed_identity: None,
        artifact_diff: Vec::new(),
        controller_diff: Vec::new(),
        pool_diff: Vec::new(),
        embedded_config_diff: Vec::new(),
        module_hash_diff: Vec::new(),
        verifier_readiness_diff: Vec::new(),
        resume_safety: ResumeSafetyV1 {
            status: SafetyStatusV1::NotEvaluated,
            reasons: vec!["inventory not collected".to_string()],
        },
        hard_failures: Vec::new(),
        warnings: Vec::new(),
        resumable_phases: Vec::new(),
    };
    let report = SafetyReportV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        report_id: "report-1".to_string(),
        diff_id: None,
        status: SafetyStatusV1::NotEvaluated,
        summary: "deployment safety has not been evaluated".to_string(),
        hard_failures: Vec::new(),
        warnings: Vec::new(),
        next_actions: vec!["collect deployment inventory".to_string()],
    };

    assert_json_round_trip(&receipt);
    assert_json_round_trip(&diff);
    assert_json_round_trip(&report);
}

#[test]
fn current_cli_execution_context_records_backend_roots_and_capabilities() {
    let context = current_cli_execution_context(
        Some("/workspace/canic".to_string()),
        Some("/workspace/canic/.icp".to_string()),
        vec![
            "/workspace/canic/.icp/local/canisters".to_string(),
            "/workspace/canic/target/wasm".to_string(),
        ],
    );

    assert_eq!(context.backend, DeploymentExecutorBackendV1::CurrentCli);
    assert!(has_executor_capabilities(
        &context.backend_capabilities,
        CURRENT_CLI_EXECUTOR_CAPABILITIES,
    ));
    assert_json_round_trip(&context);
}

#[test]
fn current_cli_executor_returns_declared_execution_context() {
    let executor = CurrentCliDeploymentExecutor::new(
        Some("/workspace/canic".to_string()),
        Some("/workspace/canic/.icp".to_string()),
        vec!["/workspace/canic/.icp/local/canisters".to_string()],
    );

    let context = executor.execution_context();

    assert_eq!(context.backend, DeploymentExecutorBackendV1::CurrentCli);
    assert_eq!(context.workspace_root.as_deref(), Some("/workspace/canic"));
    assert_eq!(context.icp_root.as_deref(), Some("/workspace/canic/.icp"));
    assert_eq!(
        context.artifact_roots,
        vec!["/workspace/canic/.icp/local/canisters".to_string()]
    );
    assert!(has_executor_capabilities(
        &context.backend_capabilities,
        CURRENT_CLI_EXECUTOR_CAPABILITIES,
    ));
}

#[test]
fn testkit_preflight_context_has_no_local_workspace_roots() {
    let context = testkit_execution_context(vec!["memory://pocket-ic/artifacts".to_string()]);

    assert_eq!(context.backend, DeploymentExecutorBackendV1::PocketIc);
    assert_eq!(context.workspace_root, None);
    assert_eq!(context.icp_root, None);
    assert_eq!(
        context.artifact_roots,
        vec!["memory://pocket-ic/artifacts".to_string()]
    );
    assert_eq!(context.backend_capabilities, TESTKIT_PREFLIGHT_CAPABILITIES);
}

#[test]
fn missing_executor_capabilities_are_reported_in_required_order() {
    let available = [
        DeploymentExecutorCapabilityV1::CanisterStatus,
        DeploymentExecutorCapabilityV1::StageArtifact,
    ];
    let required = [
        DeploymentExecutorCapabilityV1::StageArtifact,
        DeploymentExecutorCapabilityV1::InstallCode,
        DeploymentExecutorCapabilityV1::CanisterStatus,
        DeploymentExecutorCapabilityV1::UpdateSettings,
    ];

    assert_eq!(
        missing_executor_capabilities(&available, &required),
        vec![
            DeploymentExecutorCapabilityV1::InstallCode,
            DeploymentExecutorCapabilityV1::UpdateSettings,
        ],
    );
}

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
            "create_root",
            "build_artifacts",
            "materialize_artifacts",
            "install_root",
            "stage_release_set",
            "resume_bootstrap",
            "wait_ready",
            "post_validate",
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
fn testkit_preflight_validates_same_plan_shape_as_current_cli() {
    let check = sample_check(sample_plan(), sample_matching_inventory());
    let current_cli = CurrentCliDeploymentExecutor::new(
        Some("/workspace/canic".to_string()),
        Some("/workspace/canic/.icp".to_string()),
        vec!["/workspace/canic/.icp/local/canisters".to_string()],
    );
    let pocket_ic = TestkitPreflightContext::new(vec!["memory://pocket-ic/artifacts".to_string()]);

    let current_cli_preflight = deployment_execution_preflight_from_check(
        &check,
        &current_cli,
        CURRENT_CLI_EXECUTOR_CAPABILITIES,
    );
    let pocket_ic_preflight = deployment_execution_preflight_from_check(
        &check,
        &pocket_ic,
        TESTKIT_PREFLIGHT_CAPABILITIES,
    );

    validate_deployment_execution_preflight_for_check(&check, &current_cli_preflight)
        .expect("current CLI preflight should validate against source check");
    validate_deployment_execution_preflight_for_check(&check, &pocket_ic_preflight)
        .expect("PocketIC preflight should validate against source check");
    assert_eq!(current_cli_preflight.plan_id, pocket_ic_preflight.plan_id);
    assert_eq!(
        current_cli_preflight.safety_report_id,
        pocket_ic_preflight.safety_report_id
    );
    assert_eq!(
        current_cli_preflight.authority_plan_id,
        pocket_ic_preflight.authority_plan_id
    );
    assert_eq!(
        current_cli_preflight.planned_phases,
        pocket_ic_preflight.planned_phases
    );
    assert_eq!(
        pocket_ic_preflight.backend,
        DeploymentExecutorBackendV1::PocketIc
    );
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

    assert!(matches!(
        err,
        DeploymentExecutionPreflightError::StatusBlockerMismatch { .. }
    ));
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

    assert!(matches!(
        err,
        DeploymentExecutionPreflightError::SourceCheckMismatch {
            field: "plan_id",
            ..
        }
    ));
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

    assert!(matches!(
        err,
        DeploymentExecutionPreflightError::MissingCapabilityNotRequired {
            capability: DeploymentExecutorCapabilityV1::InstallCode
        }
    ));
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
fn staging_receipt_v1_json_schema_shape_is_stable() {
    let receipt = StagingReceiptV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        role: "user_hub".to_string(),
        artifact_identity: "embedded:user_hub:0.43.4:abc123".to_string(),
        transport: ArtifactTransportV1::WasmStore,
        wasm_store_locator: Some("root:aaaaa-aa:bootstrap".to_string()),
        prepared_chunk_hashes: vec!["chunk-a".to_string(), "chunk-b".to_string()],
        published_chunk_count: 2,
        verified_postcondition: VerifiedPostconditionV1 {
            status: ObservationStatusV1::Observed,
            evidence: vec!["payload_sha256:abc123".to_string()],
        },
    };
    let value = serde_json::to_value(&receipt).expect("encode staging receipt");

    assert_object_keys(
        &value,
        &[
            "schema_version",
            "role",
            "artifact_identity",
            "transport",
            "wasm_store_locator",
            "prepared_chunk_hashes",
            "published_chunk_count",
            "verified_postcondition",
        ],
    );
    assert_eq!(value["schema_version"], DEPLOYMENT_TRUTH_SCHEMA_VERSION);
    assert_eq!(value["role"], "user_hub");
    assert_eq!(value["transport"], "WasmStore");
    assert_eq!(value["prepared_chunk_hashes"][1], "chunk-b");
    assert_eq!(value["published_chunk_count"], 2);
    assert_eq!(value["verified_postcondition"]["status"], "Observed");
}

#[test]
fn staging_receipt_evidence_preserves_transport_and_chunk_facts() {
    let receipts = vec![StagingReceiptV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        role: "scale_hub".to_string(),
        artifact_identity: "embedded:scale_hub:0.43.4:def456".to_string(),
        transport: ArtifactTransportV1::WasmStore,
        wasm_store_locator: Some("root:aaaaa-aa:bootstrap".to_string()),
        prepared_chunk_hashes: vec!["chunk-a".to_string()],
        published_chunk_count: 1,
        verified_postcondition: VerifiedPostconditionV1 {
            status: ObservationStatusV1::Observed,
            evidence: Vec::new(),
        },
    }];

    let evidence = staging_receipt_evidence(&receipts);

    assert!(evidence.contains(&"staging_receipts:1".to_string()));
    assert!(evidence.contains(&"staging_role:scale_hub".to_string()));
    assert!(evidence.contains(&"staging_transport:WasmStore".to_string()));
    assert!(evidence.contains(&"staging_chunks_prepared:1".to_string()));
    assert!(evidence.contains(&"staging_chunks_published:1".to_string()));
    assert!(evidence.contains(&"staging_postcondition:Observed".to_string()));
    assert!(evidence.contains(&"staging_wasm_store:root:aaaaa-aa:bootstrap".to_string()));
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

#[test]
fn local_check_builds_plan_inventory_diff_and_report() {
    let temp = TempWorkspace::new("canic-host-local-check");
    let workspace_root = temp.path().join("workspace");
    let icp_root = temp.path().join("icp");
    let config_dir = workspace_root.join("fleets");
    fs::create_dir_all(&config_dir).expect("create config dir");
    fs::write(config_dir.join("canic.toml"), SAMPLE_CONFIG).expect("write config");
    write_artifact(&icp_root, "root", b"root-artifact");
    write_release_set_manifest(&icp_root);

    let check = check_local_deployment(&LocalDeploymentCheckRequest {
        deployment_name: "demo".to_string(),
        network: "local".to_string(),
        workspace_root,
        icp_root,
        config_path: None,
        observed_at: "2026-05-21T00:00:00Z".to_string(),
        runtime_variant: "local".to_string(),
        build_profile: "fast".to_string(),
    })
    .expect("check local deployment");

    assert_eq!(check.schema_version, DEPLOYMENT_TRUTH_SCHEMA_VERSION);
    assert_eq!(check.check_id, "local:local:demo:check");
    assert_eq!(check.plan.plan_id, "local:local:demo:plan");
    assert_eq!(check.inventory.inventory_id, "local:local:demo");
    assert_eq!(check.diff.resume_safety.status, check.report.status);
    assert!(
        check
            .diff
            .hard_failures
            .iter()
            .any(|finding| finding.code == "artifact_missing")
    );
    assert_eq!(check.report.status, SafetyStatusV1::Blocked);
}

#[test]
fn local_inventory_collects_configured_roles_and_artifacts_without_live_queries() {
    let temp = TempWorkspace::new("canic-host-local-inventory");
    let workspace_root = temp.path().join("workspace");
    let icp_root = temp.path().join("icp");
    let config_dir = workspace_root.join("fleets");
    fs::create_dir_all(&config_dir).expect("create config dir");
    fs::write(config_dir.join("canic.toml"), SAMPLE_CONFIG).expect("write config");

    let artifact_path = icp_root
        .join(".icp")
        .join("local")
        .join("canisters")
        .join("root")
        .join("root.wasm.gz");
    fs::create_dir_all(artifact_path.parent().expect("artifact parent"))
        .expect("create artifact dir");
    fs::write(&artifact_path, b"artifact").expect("write artifact");
    write_release_set_manifest(&icp_root);

    let inventory = collect_local_deployment_inventory(&LocalInventoryRequest {
        deployment_name: "demo".to_string(),
        network: "local".to_string(),
        workspace_root,
        icp_root,
        config_path: None,
        observed_at: "2026-05-21T00:00:00Z".to_string(),
    })
    .expect("collect inventory");

    assert_eq!(inventory.schema_version, DEPLOYMENT_TRUTH_SCHEMA_VERSION);
    assert_eq!(inventory.inventory_id, "local:local:demo");
    assert_sha256_len(inventory.local_config.raw_config_sha256.as_ref());
    assert_sha256_len(
        inventory
            .local_config
            .canonical_embedded_config_sha256
            .as_ref(),
    );
    let observed_identity = inventory.observed_identity.as_ref().expect("identity");
    assert_sha256_len(observed_identity.deployment_manifest_digest.as_ref());
    assert_sha256_len(observed_identity.canonical_runtime_config_digest.as_ref());
    assert_sha256_len(observed_identity.role_topology_hash.as_ref());
    assert_sha256_len(observed_identity.artifact_set_digest.as_ref());
    assert_sha256_len(observed_identity.pool_identity_set_digest.as_ref());
    assert_eq!(inventory.observed_artifacts.len(), 1);
    assert_eq!(inventory.observed_artifacts[0].role, "root");
    assert_eq!(inventory.observed_artifacts[0].payload_size_bytes, Some(8));
    assert_eq!(
        inventory.observed_artifacts[0].file_sha256_source,
        Some(ArtifactDigestSourceV1::ObservedFileDigest)
    );
    assert_sha256_len(inventory.observed_artifacts[0].file_sha256.as_ref());
    assert!(
        inventory
            .unresolved_observations
            .iter()
            .any(|gap| gap.key == "local_artifacts.user_hub")
    );
}

#[test]
fn live_root_status_observation_maps_status_controllers_and_module_hash() {
    let state = sample_install_state("aaaaa-aa");
    let report = IcpCanisterStatusReport {
        id: "aaaaa-aa".to_string(),
        name: Some("root".to_string()),
        status: "Running".to_string(),
        settings: Some(IcpCanisterStatusSettings {
            controllers: vec!["aaaaa-aa".to_string()],
            compute_allocation: Some("0".to_string()),
            memory_allocation: None,
            freezing_threshold: None,
            reserved_cycles_limit: None,
            wasm_memory_limit: None,
            wasm_memory_threshold: None,
            log_memory_limit: None,
        }),
        module_hash: Some("0xABCD".to_string()),
        memory_size: None,
        cycles: None,
        reserved_cycles: None,
        idle_cycles_burned_per_day: None,
    };

    let observed = observed_root_from_status(&state, &report);

    assert_eq!(observed.canister_id, "aaaaa-aa");
    assert_eq!(
        observed.control_class,
        CanisterControlClassV1::DeploymentControlled
    );
    assert_eq!(observed.controllers, vec!["aaaaa-aa"]);
    assert_eq!(observed.module_hash.as_deref(), Some("abcd"));
    assert_eq!(observed.status.as_deref(), Some("Running"));
    assert_eq!(
        observed.role_assignment_source.as_deref(),
        Some("icp_canister_status")
    );
}

#[test]
fn registry_entries_map_configured_pool_roles_to_observed_pool() {
    let mut gaps = Vec::new();
    let entries = vec![
        RegistryEntry {
            pid: "root-id".to_string(),
            role: Some("root".to_string()),
            kind: None,
            parent_pid: None,
            module_hash: None,
        },
        RegistryEntry {
            pid: "shard-id".to_string(),
            role: Some("user_shard".to_string()),
            kind: None,
            parent_pid: Some("user_hub-id".to_string()),
            module_hash: Some("module".to_string()),
        },
        RegistryEntry {
            pid: "user_hub-id".to_string(),
            role: Some("user_hub".to_string()),
            kind: None,
            parent_pid: Some("root-id".to_string()),
            module_hash: None,
        },
    ];
    let expectations = vec![ConfiguredPoolExpectation {
        pool: "user_shards".to_string(),
        canister_role: "user_shard".to_string(),
    }];

    let observed = registry_entries_to_observed_pool("root-id", &entries, &expectations, &mut gaps);

    assert_eq!(
        observed,
        vec![ObservedPoolCanisterV1 {
            pool: "user_shards".to_string(),
            canister_id: "shard-id".to_string(),
            role: Some("user_shard".to_string()),
            control_class: CanisterControlClassV1::CanicManagedPool,
        }]
    );
    assert!(gaps.is_empty());
}

#[test]
fn registry_entries_map_roles_to_observed_canisters_without_controller_authority() {
    let entries = vec![
        RegistryEntry {
            pid: "root-id".to_string(),
            role: Some("root".to_string()),
            kind: None,
            parent_pid: None,
            module_hash: None,
        },
        RegistryEntry {
            pid: "user_hub-id".to_string(),
            role: Some("user_hub".to_string()),
            kind: None,
            parent_pid: Some("root-id".to_string()),
            module_hash: Some("0xABCDEF".to_string()),
        },
    ];

    let observed = registry_entries_to_observed_canisters("root-id", &entries);

    assert_eq!(observed.len(), 1);
    assert_eq!(observed[0].canister_id, "user_hub-id");
    assert_eq!(observed[0].role.as_deref(), Some("user_hub"));
    assert_eq!(
        observed[0].control_class,
        CanisterControlClassV1::CanicManagedPool
    );
    assert!(observed[0].controllers.is_empty());
    assert_eq!(observed[0].module_hash.as_deref(), Some("abcdef"));
    assert_eq!(
        observed[0].role_assignment_source.as_deref(),
        Some("subnet_registry")
    );
}

#[test]
fn registry_observation_can_be_enriched_with_live_status() {
    let mut observed = registry_entries_to_observed_canisters(
        "root-id",
        &[RegistryEntry {
            pid: "user_hub-id".to_string(),
            role: Some("user_hub".to_string()),
            kind: None,
            parent_pid: Some("root-id".to_string()),
            module_hash: Some("stale".to_string()),
        }],
    )
    .pop()
    .expect("registry observation");
    let report = IcpCanisterStatusReport {
        id: "user_hub-id".to_string(),
        name: Some("user_hub".to_string()),
        status: "Running".to_string(),
        settings: Some(IcpCanisterStatusSettings {
            controllers: vec!["root-id".to_string()],
            compute_allocation: Some("0".to_string()),
            memory_allocation: None,
            freezing_threshold: None,
            reserved_cycles_limit: None,
            wasm_memory_limit: None,
            wasm_memory_threshold: None,
            log_memory_limit: None,
        }),
        module_hash: Some("0xCAFE".to_string()),
        memory_size: None,
        cycles: None,
        reserved_cycles: None,
        idle_cycles_burned_per_day: None,
    };

    apply_live_status_to_registry_observation(&mut observed, &report);

    assert_eq!(
        observed.control_class,
        CanisterControlClassV1::CanicManagedPool
    );
    assert_eq!(observed.controllers, vec!["root-id"]);
    assert_eq!(observed.module_hash.as_deref(), Some("cafe"));
    assert_eq!(observed.status.as_deref(), Some("Running"));
    assert_eq!(
        observed.role_assignment_source.as_deref(),
        Some("subnet_registry+icp_canister_status")
    );
}

#[test]
fn observed_pool_control_uses_enriched_canister_status() {
    let mut observed_pool = vec![ObservedPoolCanisterV1 {
        pool: "user_shards".to_string(),
        canister_id: "shard-id".to_string(),
        role: Some("user_shard".to_string()),
        control_class: CanisterControlClassV1::CanicManagedPool,
    }];
    let observed_canisters = vec![ObservedCanisterV1 {
        canister_id: "shard-id".to_string(),
        role: Some("user_shard".to_string()),
        control_class: CanisterControlClassV1::UnknownUnsafe,
        controllers: vec!["external-controller".to_string()],
        module_hash: Some("module".to_string()),
        status: Some("Running".to_string()),
        root_trust_anchor: Some("root-id".to_string()),
        canonical_embedded_config_digest: None,
        role_assignment_source: Some("subnet_registry+icp_canister_status".to_string()),
    }];

    apply_canister_control_to_observed_pool(&mut observed_pool, &observed_canisters);

    assert_eq!(
        observed_pool[0].control_class,
        CanisterControlClassV1::UnknownUnsafe
    );
}

#[test]
fn registry_entries_report_ambiguous_pool_role_mapping() {
    let mut gaps = Vec::new();
    let entries = vec![RegistryEntry {
        pid: "worker-id".to_string(),
        role: Some("worker".to_string()),
        kind: None,
        parent_pid: Some("root-id".to_string()),
        module_hash: None,
    }];
    let expectations = vec![
        ConfiguredPoolExpectation {
            pool: "workers_a".to_string(),
            canister_role: "worker".to_string(),
        },
        ConfiguredPoolExpectation {
            pool: "workers_b".to_string(),
            canister_role: "worker".to_string(),
        },
    ];

    let observed = registry_entries_to_observed_pool("root-id", &entries, &expectations, &mut gaps);

    assert!(observed.is_empty());
    assert!(
        gaps.iter()
            .any(|gap| gap.key == "live_subnet_registry.pool.worker")
    );
}

#[test]
fn local_inventory_reports_missing_config_as_observation_gap() {
    let temp = TempWorkspace::new("canic-host-local-inventory-missing-config");

    let inventory = collect_local_deployment_inventory(&LocalInventoryRequest {
        deployment_name: "demo".to_string(),
        network: "local".to_string(),
        workspace_root: temp.path().join("workspace"),
        icp_root: temp.path().join("icp"),
        config_path: None,
        observed_at: "2026-05-21T00:00:00Z".to_string(),
    })
    .expect("collect inventory");

    assert_eq!(inventory.inventory_id, "local:local:demo");
    assert!(
        inventory
            .unresolved_observations
            .iter()
            .any(|gap| gap.key == "local_config.fleet_name")
    );
    assert!(
        inventory
            .unresolved_observations
            .iter()
            .any(|gap| gap.key == "local_config.roles")
    );
}

#[test]
fn local_artifact_manifest_collects_roles_and_release_set_hashes() {
    let temp = TempWorkspace::new("canic-host-local-artifact-manifest");
    let workspace_root = temp.path().join("workspace");
    let icp_root = temp.path().join("icp");
    let config_dir = workspace_root.join("fleets");
    fs::create_dir_all(&config_dir).expect("create config dir");
    fs::write(config_dir.join("canic.toml"), SAMPLE_CONFIG).expect("write config");
    write_artifact(&icp_root, "root", b"root-artifact");
    write_artifact(&icp_root, "wasm_store", b"wasm-store-artifact");
    write_artifact(&icp_root, "user_hub", b"user-hub-artifact");
    write_release_set_manifest(&icp_root);

    let manifest = collect_local_role_artifact_manifest(&LocalArtifactManifestRequest {
        network: "local".to_string(),
        workspace_root,
        icp_root,
        config_path: None,
    });

    assert_eq!(manifest.manifest_id, "local:local:demo:artifacts");
    assert_eq!(manifest.role_artifacts.len(), 3);
    let wasm_store = manifest
        .role_artifacts
        .iter()
        .find(|artifact| artifact.role == "wasm_store")
        .expect("wasm_store artifact");
    assert_eq!(wasm_store.source, ArtifactSourceV1::WasmStore);
    assert_eq!(
        wasm_store.observed_wasm_gz_file_sha256_source,
        Some(ArtifactDigestSourceV1::ObservedFileDigest)
    );
    let user_hub = manifest
        .role_artifacts
        .iter()
        .find(|artifact| artifact.role == "user_hub")
        .expect("user_hub artifact");
    assert_eq!(user_hub.wasm_gz_sha256.as_deref(), Some("user-hub-hash"));
    assert_eq!(
        user_hub.wasm_gz_sha256_source,
        Some(ArtifactDigestSourceV1::ReleaseSetManifest)
    );
    assert_eq!(user_hub.wasm_gz_size_bytes, Some(17));
    assert_eq!(
        user_hub.observed_wasm_gz_file_sha256_source,
        Some(ArtifactDigestSourceV1::ObservedFileDigest)
    );
    assert_eq!(
        user_hub
            .observed_wasm_gz_file_sha256
            .as_ref()
            .map(String::len),
        Some(64)
    );
    let root = manifest
        .role_artifacts
        .iter()
        .find(|artifact| artifact.role == "root")
        .expect("root artifact");
    assert_eq!(root.wasm_gz_sha256, None);
    assert_eq!(root.wasm_gz_sha256_source, None);
    assert!(manifest.unresolved_artifacts.is_empty());
}

#[test]
fn local_artifact_manifest_reports_network_artifact_fallback() {
    let temp = TempWorkspace::new("canic-host-local-artifact-manifest-fallback");
    let workspace_root = temp.path().join("workspace");
    let icp_root = temp.path().join("icp");
    let config_dir = workspace_root.join("fleets");
    fs::create_dir_all(&config_dir).expect("create config dir");
    fs::write(config_dir.join("canic.toml"), SAMPLE_CONFIG).expect("write config");
    write_artifact(&icp_root, "root", b"root-artifact");

    let manifest = collect_local_role_artifact_manifest(&LocalArtifactManifestRequest {
        network: "ic".to_string(),
        workspace_root,
        icp_root,
        config_path: None,
    });

    assert!(
        manifest
            .unresolved_artifacts
            .iter()
            .any(|gap| gap.key == "local_artifacts.network_fallback")
    );
}

#[test]
fn local_artifact_manifest_records_missing_artifacts_as_gaps() {
    let temp = TempWorkspace::new("canic-host-local-artifact-manifest-missing");
    let workspace_root = temp.path().join("workspace");
    let icp_root = temp.path().join("icp");
    let config_dir = workspace_root.join("fleets");
    fs::create_dir_all(&config_dir).expect("create config dir");
    fs::write(config_dir.join("canic.toml"), SAMPLE_CONFIG).expect("write config");
    write_artifact(&icp_root, "root", b"root-artifact");

    let manifest = collect_local_role_artifact_manifest(&LocalArtifactManifestRequest {
        network: "local".to_string(),
        workspace_root,
        icp_root,
        config_path: None,
    });

    assert!(
        manifest
            .unresolved_artifacts
            .iter()
            .any(|gap| gap.key == "local_artifacts.release_set_manifest")
    );
    assert!(
        manifest
            .unresolved_artifacts
            .iter()
            .any(|gap| gap.key == "local_artifacts.user_hub")
    );
    assert!(
        manifest
            .unresolved_artifacts
            .iter()
            .any(|gap| gap.key == "local_artifacts.wasm_store")
    );
}

#[test]
fn local_plan_uses_configured_roles_and_local_artifact_manifest() {
    let temp = TempWorkspace::new("canic-host-local-plan");
    let workspace_root = temp.path().join("workspace");
    let icp_root = temp.path().join("icp");
    let config_dir = workspace_root.join("fleets");
    fs::create_dir_all(&config_dir).expect("create config dir");
    fs::write(config_dir.join("canic.toml"), SAMPLE_CONFIG).expect("write config");
    write_artifact(&icp_root, "root", b"root-artifact");
    write_artifact(&icp_root, "wasm_store", b"wasm-store-artifact");
    write_artifact(&icp_root, "user_hub", b"user-hub-artifact");
    write_release_set_manifest(&icp_root);

    let plan = build_local_deployment_plan(&LocalDeploymentPlanRequest {
        deployment_name: "demo-local".to_string(),
        network: "local".to_string(),
        workspace_root,
        icp_root,
        config_path: None,
        runtime_variant: "local".to_string(),
        build_profile: "fast".to_string(),
    });

    assert_eq!(plan.plan_id, "local:local:demo-local:plan");
    assert_eq!(plan.deployment_identity.deployment_name, "demo-local");
    assert_eq!(
        plan.deployment_identity
            .deployment_manifest_digest
            .as_ref()
            .map(String::len),
        Some(64)
    );
    assert_eq!(
        plan.deployment_identity
            .canonical_runtime_config_digest
            .as_ref()
            .map(String::len),
        Some(64)
    );
    assert_eq!(
        plan.deployment_identity
            .authority_profile_hash
            .as_ref()
            .map(String::len),
        Some(64)
    );
    assert_eq!(
        plan.deployment_identity
            .role_topology_hash
            .as_ref()
            .map(String::len),
        Some(64)
    );
    assert_eq!(
        plan.deployment_identity
            .artifact_set_digest
            .as_ref()
            .map(String::len),
        Some(64)
    );
    assert_eq!(
        plan.deployment_identity
            .pool_identity_set_digest
            .as_ref()
            .map(String::len),
        Some(64)
    );
    assert_eq!(
        plan.role_artifacts[0]
            .raw_config_sha256
            .as_ref()
            .map(String::len),
        Some(64)
    );
    assert_eq!(plan.fleet_template, "demo");
    assert_eq!(plan.runtime_variant, "local");
    assert_eq!(plan.role_artifacts.len(), 3);
    assert!(
        plan.role_artifacts
            .iter()
            .all(|artifact| artifact.build_profile == "fast")
    );
    assert_plan_has_implicit_wasm_store_artifact(&plan);
    assert_plan_has_user_hub_release_artifact(&plan);
    assert_eq!(
        plan.expected_canisters
            .iter()
            .map(|canister| canister.role.as_str())
            .collect::<Vec<_>>(),
        vec!["root", "wasm_store", "user_hub"]
    );
    assert!(
        plan.unresolved_assumptions
            .iter()
            .any(|assumption| assumption.key == "local_state.root_canister_id")
    );
}

fn assert_plan_has_implicit_wasm_store_artifact(plan: &DeploymentPlanV1) {
    assert!(
        plan.role_artifacts
            .iter()
            .any(|artifact| artifact.role == "wasm_store"
                && artifact.source == ArtifactSourceV1::WasmStore
                && artifact.observed_wasm_gz_file_sha256_source
                    == Some(ArtifactDigestSourceV1::ObservedFileDigest))
    );
}

fn assert_plan_has_user_hub_release_artifact(plan: &DeploymentPlanV1) {
    assert!(
        plan.role_artifacts
            .iter()
            .any(|artifact| artifact.role == "user_hub"
                && artifact.wasm_gz_sha256.as_deref() == Some("user-hub-hash")
                && artifact.wasm_gz_sha256_source
                    == Some(ArtifactDigestSourceV1::ReleaseSetManifest)
                && artifact.observed_wasm_gz_file_sha256_source
                    == Some(ArtifactDigestSourceV1::ObservedFileDigest))
    );
}

#[test]
fn local_plan_uses_configured_controllers_as_expected_authority() {
    let temp = TempWorkspace::new("canic-host-local-plan-controllers");
    let workspace_root = temp.path().join("workspace");
    let icp_root = temp.path().join("icp");
    let config_dir = workspace_root.join("fleets");
    fs::create_dir_all(&config_dir).expect("create config dir");
    fs::write(
        config_dir.join("canic.toml"),
        r#"
controllers = [
  "zbf4m-zw3nk-6owqc-qmluz-xhwxt-2pkky-xhjy2-kqxor-qzxsn-6d2bz-nae",
  "aaaaa-aa",
]
app_index = []

[fleet]
name = "demo"

[app]
init_mode = "enabled"
[app.whitelist]

[subnets.prime.canisters.root]
kind = "root"
"#,
    )
    .expect("write config");
    write_artifact(&icp_root, "root", b"root-artifact");

    let plan = build_local_deployment_plan(&LocalDeploymentPlanRequest {
        deployment_name: "demo-local".to_string(),
        network: "local".to_string(),
        workspace_root,
        icp_root,
        config_path: None,
        runtime_variant: "local".to_string(),
        build_profile: "fast".to_string(),
    });

    assert_eq!(
        plan.authority_profile.expected_controllers,
        vec![
            "aaaaa-aa".to_string(),
            "zbf4m-zw3nk-6owqc-qmluz-xhwxt-2pkky-xhjy2-kqxor-qzxsn-6d2bz-nae".to_string(),
        ]
    );
    assert!(plan.authority_profile.staging_controllers.is_empty());
    assert!(plan.authority_profile.emergency_controllers.is_empty());
    assert!(
        plan.unresolved_assumptions
            .iter()
            .any(|assumption| assumption.key == "local_state.root_canister_id")
    );
}

#[test]
fn local_plan_uses_install_state_root_as_expected_canister() {
    let temp = TempWorkspace::new("canic-host-local-plan-root-state");
    let workspace_root = temp.path().join("workspace");
    let icp_root = temp.path().join("icp");
    let config_dir = workspace_root.join("fleets");
    fs::create_dir_all(&config_dir).expect("create config dir");
    fs::write(config_dir.join("canic.toml"), SAMPLE_CONFIG).expect("write config");
    write_artifact(&icp_root, "root", b"root-artifact");
    write_artifact(&icp_root, "wasm_store", b"wasm-store-artifact");
    write_artifact(&icp_root, "user_hub", b"user-hub-artifact");
    write_release_set_manifest(&icp_root);
    let state_path = icp_root.join(".canic/local/fleets/demo.json");
    fs::create_dir_all(state_path.parent().expect("state parent")).expect("create state dir");
    fs::write(
        state_path,
        serde_json::to_vec_pretty(&sample_install_state("aaaaa-aa")).expect("encode state"),
    )
    .expect("write install state");

    let plan = build_local_deployment_plan(&LocalDeploymentPlanRequest {
        deployment_name: "demo-local".to_string(),
        network: "local".to_string(),
        workspace_root,
        icp_root,
        config_path: None,
        runtime_variant: "local".to_string(),
        build_profile: "fast".to_string(),
    });

    assert_eq!(
        plan.deployment_identity.root_principal.as_deref(),
        Some("aaaaa-aa")
    );
    assert_eq!(
        plan.trust_domain.root_trust_anchor.as_deref(),
        Some("aaaaa-aa")
    );
    assert!(
        plan.expected_canisters
            .iter()
            .any(|canister| canister.role == "root"
                && canister.canister_id.as_deref() == Some("aaaaa-aa"))
    );
    assert!(plan.unresolved_assumptions.is_empty());
}

#[test]
fn local_plan_uses_configured_pools_as_expected_pool_identities() {
    let temp = TempWorkspace::new("canic-host-local-plan-pools");
    let workspace_root = temp.path().join("workspace");
    let icp_root = temp.path().join("icp");
    let config_dir = workspace_root.join("fleets");
    fs::create_dir_all(&config_dir).expect("create config dir");
    fs::write(
        config_dir.join("canic.toml"),
        r#"
controllers = []
app_index = []

[fleet]
name = "demo"

[app]
init_mode = "enabled"
[app.whitelist]

[subnets.prime.canisters.root]
kind = "root"

[subnets.prime.canisters.user_hub]
kind = "singleton"

[subnets.prime.canisters.user_hub.sharding.pools.user_shards]
canister_role = "user_shard"
policy.capacity = 100
policy.max_shards = 4

[subnets.prime.canisters.user_shard]
kind = "shard"
"#,
    )
    .expect("write config");
    write_artifact(&icp_root, "root", b"root-artifact");
    write_artifact(&icp_root, "user_hub", b"user-hub-artifact");
    write_artifact(&icp_root, "user_shard", b"user-shard-artifact");

    let plan = build_local_deployment_plan(&LocalDeploymentPlanRequest {
        deployment_name: "demo-local".to_string(),
        network: "local".to_string(),
        workspace_root,
        icp_root,
        config_path: None,
        runtime_variant: "local".to_string(),
        build_profile: "fast".to_string(),
    });

    assert_eq!(
        plan.expected_pool,
        vec![ExpectedPoolCanisterV1 {
            pool: "user_shards".to_string(),
            canister_id: None,
            role: Some("user_shard".to_string()),
        }]
    );
    let inventory = sample_matching_inventory();
    let diff = compare_plan_to_inventory(&plan, &inventory);
    assert!(
        diff.warnings
            .iter()
            .any(|finding| finding.code == "pool_canister_unobserved"
                && finding.subject.as_deref() == Some("user_shards:user_shard"))
    );
}

#[test]
fn deployment_diff_blocks_deployment_manifest_mismatch() {
    let mut plan = sample_plan();
    plan.expected_canisters.clear();
    plan.role_artifacts[0].wasm_gz_sha256 = None;
    plan.expected_verifier_readiness.required = false;
    let mut observed_identity = sample_identity();
    observed_identity.deployment_manifest_digest = Some("different-manifest".to_string());
    let inventory = DeploymentInventoryV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        inventory_id: "inventory-1".to_string(),
        observed_at: "2026-05-21T00:00:00Z".to_string(),
        observed_identity: Some(observed_identity),
        local_config: LocalDeploymentConfigV1 {
            config_path: Some("icp.yml".to_string()),
            raw_config_sha256: Some("different-manifest".to_string()),
            canonical_embedded_config_sha256: Some("runtime".to_string()),
        },
        observed_canisters: Vec::new(),
        observed_pool: Vec::new(),
        observed_artifacts: vec![ObservedArtifactV1 {
            role: "root".to_string(),
            artifact_path: "root.wasm.gz".to_string(),
            file_sha256: Some("file".to_string()),
            file_sha256_source: Some(ArtifactDigestSourceV1::ObservedFileDigest),
            payload_sha256: None,
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

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Blocked);
    assert!(
        diff.hard_failures
            .iter()
            .any(|finding| finding.code == "deployment_manifest_mismatch")
    );
}

#[test]
fn deployment_diff_blocks_raw_config_digest_mismatch_without_claiming_manifest_identity() {
    let mut plan = sample_plan();
    plan.deployment_identity.deployment_manifest_digest = None;
    plan.expected_canisters.clear();
    plan.role_artifacts[0].wasm_gz_sha256 = None;
    plan.role_artifacts[0].raw_config_sha256 = Some("planned-raw-config".to_string());
    plan.expected_verifier_readiness.required = false;
    let mut observed_identity = sample_identity();
    observed_identity.deployment_manifest_digest = None;
    let inventory = DeploymentInventoryV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        inventory_id: "inventory-1".to_string(),
        observed_at: "2026-05-21T00:00:00Z".to_string(),
        observed_identity: Some(observed_identity),
        local_config: LocalDeploymentConfigV1 {
            config_path: Some("icp.yml".to_string()),
            raw_config_sha256: Some("observed-raw-config".to_string()),
            canonical_embedded_config_sha256: Some("runtime".to_string()),
        },
        observed_canisters: Vec::new(),
        observed_pool: Vec::new(),
        observed_artifacts: vec![ObservedArtifactV1 {
            role: "root".to_string(),
            artifact_path: "root.wasm.gz".to_string(),
            file_sha256: Some("file".to_string()),
            file_sha256_source: Some(ArtifactDigestSourceV1::ObservedFileDigest),
            payload_sha256: None,
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

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Blocked);
    assert!(
        diff.hard_failures
            .iter()
            .any(|finding| finding.code == "raw_config_digest_mismatch")
    );
    assert!(diff.embedded_config_diff.iter().any(|item| {
        item.category == "raw_config_sha256"
            && item.expected.as_deref() == Some("planned-raw-config")
            && item.observed.as_deref() == Some("observed-raw-config")
    }));
}

#[test]
fn deployment_diff_blocks_installed_module_hash_mismatch() {
    let mut plan = sample_plan();
    plan.expected_canisters.clear();
    plan.role_artifacts[0].wasm_gz_sha256 = None;
    plan.expected_verifier_readiness.required = false;
    let inventory = DeploymentInventoryV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        inventory_id: "inventory-1".to_string(),
        observed_at: "2026-05-21T00:00:00Z".to_string(),
        observed_identity: Some(sample_identity()),
        local_config: LocalDeploymentConfigV1 {
            config_path: Some("icp.yml".to_string()),
            raw_config_sha256: None,
            canonical_embedded_config_sha256: Some("runtime".to_string()),
        },
        observed_canisters: vec![ObservedCanisterV1 {
            canister_id: "aaaaa-aa".to_string(),
            role: Some("root".to_string()),
            control_class: CanisterControlClassV1::DeploymentControlled,
            controllers: vec!["aaaaa-aa".to_string()],
            module_hash: Some("different-module".to_string()),
            status: Some("Running".to_string()),
            root_trust_anchor: Some("aaaaa-aa".to_string()),
            canonical_embedded_config_digest: None,
            role_assignment_source: Some("icp_canister_status".to_string()),
        }],
        observed_pool: Vec::new(),
        observed_artifacts: vec![ObservedArtifactV1 {
            role: "root".to_string(),
            artifact_path: "root.wasm.gz".to_string(),
            file_sha256: Some("file".to_string()),
            file_sha256_source: Some(ArtifactDigestSourceV1::ObservedFileDigest),
            payload_sha256: None,
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

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Blocked);
    assert!(
        diff.hard_failures
            .iter()
            .any(|finding| finding.code == "installed_module_hash_mismatch")
    );
    assert!(diff.module_hash_diff.iter().any(|item| {
        item.category == "installed_module_hash"
            && item.expected.as_deref() == Some("module")
            && item.observed.as_deref() == Some("different-module")
    }));
}

#[test]
fn deployment_diff_uses_concrete_expected_id_for_installed_module_hash() {
    let mut plan = sample_plan();
    plan.expected_verifier_readiness.required = false;
    let mut inventory = sample_matching_inventory();
    inventory.observed_canisters.push(ObservedCanisterV1 {
        canister_id: "duplicate-root-id".to_string(),
        role: Some("root".to_string()),
        control_class: CanisterControlClassV1::DeploymentControlled,
        controllers: vec!["aaaaa-aa".to_string()],
        module_hash: Some("different-module".to_string()),
        status: Some("Running".to_string()),
        root_trust_anchor: Some("aaaaa-aa".to_string()),
        canonical_embedded_config_digest: None,
        role_assignment_source: Some("subnet_registry+icp_canister_status".to_string()),
    });

    let diff = compare_plan_to_inventory(&plan, &inventory);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Warning);
    assert!(
        diff.hard_failures
            .iter()
            .all(|finding| finding.code != "installed_module_hash_mismatch")
    );
    assert!(
        diff.module_hash_diff
            .iter()
            .all(|item| item.category != "installed_module_hash")
    );
}

#[test]
fn deployment_diff_blocks_ambiguous_installed_module_hash_target() {
    let mut plan = sample_plan();
    plan.expected_canisters.clear();
    plan.expected_verifier_readiness.required = false;
    let mut inventory = sample_matching_inventory();
    inventory.observed_canisters.push(ObservedCanisterV1 {
        canister_id: "duplicate-root-id".to_string(),
        role: Some("root".to_string()),
        control_class: CanisterControlClassV1::DeploymentControlled,
        controllers: vec!["aaaaa-aa".to_string()],
        module_hash: Some("module".to_string()),
        status: Some("Running".to_string()),
        root_trust_anchor: Some("aaaaa-aa".to_string()),
        canonical_embedded_config_digest: None,
        role_assignment_source: Some("subnet_registry+icp_canister_status".to_string()),
    });

    let diff = compare_plan_to_inventory(&plan, &inventory);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Blocked);
    assert!(
        diff.hard_failures
            .iter()
            .any(|finding| finding.code == "installed_module_hash_ambiguous"
                && finding.subject.as_deref() == Some("root"))
    );
    assert!(diff.module_hash_diff.iter().any(|item| {
        item.category == "installed_module_hash_ambiguous"
            && item.subject == "root"
            && item.observed.as_deref().is_some_and(|observed| {
                observed.contains("aaaaa-aa") && observed.contains("duplicate-root-id")
            })
            && item.severity == SafetySeverityV1::HardFailure
    }));
}

#[test]
fn deployment_diff_blocks_missing_expected_controller() {
    let mut plan = sample_plan();
    plan.role_artifacts[0].wasm_gz_sha256 = None;
    plan.role_artifacts[0].installed_module_hash = None;
    plan.expected_verifier_readiness.required = false;
    let inventory = DeploymentInventoryV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        inventory_id: "inventory-1".to_string(),
        observed_at: "2026-05-21T00:00:00Z".to_string(),
        observed_identity: Some(sample_identity()),
        local_config: LocalDeploymentConfigV1 {
            config_path: Some("icp.yml".to_string()),
            raw_config_sha256: None,
            canonical_embedded_config_sha256: Some("runtime".to_string()),
        },
        observed_canisters: vec![ObservedCanisterV1 {
            canister_id: "aaaaa-aa".to_string(),
            role: Some("root".to_string()),
            control_class: CanisterControlClassV1::DeploymentControlled,
            controllers: vec!["external-controller".to_string()],
            module_hash: None,
            status: Some("Running".to_string()),
            root_trust_anchor: Some("aaaaa-aa".to_string()),
            canonical_embedded_config_digest: None,
            role_assignment_source: Some("icp_canister_status".to_string()),
        }],
        observed_pool: Vec::new(),
        observed_artifacts: vec![ObservedArtifactV1 {
            role: "root".to_string(),
            artifact_path: "root.wasm.gz".to_string(),
            file_sha256: Some("file".to_string()),
            file_sha256_source: Some(ArtifactDigestSourceV1::ObservedFileDigest),
            payload_sha256: None,
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

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Blocked);
    assert!(
        diff.hard_failures
            .iter()
            .any(|finding| finding.code == "expected_controller_missing")
    );
    assert!(diff.controller_diff.iter().any(|item| {
        item.category == "controller_missing"
            && item.expected.as_deref() == Some("aaaaa-aa")
            && item.observed.as_deref() == Some("external-controller")
    }));
}

#[test]
fn deployment_diff_warns_for_extra_declared_emergency_controller() {
    let mut plan = sample_plan();
    plan.role_artifacts[0].wasm_gz_sha256 = None;
    plan.role_artifacts[0].installed_module_hash = None;
    plan.authority_profile
        .emergency_controllers
        .push("emergency-controller".to_string());
    plan.expected_verifier_readiness.required = false;
    let inventory = DeploymentInventoryV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        inventory_id: "inventory-1".to_string(),
        observed_at: "2026-05-21T00:00:00Z".to_string(),
        observed_identity: Some(sample_identity()),
        local_config: LocalDeploymentConfigV1 {
            config_path: Some("icp.yml".to_string()),
            raw_config_sha256: None,
            canonical_embedded_config_sha256: Some("runtime".to_string()),
        },
        observed_canisters: vec![ObservedCanisterV1 {
            canister_id: "aaaaa-aa".to_string(),
            role: Some("root".to_string()),
            control_class: CanisterControlClassV1::DeploymentControlled,
            controllers: vec!["aaaaa-aa".to_string(), "emergency-controller".to_string()],
            module_hash: None,
            status: Some("Running".to_string()),
            root_trust_anchor: Some("aaaaa-aa".to_string()),
            canonical_embedded_config_digest: None,
            role_assignment_source: Some("icp_canister_status".to_string()),
        }],
        observed_pool: Vec::new(),
        observed_artifacts: vec![ObservedArtifactV1 {
            role: "root".to_string(),
            artifact_path: "root.wasm.gz".to_string(),
            file_sha256: Some("file".to_string()),
            file_sha256_source: Some(ArtifactDigestSourceV1::ObservedFileDigest),
            payload_sha256: None,
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

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Safe);
    assert!(diff.hard_failures.is_empty());
    assert!(
        diff.warnings
            .iter()
            .all(|finding| finding.code != "extra_controller_observed")
    );
}

#[test]
fn deployment_diff_blocks_authority_profile_controller_overlap() {
    let mut plan = sample_plan();
    plan.role_artifacts.clear();
    plan.expected_canisters.clear();
    plan.expected_verifier_readiness.required = false;
    plan.authority_profile
        .staging_controllers
        .push("aaaaa-aa".to_string());
    let inventory = DeploymentInventoryV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        inventory_id: "inventory-1".to_string(),
        observed_at: "2026-05-21T00:00:00Z".to_string(),
        observed_identity: Some(sample_identity()),
        local_config: LocalDeploymentConfigV1 {
            config_path: Some("icp.yml".to_string()),
            raw_config_sha256: None,
            canonical_embedded_config_sha256: Some("runtime".to_string()),
        },
        observed_canisters: Vec::new(),
        observed_pool: Vec::new(),
        observed_artifacts: Vec::new(),
        observed_verifier_readiness: VerifierReadinessObservationV1 {
            status: ObservationStatusV1::NotObserved,
            role_epochs: Vec::new(),
        },
        unresolved_observations: Vec::new(),
    };

    let diff = compare_plan_to_inventory(&plan, &inventory);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Blocked);
    assert!(
        diff.hard_failures
            .iter()
            .any(|finding| finding.code == "controller_authority_overlap")
    );
    assert!(diff.controller_diff.iter().any(|item| {
        item.category == "controller_authority_overlap"
            && item.expected.as_deref() == Some("expected-only")
            && item.observed.as_deref() == Some("aaaaa-aa")
    }));
}

#[test]
fn deployment_diff_warns_for_undeclared_extra_controller() {
    let mut plan = sample_plan();
    plan.role_artifacts[0].wasm_gz_sha256 = None;
    plan.role_artifacts[0].installed_module_hash = None;
    plan.expected_verifier_readiness.required = false;
    let inventory = DeploymentInventoryV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        inventory_id: "inventory-1".to_string(),
        observed_at: "2026-05-21T00:00:00Z".to_string(),
        observed_identity: Some(sample_identity()),
        local_config: LocalDeploymentConfigV1 {
            config_path: Some("icp.yml".to_string()),
            raw_config_sha256: None,
            canonical_embedded_config_sha256: Some("runtime".to_string()),
        },
        observed_canisters: vec![ObservedCanisterV1 {
            canister_id: "aaaaa-aa".to_string(),
            role: Some("root".to_string()),
            control_class: CanisterControlClassV1::DeploymentControlled,
            controllers: vec!["aaaaa-aa".to_string(), "surprise-controller".to_string()],
            module_hash: None,
            status: Some("Running".to_string()),
            root_trust_anchor: Some("aaaaa-aa".to_string()),
            canonical_embedded_config_digest: None,
            role_assignment_source: Some("icp_canister_status".to_string()),
        }],
        observed_pool: Vec::new(),
        observed_artifacts: vec![ObservedArtifactV1 {
            role: "root".to_string(),
            artifact_path: "root.wasm.gz".to_string(),
            file_sha256: Some("file".to_string()),
            file_sha256_source: Some(ArtifactDigestSourceV1::ObservedFileDigest),
            payload_sha256: None,
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

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Warning);
    assert!(diff.hard_failures.is_empty());
    assert!(
        diff.warnings
            .iter()
            .any(|finding| finding.code == "extra_controller_observed")
    );
    assert!(diff.controller_diff.iter().any(|item| {
        item.category == "controller_extra"
            && item.expected.as_deref() == Some("aaaaa-aa")
            && item.observed.as_deref() == Some("surprise-controller")
    }));
}

#[test]
fn deployment_diff_blocks_artifact_file_digest_mismatch() {
    let mut plan = sample_plan();
    plan.expected_canisters.clear();
    plan.role_artifacts[0].wasm_gz_sha256 = None;
    plan.role_artifacts[0].observed_wasm_gz_file_sha256 = Some("planned-file".to_string());
    plan.expected_verifier_readiness.required = false;
    let inventory = DeploymentInventoryV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        inventory_id: "inventory-1".to_string(),
        observed_at: "2026-05-21T00:00:00Z".to_string(),
        observed_identity: Some(sample_identity()),
        local_config: LocalDeploymentConfigV1 {
            config_path: Some("icp.yml".to_string()),
            raw_config_sha256: None,
            canonical_embedded_config_sha256: Some("runtime".to_string()),
        },
        observed_canisters: Vec::new(),
        observed_pool: Vec::new(),
        observed_artifacts: vec![ObservedArtifactV1 {
            role: "root".to_string(),
            artifact_path: "root.wasm.gz".to_string(),
            file_sha256: Some("observed-file".to_string()),
            file_sha256_source: Some(ArtifactDigestSourceV1::ObservedFileDigest),
            payload_sha256: None,
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

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Blocked);
    assert!(
        diff.hard_failures
            .iter()
            .any(|finding| finding.code == "artifact_file_digest_mismatch")
    );
    assert!(diff.artifact_diff.iter().any(|item| {
        item.category == "artifact_file_sha256"
            && item.expected.as_deref() == Some("planned-file")
            && item.observed.as_deref() == Some("observed-file")
            && item.severity == SafetySeverityV1::HardFailure
    }));
}

#[test]
fn deployment_diff_blocks_conflicting_artifact_observations_for_same_role() {
    let plan = sample_plan();
    let mut inventory = sample_matching_inventory();
    inventory.observed_artifacts.push(ObservedArtifactV1 {
        role: "root".to_string(),
        artifact_path: "alternate-root.wasm.gz".to_string(),
        file_sha256: Some("different-file".to_string()),
        file_sha256_source: Some(ArtifactDigestSourceV1::ObservedFileDigest),
        payload_sha256: Some("different-gzip".to_string()),
        payload_size_bytes: Some(99),
        source: ArtifactSourceV1::LocalBuild,
    });

    let diff = compare_plan_to_inventory(&plan, &inventory);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Blocked);
    assert!(
        diff.hard_failures
            .iter()
            .any(|finding| finding.code == "artifact_role_conflict"
                && finding.subject.as_deref() == Some("root"))
    );
    assert!(diff.artifact_diff.iter().any(|item| {
        item.category == "artifact_role_conflict"
            && item.subject == "root"
            && item.observed.as_deref().is_some_and(|observed| {
                observed.contains("root.wasm.gz") && observed.contains("alternate-root.wasm.gz")
            })
            && item.severity == SafetySeverityV1::HardFailure
    }));
}

#[test]
fn deployment_diff_warns_for_duplicate_identical_artifact_observation() {
    let mut inventory = sample_matching_inventory();
    inventory
        .observed_artifacts
        .push(inventory.observed_artifacts[0].clone());

    let diff = compare_plan_to_inventory(&sample_plan(), &inventory);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Warning);
    assert!(diff.hard_failures.is_empty());
    assert!(
        diff.warnings
            .iter()
            .any(|finding| finding.code == "duplicate_artifact_observed"
                && finding.subject.as_deref() == Some("root"))
    );
    assert!(diff.artifact_diff.iter().any(|item| {
        item.category == "artifact_duplicate"
            && item.subject == "root"
            && item.observed.as_deref() == Some("2")
            && item.severity == SafetySeverityV1::Warning
    }));
}

#[test]
fn deployment_diff_blocks_conflicting_planned_artifacts_for_same_role() {
    let mut plan = sample_plan();
    let mut duplicate = sample_role_artifact();
    duplicate.wasm_gz_path = Some("alternate-root.wasm.gz".to_string());
    duplicate.wasm_gz_sha256 = Some("different-gzip".to_string());
    plan.role_artifacts.push(duplicate);

    let diff = compare_plan_to_inventory(&plan, &sample_matching_inventory());

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Blocked);
    assert!(
        diff.hard_failures
            .iter()
            .any(|finding| finding.code == "planned_artifact_role_conflict"
                && finding.subject.as_deref() == Some("root"))
    );
    assert!(diff.artifact_diff.iter().any(|item| {
        item.category == "planned_artifact_role_conflict"
            && item.subject == "root"
            && item.observed.as_deref().is_some_and(|observed| {
                observed.contains("root.wasm.gz") && observed.contains("alternate-root.wasm.gz")
            })
            && item.severity == SafetySeverityV1::HardFailure
    }));
}

#[test]
fn deployment_diff_warns_for_duplicate_identical_planned_artifact_role() {
    let mut plan = sample_plan();
    plan.role_artifacts.push(sample_role_artifact());

    let diff = compare_plan_to_inventory(&plan, &sample_matching_inventory());

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Warning);
    assert!(diff.hard_failures.is_empty());
    assert!(
        diff.warnings
            .iter()
            .any(|finding| finding.code == "duplicate_planned_artifact_role"
                && finding.subject.as_deref() == Some("root"))
    );
    assert!(diff.artifact_diff.iter().any(|item| {
        item.category == "planned_artifact_duplicate"
            && item.subject == "root"
            && item.observed.as_deref() == Some("2")
            && item.severity == SafetySeverityV1::Warning
    }));
}

#[test]
fn deployment_diff_blocks_missing_artifacts_and_unsafe_control_class() {
    let plan = sample_plan();
    let inventory = DeploymentInventoryV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        inventory_id: "inventory-1".to_string(),
        observed_at: "2026-05-21T00:00:00Z".to_string(),
        observed_identity: Some(sample_identity()),
        local_config: LocalDeploymentConfigV1 {
            config_path: Some("icp.yml".to_string()),
            raw_config_sha256: None,
            canonical_embedded_config_sha256: Some("runtime".to_string()),
        },
        observed_canisters: vec![ObservedCanisterV1 {
            canister_id: "aaaaa-aa".to_string(),
            role: Some("root".to_string()),
            control_class: CanisterControlClassV1::UnknownUnsafe,
            controllers: Vec::new(),
            module_hash: None,
            status: None,
            root_trust_anchor: Some("aaaaa-aa".to_string()),
            canonical_embedded_config_digest: None,
            role_assignment_source: Some("local_install_state".to_string()),
        }],
        observed_pool: Vec::new(),
        observed_artifacts: Vec::new(),
        observed_verifier_readiness: VerifierReadinessObservationV1 {
            status: ObservationStatusV1::Observed,
            role_epochs: Vec::new(),
        },
        unresolved_observations: Vec::new(),
    };

    let diff = compare_plan_to_inventory(&plan, &inventory);
    let report = safety_report_from_diff("report-1", Some("diff-1".to_string()), &diff);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Blocked);
    assert!(
        diff.hard_failures
            .iter()
            .any(|item| item.code == "artifact_missing")
    );
    assert!(
        diff.hard_failures
            .iter()
            .any(|item| item.code == "unsafe_control_class")
    );
    assert_eq!(report.status, SafetyStatusV1::Blocked);
    assert_eq!(
        report.next_actions,
        vec!["resolve blocking deployment truth differences before mutation".to_string()]
    );
}

#[test]
fn deployment_diff_warns_on_observation_gaps_without_blocking() {
    let mut plan = sample_plan();
    plan.expected_canisters.clear();
    plan.role_artifacts[0].wasm_gz_sha256 = None;
    plan.expected_verifier_readiness.required = false;
    let inventory = DeploymentInventoryV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        inventory_id: "inventory-1".to_string(),
        observed_at: "2026-05-21T00:00:00Z".to_string(),
        observed_identity: Some(sample_identity()),
        local_config: LocalDeploymentConfigV1 {
            config_path: Some("icp.yml".to_string()),
            raw_config_sha256: None,
            canonical_embedded_config_sha256: Some("runtime".to_string()),
        },
        observed_canisters: Vec::new(),
        observed_pool: Vec::new(),
        observed_artifacts: vec![ObservedArtifactV1 {
            role: "root".to_string(),
            artifact_path: "root.wasm.gz".to_string(),
            file_sha256: Some("file".to_string()),
            file_sha256_source: Some(ArtifactDigestSourceV1::ObservedFileDigest),
            payload_sha256: None,
            payload_size_bytes: Some(10),
            source: ArtifactSourceV1::LocalBuild,
        }],
        observed_verifier_readiness: VerifierReadinessObservationV1 {
            status: ObservationStatusV1::NotObserved,
            role_epochs: Vec::new(),
        },
        unresolved_observations: vec![DeploymentObservationGapV1 {
            key: "local_artifacts.user_hub".to_string(),
            description: "missing built artifact".to_string(),
        }],
    };

    let diff = compare_plan_to_inventory(&plan, &inventory);
    let report = safety_report_from_diff("report-1", None, &diff);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Warning);
    assert!(diff.hard_failures.is_empty());
    assert!(
        diff.artifact_diff
            .iter()
            .any(|item| item.category == "artifact_file_sha256"
                && item.severity == SafetySeverityV1::Info)
    );
    assert!(
        diff.warnings
            .iter()
            .any(|item| item.code == "observation_gap")
    );
    assert_eq!(report.status, SafetyStatusV1::Warning);
}

#[test]
fn deployment_diff_warns_on_plan_assumptions_without_blocking() {
    let mut plan = sample_plan();
    plan.expected_canisters.clear();
    plan.role_artifacts[0].wasm_gz_sha256 = None;
    plan.expected_verifier_readiness.required = false;
    plan.unresolved_assumptions.push(DeploymentAssumptionV1 {
        key: "local_state.root_canister_id".to_string(),
        description: "root identity is unknown until install".to_string(),
    });
    let inventory = DeploymentInventoryV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        inventory_id: "inventory-1".to_string(),
        observed_at: "2026-05-21T00:00:00Z".to_string(),
        observed_identity: Some(sample_identity()),
        local_config: LocalDeploymentConfigV1 {
            config_path: Some("icp.yml".to_string()),
            raw_config_sha256: None,
            canonical_embedded_config_sha256: Some("runtime".to_string()),
        },
        observed_canisters: Vec::new(),
        observed_pool: Vec::new(),
        observed_artifacts: vec![ObservedArtifactV1 {
            role: "root".to_string(),
            artifact_path: "root.wasm.gz".to_string(),
            file_sha256: Some("file".to_string()),
            file_sha256_source: Some(ArtifactDigestSourceV1::ObservedFileDigest),
            payload_sha256: None,
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
    let report = safety_report_from_diff("report-1", None, &diff);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Warning);
    assert!(diff.hard_failures.is_empty());
    assert!(
        diff.warnings
            .iter()
            .any(|item| item.code == "plan_assumption"
                && item.subject.as_deref() == Some("local_state.root_canister_id"))
    );
    assert_eq!(report.status, SafetyStatusV1::Warning);
}

#[test]
fn deployment_diff_warns_when_unspecified_canister_id_is_unobserved() {
    let mut plan = sample_plan();
    plan.expected_canisters[0].canister_id = None;
    plan.expected_verifier_readiness.required = false;
    let inventory = DeploymentInventoryV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        inventory_id: "inventory-1".to_string(),
        observed_at: "2026-05-21T00:00:00Z".to_string(),
        observed_identity: Some(sample_identity()),
        local_config: LocalDeploymentConfigV1 {
            config_path: Some("icp.yml".to_string()),
            raw_config_sha256: None,
            canonical_embedded_config_sha256: Some("runtime".to_string()),
        },
        observed_canisters: Vec::new(),
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

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Warning);
    assert!(diff.hard_failures.is_empty());
    assert!(
        diff.warnings
            .iter()
            .any(|finding| finding.code == "canister_unobserved"
                && finding.subject.as_deref() == Some("root"))
    );
}

#[test]
fn deployment_diff_blocks_conflicting_planned_canisters_for_same_role() {
    let mut plan = sample_plan();
    plan.role_artifacts.clear();
    plan.expected_verifier_readiness.required = false;
    plan.expected_canisters.push(ExpectedCanisterV1 {
        role: "root".to_string(),
        canister_id: Some("duplicate-root-id".to_string()),
        control_class: CanisterControlClassV1::DeploymentControlled,
    });

    let diff = compare_plan_to_inventory(&plan, &sample_matching_inventory());

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Blocked);
    assert!(
        diff.hard_failures
            .iter()
            .any(|finding| finding.code == "planned_canister_role_conflict"
                && finding.subject.as_deref() == Some("root"))
    );
    assert!(diff.controller_diff.iter().any(|item| {
        item.category == "planned_canister_role_conflict"
            && item.subject == "root"
            && item.observed.as_deref().is_some_and(|observed| {
                observed.contains("aaaaa-aa") && observed.contains("duplicate-root-id")
            })
            && item.severity == SafetySeverityV1::HardFailure
    }));
}

#[test]
fn deployment_diff_blocks_conflicting_planned_roles_for_same_canister_id() {
    let mut plan = sample_plan();
    plan.role_artifacts.clear();
    plan.expected_verifier_readiness.required = false;
    plan.expected_canisters.push(ExpectedCanisterV1 {
        role: "user_hub".to_string(),
        canister_id: Some("aaaaa-aa".to_string()),
        control_class: CanisterControlClassV1::DeploymentControlled,
    });

    let diff = compare_plan_to_inventory(&plan, &sample_matching_inventory());

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Blocked);
    assert!(
        diff.hard_failures
            .iter()
            .any(|finding| finding.code == "planned_canister_id_conflict"
                && finding.subject.as_deref() == Some("aaaaa-aa"))
    );
    assert!(diff.controller_diff.iter().any(|item| {
        item.category == "planned_canister_id_conflict"
            && item.subject == "aaaaa-aa"
            && item
                .observed
                .as_deref()
                .is_some_and(|observed| observed.contains("root") && observed.contains("user_hub"))
            && item.severity == SafetySeverityV1::HardFailure
    }));
}

#[test]
fn deployment_diff_warns_for_duplicate_identical_planned_canister_role() {
    let mut plan = sample_plan();
    plan.role_artifacts.clear();
    plan.expected_verifier_readiness.required = false;
    plan.expected_canisters
        .push(plan.expected_canisters[0].clone());

    let diff = compare_plan_to_inventory(&plan, &sample_matching_inventory());

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Warning);
    assert!(diff.hard_failures.is_empty());
    assert!(
        diff.warnings
            .iter()
            .any(|finding| finding.code == "duplicate_planned_canister_role"
                && finding.subject.as_deref() == Some("root"))
    );
    assert!(diff.controller_diff.iter().any(|item| {
        item.category == "planned_canister_duplicate"
            && item.subject == "root"
            && item.observed.as_deref() == Some("2")
            && item.severity == SafetySeverityV1::Warning
    }));
}

#[test]
fn deployment_diff_warns_for_extra_observed_canister_roles() {
    let plan = sample_plan();
    let mut inventory = sample_matching_inventory();
    inventory.observed_canisters.push(ObservedCanisterV1 {
        canister_id: "user-hub-id".to_string(),
        role: Some("user_hub".to_string()),
        control_class: CanisterControlClassV1::CanicManagedPool,
        controllers: vec!["aaaaa-aa".to_string()],
        module_hash: Some("module".to_string()),
        status: Some("Running".to_string()),
        root_trust_anchor: Some("aaaaa-aa".to_string()),
        canonical_embedded_config_digest: None,
        role_assignment_source: Some("subnet_registry+icp_canister_status".to_string()),
    });

    let diff = compare_plan_to_inventory(&plan, &inventory);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Warning);
    assert!(diff.hard_failures.is_empty());
    assert!(
        diff.warnings
            .iter()
            .any(|finding| finding.code == "extra_canister_observed"
                && finding.subject.as_deref() == Some("user_hub"))
    );
    assert!(diff.controller_diff.iter().any(|item| {
        item.category == "canister_extra"
            && item.subject == "user_hub"
            && item.observed.as_deref() == Some("user-hub-id")
    }));
}

#[test]
fn deployment_diff_warns_for_duplicate_observed_planned_role() {
    let plan = sample_plan();
    let mut inventory = sample_matching_inventory();
    inventory.observed_canisters.push(ObservedCanisterV1 {
        canister_id: "duplicate-root-id".to_string(),
        role: Some("root".to_string()),
        control_class: CanisterControlClassV1::DeploymentControlled,
        controllers: vec!["aaaaa-aa".to_string()],
        module_hash: Some("module".to_string()),
        status: Some("Running".to_string()),
        root_trust_anchor: Some("aaaaa-aa".to_string()),
        canonical_embedded_config_digest: None,
        role_assignment_source: Some("subnet_registry+icp_canister_status".to_string()),
    });

    let diff = compare_plan_to_inventory(&plan, &inventory);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Warning);
    assert!(diff.hard_failures.is_empty());
    assert!(
        diff.warnings
            .iter()
            .any(|finding| finding.code == "extra_canister_observed"
                && finding.subject.as_deref() == Some("root"))
    );
    assert!(diff.controller_diff.iter().any(|item| {
        item.category == "canister_extra"
            && item.subject == "root"
            && item.observed.as_deref() == Some("duplicate-root-id")
    }));
}

#[test]
fn deployment_diff_blocks_ambiguous_expected_role_without_canister_id() {
    let mut plan = sample_plan();
    plan.role_artifacts.clear();
    plan.expected_verifier_readiness.required = false;
    plan.expected_canisters.push(ExpectedCanisterV1 {
        role: "user_hub".to_string(),
        canister_id: None,
        control_class: CanisterControlClassV1::DeploymentControlled,
    });
    let mut inventory = sample_matching_inventory();
    inventory.observed_artifacts.clear();
    inventory.observed_canisters.push(ObservedCanisterV1 {
        canister_id: "user-hub-a".to_string(),
        role: Some("user_hub".to_string()),
        control_class: CanisterControlClassV1::DeploymentControlled,
        controllers: vec!["aaaaa-aa".to_string()],
        module_hash: Some("module".to_string()),
        status: Some("Running".to_string()),
        root_trust_anchor: Some("aaaaa-aa".to_string()),
        canonical_embedded_config_digest: None,
        role_assignment_source: Some("subnet_registry+icp_canister_status".to_string()),
    });
    inventory.observed_canisters.push(ObservedCanisterV1 {
        canister_id: "user-hub-b".to_string(),
        role: Some("user_hub".to_string()),
        control_class: CanisterControlClassV1::DeploymentControlled,
        controllers: vec!["aaaaa-aa".to_string()],
        module_hash: Some("module".to_string()),
        status: Some("Running".to_string()),
        root_trust_anchor: Some("aaaaa-aa".to_string()),
        canonical_embedded_config_digest: None,
        role_assignment_source: Some("subnet_registry+icp_canister_status".to_string()),
    });

    let diff = compare_plan_to_inventory(&plan, &inventory);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Blocked);
    assert!(
        diff.hard_failures
            .iter()
            .any(|finding| finding.code == "canister_role_ambiguous"
                && finding.subject.as_deref() == Some("user_hub"))
    );
    assert!(diff.controller_diff.iter().any(|item| {
        item.category == "canister_role_ambiguous"
            && item.subject == "user_hub"
            && item.observed.as_deref().is_some_and(|observed| {
                observed.contains("user-hub-a") && observed.contains("user-hub-b")
            })
            && item.severity == SafetySeverityV1::HardFailure
    }));
}

#[test]
fn deployment_diff_blocks_expected_canister_role_mismatch() {
    let mut plan = sample_plan();
    plan.role_artifacts.clear();
    plan.expected_verifier_readiness.required = false;
    let mut inventory = sample_matching_inventory();
    inventory.observed_artifacts.clear();
    inventory.observed_canisters[0].role = Some("user_hub".to_string());

    let diff = compare_plan_to_inventory(&plan, &inventory);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Blocked);
    assert!(
        diff.hard_failures
            .iter()
            .any(|finding| finding.code == "canister_role_mismatch"
                && finding.subject.as_deref() == Some("root"))
    );
    assert!(diff.controller_diff.iter().any(|item| {
        item.category == "role_mismatch"
            && item.subject == "root"
            && item.expected.as_deref() == Some("root")
            && item.observed.as_deref() == Some("user_hub")
    }));
}

#[test]
fn deployment_diff_blocks_conflicting_roles_for_same_canister_id() {
    let mut plan = sample_plan();
    plan.role_artifacts.clear();
    plan.expected_verifier_readiness.required = false;
    let mut inventory = sample_matching_inventory();
    inventory.observed_artifacts.clear();
    inventory.observed_canisters.push(ObservedCanisterV1 {
        canister_id: "aaaaa-aa".to_string(),
        role: Some("user_hub".to_string()),
        control_class: CanisterControlClassV1::DeploymentControlled,
        controllers: vec!["aaaaa-aa".to_string()],
        module_hash: Some("module".to_string()),
        status: Some("Running".to_string()),
        root_trust_anchor: Some("aaaaa-aa".to_string()),
        canonical_embedded_config_digest: None,
        role_assignment_source: Some("subnet_registry+icp_canister_status".to_string()),
    });

    let diff = compare_plan_to_inventory(&plan, &inventory);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Blocked);
    assert!(
        diff.hard_failures
            .iter()
            .any(|finding| finding.code == "canister_id_role_conflict"
                && finding.subject.as_deref() == Some("aaaaa-aa"))
    );
    assert!(diff.controller_diff.iter().any(|item| {
        item.category == "canister_id_role_conflict"
            && item.subject == "aaaaa-aa"
            && item.observed.as_deref() == Some("root,user_hub")
    }));
}

#[test]
fn deployment_diff_warns_for_exact_duplicate_canister_observation() {
    let mut plan = sample_plan();
    plan.role_artifacts.clear();
    plan.expected_verifier_readiness.required = false;
    let mut inventory = sample_matching_inventory();
    inventory.observed_artifacts.clear();
    inventory
        .observed_canisters
        .push(inventory.observed_canisters[0].clone());

    let diff = compare_plan_to_inventory(&plan, &inventory);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Warning);
    assert!(diff.hard_failures.is_empty());
    assert!(
        diff.warnings
            .iter()
            .any(|finding| finding.code == "duplicate_canister_observed"
                && finding.subject.as_deref() == Some("aaaaa-aa"))
    );
    assert!(diff.controller_diff.iter().any(|item| {
        item.category == "canister_duplicate"
            && item.subject == "aaaaa-aa"
            && item.expected.as_deref() == Some("root")
            && item.observed.as_deref() == Some("2")
    }));
}

#[test]
fn enriched_registry_status_participates_in_controller_checks() {
    let mut plan = sample_plan();
    plan.role_artifacts.clear();
    plan.expected_verifier_readiness.required = false;
    plan.expected_canisters.push(ExpectedCanisterV1 {
        role: "user_hub".to_string(),
        canister_id: None,
        control_class: CanisterControlClassV1::DeploymentControlled,
    });
    let mut inventory = sample_matching_inventory();
    inventory.observed_artifacts.clear();
    inventory.observed_canisters.push(ObservedCanisterV1 {
        canister_id: "user-hub-id".to_string(),
        role: Some("user_hub".to_string()),
        control_class: CanisterControlClassV1::DeploymentControlled,
        controllers: Vec::new(),
        module_hash: Some("module".to_string()),
        status: Some("Running".to_string()),
        root_trust_anchor: Some("aaaaa-aa".to_string()),
        canonical_embedded_config_digest: None,
        role_assignment_source: Some("subnet_registry+icp_canister_status".to_string()),
    });

    let diff = compare_plan_to_inventory(&plan, &inventory);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Blocked);
    assert!(
        diff.hard_failures
            .iter()
            .any(|finding| finding.code == "expected_controller_missing"
                && finding.subject.as_deref() == Some("user_hub"))
    );
    assert!(
        diff.warnings
            .iter()
            .all(|finding| finding.code != "controllers_unobserved")
    );
}

#[test]
fn deployment_diff_blocks_missing_expected_pool_canister() {
    let mut plan = sample_plan();
    plan.expected_pool.push(ExpectedPoolCanisterV1 {
        pool: "user_shards".to_string(),
        canister_id: Some("pool-canister".to_string()),
        role: Some("user_shard".to_string()),
    });
    let inventory = sample_matching_inventory();

    let diff = compare_plan_to_inventory(&plan, &inventory);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Blocked);
    assert!(
        diff.hard_failures
            .iter()
            .any(|finding| finding.code == "pool_canister_missing")
    );
    assert!(diff.pool_diff.iter().any(|item| {
        item.category == "pool_canister"
            && item.subject == "user_shards:user_shard"
            && item.expected.as_deref() == Some("pool-canister")
            && item.observed.is_none()
            && item.severity == SafetySeverityV1::HardFailure
    }));
}

#[test]
fn deployment_diff_blocks_conflicting_planned_pool_subject() {
    let mut plan = sample_plan();
    plan.expected_pool.push(ExpectedPoolCanisterV1 {
        pool: "user_shards".to_string(),
        canister_id: Some("pool-a".to_string()),
        role: Some("user_shard".to_string()),
    });
    plan.expected_pool.push(ExpectedPoolCanisterV1 {
        pool: "user_shards".to_string(),
        canister_id: Some("pool-b".to_string()),
        role: Some("user_shard".to_string()),
    });

    let diff = compare_plan_to_inventory(&plan, &sample_matching_inventory());

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Blocked);
    assert!(
        diff.hard_failures
            .iter()
            .any(|finding| finding.code == "planned_pool_conflict"
                && finding.subject.as_deref() == Some("user_shards:user_shard"))
    );
    assert!(diff.pool_diff.iter().any(|item| {
        item.category == "planned_pool_conflict"
            && item.subject == "user_shards:user_shard"
            && item
                .observed
                .as_deref()
                .is_some_and(|observed| observed.contains("pool-a") && observed.contains("pool-b"))
            && item.severity == SafetySeverityV1::HardFailure
    }));
}

#[test]
fn deployment_diff_blocks_conflicting_planned_pool_id() {
    let mut plan = sample_plan();
    plan.expected_pool.push(ExpectedPoolCanisterV1 {
        pool: "user_shards".to_string(),
        canister_id: Some("pool-canister".to_string()),
        role: Some("user_shard".to_string()),
    });
    plan.expected_pool.push(ExpectedPoolCanisterV1 {
        pool: "directory".to_string(),
        canister_id: Some("pool-canister".to_string()),
        role: Some("project_instance".to_string()),
    });

    let diff = compare_plan_to_inventory(&plan, &sample_matching_inventory());

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Blocked);
    assert!(
        diff.hard_failures
            .iter()
            .any(|finding| finding.code == "planned_pool_id_conflict"
                && finding.subject.as_deref() == Some("pool-canister"))
    );
    assert!(diff.pool_diff.iter().any(|item| {
        item.category == "planned_pool_id_conflict"
            && item.subject == "pool-canister"
            && item.observed.as_deref().is_some_and(|observed| {
                observed.contains("directory:project_instance")
                    && observed.contains("user_shards:user_shard")
            })
            && item.severity == SafetySeverityV1::HardFailure
    }));
}

#[test]
fn deployment_diff_warns_for_duplicate_identical_planned_pool() {
    let mut plan = sample_plan();
    let planned = ExpectedPoolCanisterV1 {
        pool: "user_shards".to_string(),
        canister_id: Some("pool-canister".to_string()),
        role: Some("user_shard".to_string()),
    };
    plan.expected_pool.push(planned.clone());
    plan.expected_pool.push(planned);
    let mut inventory = sample_matching_inventory();
    inventory.observed_pool.push(ObservedPoolCanisterV1 {
        pool: "user_shards".to_string(),
        canister_id: "pool-canister".to_string(),
        role: Some("user_shard".to_string()),
        control_class: CanisterControlClassV1::CanicManagedPool,
    });

    let diff = compare_plan_to_inventory(&plan, &inventory);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Warning);
    assert!(diff.hard_failures.is_empty());
    assert!(
        diff.warnings
            .iter()
            .any(|finding| finding.code == "duplicate_planned_pool"
                && finding.subject.as_deref() == Some("user_shards:user_shard"))
    );
    assert!(diff.pool_diff.iter().any(|item| {
        item.category == "planned_pool_duplicate"
            && item.subject == "user_shards:user_shard"
            && item.observed.as_deref() == Some("2")
            && item.severity == SafetySeverityV1::Warning
    }));
}

#[test]
fn deployment_diff_blocks_unsafe_pool_control_class() {
    let mut plan = sample_plan();
    plan.expected_pool.push(ExpectedPoolCanisterV1 {
        pool: "user_shards".to_string(),
        canister_id: Some("pool-canister".to_string()),
        role: Some("user_shard".to_string()),
    });
    let mut inventory = sample_matching_inventory();
    inventory.observed_pool.push(ObservedPoolCanisterV1 {
        pool: "user_shards".to_string(),
        canister_id: "pool-canister".to_string(),
        role: Some("user_shard".to_string()),
        control_class: CanisterControlClassV1::UserControlled,
    });

    let diff = compare_plan_to_inventory(&plan, &inventory);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Blocked);
    assert!(
        diff.hard_failures
            .iter()
            .any(|finding| finding.code == "unsafe_pool_control_class")
    );
    assert!(diff.pool_diff.iter().any(|item| {
        item.category == "pool_control_class"
            && item.subject == "user_shards:user_shard"
            && item.expected.as_deref() == Some("CanicManagedPool")
            && item.observed.as_deref() == Some("UserControlled")
    }));
}

#[test]
fn deployment_diff_blocks_pool_canister_id_mismatch() {
    let mut plan = sample_plan();
    plan.expected_pool.push(ExpectedPoolCanisterV1 {
        pool: "user_shards".to_string(),
        canister_id: Some("planned-pool-canister".to_string()),
        role: Some("user_shard".to_string()),
    });
    let mut inventory = sample_matching_inventory();
    inventory.observed_pool.push(ObservedPoolCanisterV1 {
        pool: "user_shards".to_string(),
        canister_id: "observed-pool-canister".to_string(),
        role: Some("user_shard".to_string()),
        control_class: CanisterControlClassV1::CanicManagedPool,
    });

    let diff = compare_plan_to_inventory(&plan, &inventory);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Blocked);
    assert!(
        diff.hard_failures
            .iter()
            .any(|finding| finding.code == "pool_canister_id_mismatch")
    );
    assert!(diff.pool_diff.iter().any(|item| {
        item.category == "pool_canister_id"
            && item.subject == "user_shards:user_shard"
            && item.expected.as_deref() == Some("planned-pool-canister")
            && item.observed.as_deref() == Some("observed-pool-canister")
    }));
    assert!(
        diff.warnings
            .iter()
            .all(|finding| finding.code != "extra_pool_canister_observed")
    );
}

#[test]
fn deployment_diff_blocks_conflicting_pool_identities_for_same_canister_id() {
    let mut plan = sample_plan();
    plan.expected_pool.push(ExpectedPoolCanisterV1 {
        pool: "user_shards".to_string(),
        canister_id: Some("pool-canister".to_string()),
        role: Some("user_shard".to_string()),
    });
    let mut inventory = sample_matching_inventory();
    inventory.observed_pool.push(ObservedPoolCanisterV1 {
        pool: "user_shards".to_string(),
        canister_id: "pool-canister".to_string(),
        role: Some("user_shard".to_string()),
        control_class: CanisterControlClassV1::CanicManagedPool,
    });
    inventory.observed_pool.push(ObservedPoolCanisterV1 {
        pool: "directory".to_string(),
        canister_id: "pool-canister".to_string(),
        role: Some("project_instance".to_string()),
        control_class: CanisterControlClassV1::CanicManagedPool,
    });

    let diff = compare_plan_to_inventory(&plan, &inventory);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Blocked);
    assert!(
        diff.hard_failures
            .iter()
            .any(|finding| finding.code == "pool_canister_id_conflict"
                && finding.subject.as_deref() == Some("pool-canister"))
    );
    assert!(diff.pool_diff.iter().any(|item| {
        item.category == "pool_canister_id_conflict"
            && item.subject == "pool-canister"
            && item.observed.as_deref().is_some_and(|observed| {
                observed.contains("directory:project_instance")
                    && observed.contains("user_shards:user_shard")
            })
            && item.severity == SafetySeverityV1::HardFailure
    }));
}

#[test]
fn deployment_diff_warns_for_exact_duplicate_pool_observation() {
    let mut plan = sample_plan();
    plan.expected_pool.push(ExpectedPoolCanisterV1 {
        pool: "user_shards".to_string(),
        canister_id: Some("pool-canister".to_string()),
        role: Some("user_shard".to_string()),
    });
    let mut inventory = sample_matching_inventory();
    let observed = ObservedPoolCanisterV1 {
        pool: "user_shards".to_string(),
        canister_id: "pool-canister".to_string(),
        role: Some("user_shard".to_string()),
        control_class: CanisterControlClassV1::CanicManagedPool,
    };
    inventory.observed_pool.push(observed.clone());
    inventory.observed_pool.push(observed);

    let diff = compare_plan_to_inventory(&plan, &inventory);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Warning);
    assert!(diff.hard_failures.is_empty());
    assert!(
        diff.warnings
            .iter()
            .any(|finding| finding.code == "duplicate_pool_canister_observed"
                && finding.subject.as_deref() == Some("pool-canister"))
    );
    assert!(diff.pool_diff.iter().any(|item| {
        item.category == "pool_canister_duplicate"
            && item.subject == "pool-canister"
            && item.expected.as_deref() == Some("user_shards:user_shard")
            && item.observed.as_deref() == Some("2")
            && item.severity == SafetySeverityV1::Warning
    }));
}

#[test]
fn deployment_diff_blocks_cross_surface_role_conflict_for_same_canister_id() {
    let mut plan = sample_plan();
    plan.expected_pool.push(ExpectedPoolCanisterV1 {
        pool: "user_shards".to_string(),
        canister_id: Some("shared-canister".to_string()),
        role: Some("user_shard".to_string()),
    });
    let mut inventory = sample_matching_inventory();
    inventory.observed_canisters.push(ObservedCanisterV1 {
        canister_id: "shared-canister".to_string(),
        role: Some("user_hub".to_string()),
        control_class: CanisterControlClassV1::DeploymentControlled,
        controllers: vec!["aaaaa-aa".to_string()],
        module_hash: Some("module".to_string()),
        status: Some("Running".to_string()),
        root_trust_anchor: Some("aaaaa-aa".to_string()),
        canonical_embedded_config_digest: None,
        role_assignment_source: Some("subnet_registry+icp_canister_status".to_string()),
    });
    inventory.observed_pool.push(ObservedPoolCanisterV1 {
        pool: "user_shards".to_string(),
        canister_id: "shared-canister".to_string(),
        role: Some("user_shard".to_string()),
        control_class: CanisterControlClassV1::CanicManagedPool,
    });

    let diff = compare_plan_to_inventory(&plan, &inventory);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Blocked);
    assert!(
        diff.hard_failures
            .iter()
            .any(|finding| finding.code == "canister_pool_role_conflict"
                && finding.subject.as_deref() == Some("shared-canister"))
    );
    assert!(diff.pool_diff.iter().any(|item| {
        item.category == "canister_pool_role_conflict"
            && item.subject == "shared-canister"
            && item.observed.as_deref().is_some_and(|observed| {
                observed.contains("canister=user_hub")
                    && observed.contains("pool=user_shards:user_shard")
            })
            && item.severity == SafetySeverityV1::HardFailure
    }));
}

#[test]
fn deployment_diff_warns_for_extra_pool_canister() {
    let plan = sample_plan();
    let mut inventory = sample_matching_inventory();
    inventory.observed_pool.push(ObservedPoolCanisterV1 {
        pool: "directory".to_string(),
        canister_id: "extra-pool-canister".to_string(),
        role: Some("project_instance".to_string()),
        control_class: CanisterControlClassV1::CanicManagedPool,
    });

    let diff = compare_plan_to_inventory(&plan, &inventory);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Warning);
    assert!(diff.hard_failures.is_empty());
    assert!(
        diff.warnings
            .iter()
            .any(|finding| finding.code == "extra_pool_canister_observed")
    );
    assert!(diff.pool_diff.iter().any(|item| {
        item.category == "pool_extra"
            && item.subject == "directory:project_instance"
            && item.observed.as_deref() == Some("extra-pool-canister")
            && item.severity == SafetySeverityV1::Warning
    }));
}

#[test]
fn deployment_diff_blocks_stale_verifier_role_epoch() {
    let plan = sample_plan();
    let mut inventory = sample_matching_inventory();
    inventory.observed_verifier_readiness.role_epochs = vec![RoleEpochObservationV1 {
        role: "root".to_string(),
        observed_epoch: Some(0),
        status: ObservationStatusV1::Observed,
    }];

    let diff = compare_plan_to_inventory(&plan, &inventory);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Blocked);
    assert!(
        diff.hard_failures
            .iter()
            .any(|finding| finding.code == "verifier_role_epoch_stale")
    );
    assert!(diff.verifier_readiness_diff.iter().any(|item| {
        item.category == "verifier_role_epoch"
            && item.subject == "root"
            && item.expected.as_deref() == Some("1")
            && item.observed.as_deref() == Some("0")
            && item.severity == SafetySeverityV1::HardFailure
    }));
}

#[test]
fn deployment_diff_warns_when_required_verifier_role_epoch_is_unobserved() {
    let plan = sample_plan();
    let mut inventory = sample_matching_inventory();
    inventory.observed_verifier_readiness.role_epochs.clear();

    let diff = compare_plan_to_inventory(&plan, &inventory);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Warning);
    assert!(diff.hard_failures.is_empty());
    assert!(
        diff.warnings
            .iter()
            .any(|finding| finding.code == "verifier_role_epoch_unobserved")
    );
    assert!(diff.verifier_readiness_diff.iter().any(|item| {
        item.category == "verifier_role_epoch"
            && item.subject == "root"
            && item.expected.as_deref() == Some("1")
            && item.observed.as_deref() == Some("not_observed")
            && item.severity == SafetySeverityV1::Warning
    }));
}

#[test]
fn deployment_diff_blocks_conflicting_verifier_role_epoch_observations() {
    let plan = sample_plan();
    let mut inventory = sample_matching_inventory();
    inventory.observed_verifier_readiness.role_epochs = vec![
        RoleEpochObservationV1 {
            role: "root".to_string(),
            observed_epoch: Some(1),
            status: ObservationStatusV1::Observed,
        },
        RoleEpochObservationV1 {
            role: "root".to_string(),
            observed_epoch: Some(0),
            status: ObservationStatusV1::Observed,
        },
    ];

    let diff = compare_plan_to_inventory(&plan, &inventory);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Blocked);
    assert!(
        diff.hard_failures
            .iter()
            .any(|finding| finding.code == "verifier_role_epoch_conflict"
                && finding.subject.as_deref() == Some("root"))
    );
    assert!(diff.verifier_readiness_diff.iter().any(|item| {
        item.category == "verifier_role_epoch_conflict"
            && item.subject == "root"
            && item.observed.as_deref().is_some_and(|observed| {
                observed.contains("epoch=1") && observed.contains("epoch=0")
            })
            && item.severity == SafetySeverityV1::HardFailure
    }));
}

#[test]
fn deployment_diff_warns_for_duplicate_identical_verifier_role_epoch_observation() {
    let mut inventory = sample_matching_inventory();
    inventory
        .observed_verifier_readiness
        .role_epochs
        .push(inventory.observed_verifier_readiness.role_epochs[0].clone());

    let diff = compare_plan_to_inventory(&sample_plan(), &inventory);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Warning);
    assert!(diff.hard_failures.is_empty());
    assert!(diff.warnings.iter().any(|finding| finding.code
        == "duplicate_verifier_role_epoch_observed"
        && finding.subject.as_deref() == Some("root")));
    assert!(diff.verifier_readiness_diff.iter().any(|item| {
        item.category == "verifier_role_epoch_duplicate"
            && item.subject == "root"
            && item.observed.as_deref() == Some("2")
            && item.severity == SafetySeverityV1::Warning
    }));
}

#[test]
fn deployment_diff_blocks_conflicting_planned_verifier_role_epoch() {
    let mut plan = sample_plan();
    plan.expected_verifier_readiness
        .expected_role_epochs
        .push(RoleEpochExpectationV1 {
            role: "root".to_string(),
            minimum_epoch: 2,
        });

    let diff = compare_plan_to_inventory(&plan, &sample_matching_inventory());

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Blocked);
    assert!(diff.hard_failures.iter().any(|finding| finding.code
        == "planned_verifier_role_epoch_conflict"
        && finding.subject.as_deref() == Some("root")));
    assert!(diff.verifier_readiness_diff.iter().any(|item| {
        item.category == "planned_verifier_role_epoch_conflict"
            && item.subject == "root"
            && item
                .observed
                .as_deref()
                .is_some_and(|observed| observed.contains('1') && observed.contains('2'))
            && item.severity == SafetySeverityV1::HardFailure
    }));
}

#[test]
fn deployment_diff_warns_for_duplicate_identical_planned_verifier_role_epoch() {
    let mut plan = sample_plan();
    plan.expected_verifier_readiness
        .expected_role_epochs
        .push(plan.expected_verifier_readiness.expected_role_epochs[0].clone());

    let diff = compare_plan_to_inventory(&plan, &sample_matching_inventory());

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Warning);
    assert!(diff.hard_failures.is_empty());
    assert!(diff.warnings.iter().any(|finding| finding.code
        == "duplicate_planned_verifier_role_epoch"
        && finding.subject.as_deref() == Some("root")));
    assert!(diff.verifier_readiness_diff.iter().any(|item| {
        item.category == "planned_verifier_role_epoch_duplicate"
            && item.subject == "root"
            && item.observed.as_deref() == Some("2")
            && item.severity == SafetySeverityV1::Warning
    }));
}

#[test]
fn deployment_diff_is_safe_when_checked_facts_match() {
    let mut plan = sample_plan();
    plan.expected_verifier_readiness.required = false;
    let inventory = DeploymentInventoryV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        inventory_id: "inventory-1".to_string(),
        observed_at: "2026-05-21T00:00:00Z".to_string(),
        observed_identity: Some(sample_identity()),
        local_config: LocalDeploymentConfigV1 {
            config_path: Some("icp.yml".to_string()),
            raw_config_sha256: None,
            canonical_embedded_config_sha256: Some("runtime".to_string()),
        },
        observed_canisters: vec![ObservedCanisterV1 {
            canister_id: "aaaaa-aa".to_string(),
            role: Some("root".to_string()),
            control_class: CanisterControlClassV1::DeploymentControlled,
            controllers: vec!["aaaaa-aa".to_string()],
            module_hash: Some("module".to_string()),
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
    let report = safety_report_from_diff("report-1", None, &diff);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Safe);
    assert!(
        diff.artifact_diff
            .iter()
            .any(|item| item.category == "artifact_file_sha256"
                && item.severity == SafetySeverityV1::Info)
    );
    assert!(diff.hard_failures.is_empty());
    assert!(diff.warnings.is_empty());
    assert_eq!(report.status, SafetyStatusV1::Safe);
    assert!(report.next_actions.is_empty());
}

#[test]
fn authority_reconciliation_reports_already_correct_controller_state() {
    let check = sample_check(sample_plan(), sample_matching_inventory());

    let plan = build_authority_reconciliation_plan(&check);

    assert_eq!(plan.plan_id, "plan-local-root");
    assert_eq!(plan.inventory_id, "inventory-1");
    assert_eq!(plan.authority_profile_hash.as_deref(), Some("authority"));
    assert!(plan.hard_failures.is_empty());
    assert!(plan.external_actions_required.is_empty());
    assert_eq!(plan.canister_actions.len(), 1);
    assert_eq!(
        plan.canister_actions[0].state,
        AuthorityReconciliationStateV1::AlreadyCorrect
    );
    assert_eq!(plan.canister_actions[0].action, AuthorityActionV1::None);
    assert!(!plan.canister_actions[0].can_apply);
}

#[test]
fn authority_report_summarizes_safe_reconciliation_plan() {
    let check = sample_check(sample_plan(), sample_matching_inventory());
    let plan = build_authority_reconciliation_plan(&check);

    let report = authority_report_from_plan("authority-report-1", &plan);

    assert_eq!(report.status, SafetyStatusV1::Safe);
    assert_eq!(report.reconciliation_plan_id, "plan-local-root");
    assert_eq!(report.check_id, None);
    assert_eq!(report.inventory_id, "inventory-1");
    assert_eq!(report.authority_profile_hash.as_deref(), Some("authority"));
    assert_eq!(report.counts.already_correct, 1);
    assert_eq!(report.counts.can_apply_automatically, 0);
    assert_eq!(report.counts.requires_external_action, 0);
    assert_eq!(report.counts.unsafe_blocked, 0);
    assert_eq!(report.counts.unknown, 0);
    assert_eq!(report.counts.hard_failures, 0);
    assert_eq!(
        report.apply_readiness,
        AuthorityApplyReadinessV1 {
            can_apply_automatically: false,
            automatic_action_count: 0,
            blockers: Vec::new(),
        }
    );
    assert_eq!(
        report.action_counts,
        vec![AuthorityActionCountV1 {
            action: AuthorityActionV1::None,
            count: 1,
        }]
    );
    assert_eq!(
        report.control_class_counts,
        vec![AuthorityControlClassCountV1 {
            control_class: CanisterControlClassV1::DeploymentControlled,
            count: 1,
        }]
    );
    assert!(report.observation_gaps.is_empty());
    assert!(report.automatic_actions.is_empty());
    assert!(report.external_actions_required.is_empty());
    assert!(report.next_actions.is_empty());
}

#[test]
fn authority_report_can_preserve_source_check_id() {
    let check = sample_check(sample_plan(), sample_matching_inventory());
    let plan = build_authority_reconciliation_plan(&check);

    let report =
        authority_report_from_plan_with_check_id("authority-report-1", Some(check.check_id), &plan);

    assert_eq!(report.check_id.as_deref(), Some("check-1"));
    assert_eq!(report.reconciliation_plan_id, "plan-local-root");
    assert_eq!(report.inventory_id, "inventory-1");
}

#[test]
fn authority_report_from_check_preserves_source_provenance() {
    let check = sample_check(sample_plan(), sample_matching_inventory());

    let report = authority_report_from_check("authority-report-1", &check);

    assert_eq!(report.check_id.as_deref(), Some("check-1"));
    assert_eq!(report.reconciliation_plan_id, "plan-local-root");
    assert_eq!(report.inventory_id, "inventory-1");
    assert_eq!(report.authority_profile_hash.as_deref(), Some("authority"));
    assert_eq!(report.counts.already_correct, 1);
}

#[test]
fn authority_report_from_check_with_local_id_uses_deployment_identity() {
    let check = sample_check(sample_plan(), sample_matching_inventory());

    let report = authority_report_from_check_with_local_id(&check);

    assert_eq!(report.report_id, "local:local:local-root:authority-report");
    assert_eq!(report.check_id.as_deref(), Some("check-1"));
    assert_eq!(report.reconciliation_plan_id, "plan-local-root");
    assert_eq!(report.inventory_id, "inventory-1");
}

#[test]
fn authority_dry_run_evidence_from_check_with_local_ids_uses_deployment_identity() {
    let check = sample_check(sample_plan(), sample_matching_inventory());

    let evidence =
        authority_dry_run_evidence_from_check_with_local_ids(&check, "2026-05-23T00:00:01Z")
            .expect("build authority evidence");

    assert_eq!(
        evidence.evidence_id,
        "local:local:local-root:authority-evidence"
    );
    assert_eq!(evidence.check_id, "check-1");
    assert_eq!(
        evidence.authority_report.report_id,
        "local:local:local-root:authority-report"
    );
    assert_eq!(
        evidence.authority_receipt.operation_id,
        "local:local:local-root:authority-dry-run-receipt"
    );
    assert_eq!(
        evidence.authority_receipt.authority_report_id,
        evidence.authority_report.report_id
    );
    assert_eq!(evidence.generated_at, "2026-05-23T00:00:01Z");
    assert_eq!(
        evidence.authority_receipt.finished_at.as_deref(),
        Some("2026-05-23T00:00:01Z")
    );
}

#[test]
fn authority_dry_run_receipt_from_check_with_local_id_uses_deployment_identity() {
    let check = sample_check(sample_plan(), sample_matching_inventory());

    let receipt =
        authority_dry_run_receipt_from_check_with_local_id(&check, "2026-05-23T00:00:01Z")
            .expect("build authority receipt");

    assert_eq!(
        receipt.operation_id,
        "local:local:local-root:authority-dry-run-receipt"
    );
    assert_eq!(receipt.check_id.as_deref(), Some("check-1"));
    assert_eq!(receipt.reconciliation_plan_id, "plan-local-root");
    assert_eq!(
        receipt.authority_report_id,
        "local:local:local-root:authority-report"
    );
    assert_eq!(receipt.inventory_id, "inventory-1");
    assert_eq!(receipt.authority_profile_hash.as_deref(), Some("authority"));
    assert_eq!(receipt.finished_at.as_deref(), Some("2026-05-23T00:00:01Z"));
    assert!(receipt.attempted_actions.is_empty());
}

#[test]
fn authority_dry_run_receipt_from_check_preserves_explicit_report_id() {
    let check = sample_check(sample_plan(), sample_matching_inventory());

    let receipt = authority_dry_run_receipt_from_check(
        &check,
        "authority-report-explicit",
        "authority-dry-run-explicit",
        "2026-05-23T00:00:00Z",
        Some("2026-05-23T00:00:01Z".to_string()),
    )
    .expect("build authority receipt");

    assert_eq!(receipt.operation_id, "authority-dry-run-explicit");
    assert_eq!(receipt.authority_report_id, "authority-report-explicit");
    assert_eq!(receipt.check_id.as_deref(), Some("check-1"));
    assert_eq!(receipt.reconciliation_plan_id, "plan-local-root");
}

#[test]
fn authority_text_renders_plan_and_report_summaries() {
    let mut source_plan = sample_plan();
    source_plan.authority_profile.expected_controllers =
        vec!["aaaaa-aa".to_string(), "ops-principal".to_string()];
    let check = sample_check(source_plan, sample_matching_inventory());
    let plan = build_authority_reconciliation_plan(&check);
    let report =
        authority_report_from_plan_with_check_id("authority-report-1", Some(check.check_id), &plan);

    let plan_text = authority_plan_text(&plan);
    let report_text = authority_report_text(&report);

    assert!(plan_text.contains("Authority reconciliation plan"));
    assert!(plan_text.contains("mode: dry_run"));
    assert!(plan_text.contains("plan_id: plan-local-root"));
    assert!(plan_text.contains("root (aaaaa-aa) CanApplyAutomatically/AddControllers"));
    assert!(plan_text.contains("[add=ops-principal; remove=none]"));
    assert!(report_text.contains("Authority reconciliation report"));
    assert!(report_text.contains("mode: dry_run"));
    assert!(report_text.contains("check_id: check-1"));
    assert!(report_text.contains("status: safe"));
    assert!(report_text.contains("[add=ops-principal; remove=none]"));
}

#[test]
fn authority_text_renders_evidence_and_receipt_details() {
    let mut source_plan = sample_plan();
    source_plan.authority_profile.staging_controllers = vec!["aaaaa-aa".to_string()];
    let check = sample_check(source_plan, sample_matching_inventory());
    let plan = build_authority_reconciliation_plan(&check);
    let report = authority_report_from_plan_with_check_id(
        "authority-report-1",
        Some(check.check_id.clone()),
        &plan,
    );
    let receipt = authority_dry_run_receipt_from_plan(
        &plan,
        &report,
        Some(check.check_id.clone()),
        "authority-dry-run-1",
        "2026-05-23T00:00:00Z",
        Some("2026-05-23T00:00:01Z".to_string()),
    )
    .expect("build receipt");
    let evidence = AuthorityDryRunEvidenceV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        evidence_id: "authority-evidence-1".to_string(),
        check_id: check.check_id,
        generated_at: "2026-05-23T00:00:00Z".to_string(),
        reconciliation_plan: plan,
        authority_report: report,
        authority_receipt: receipt,
    };

    let evidence_text = authority_evidence_text(&evidence);
    let receipt_text = authority_receipt_text(&evidence.authority_receipt);

    assert!(evidence_text.contains("Authority dry-run evidence"));
    assert!(evidence_text.contains("mode: dry_run"));
    assert!(evidence_text.contains("evidence_id: authority-evidence-1"));
    assert!(evidence_text.contains("generated_at: 2026-05-23T00:00:00Z"));
    assert!(evidence_text.contains("controller_mutation: none_attempted"));
    assert!(evidence_text.contains("verified_controller_observations:"));
    assert!(
        evidence_text
            .contains("aaaaa-aa AlreadyCorrect/None: observed=[aaaaa-aa] desired=[aaaaa-aa]")
    );
    assert!(evidence_text.contains(
        "[authority_profile_overlap] aaaaa-aa: staging authority principal aaaaa-aa overlaps"
    ));
    assert!(receipt_text.contains("Authority dry-run receipt"));
    assert!(receipt_text.contains("mode: dry_run"));
    assert!(receipt_text.contains("operation_id: authority-dry-run-1"));
    assert!(receipt_text.contains("controller_mutation: none_attempted"));
    assert!(receipt_text.contains("verified_controller_observations:"));
    assert!(
        receipt_text
            .contains("aaaaa-aa AlreadyCorrect/None: observed=[aaaaa-aa] desired=[aaaaa-aa]")
    );
    assert!(receipt_text.contains(
        "[authority_profile_overlap] aaaaa-aa: staging authority principal aaaaa-aa overlaps"
    ));
}

#[test]
fn authority_receipt_rejects_mismatched_report_provenance() {
    let check = sample_check(sample_plan(), sample_matching_inventory());
    let reconciliation = build_authority_reconciliation_plan(&check);
    let mut report = authority_report_from_plan_with_check_id(
        "authority-report-1",
        Some(check.check_id.clone()),
        &reconciliation,
    );
    report.inventory_id = "other-inventory".to_string();

    let err = authority_dry_run_receipt_from_plan(
        &reconciliation,
        &report,
        Some(check.check_id),
        "authority-dry-run-1",
        "2026-05-23T00:00:00Z",
        Some("2026-05-23T00:00:01Z".to_string()),
    )
    .expect_err("mismatched report inventory should fail receipt construction");

    assert!(matches!(
        err,
        AuthorityEvidenceError::PlanReportMismatch {
            field: "inventory_id",
            ..
        }
    ));
}

#[test]
fn authority_receipt_rejects_mismatched_report_content() {
    let mut plan = sample_plan();
    plan.authority_profile.expected_controllers =
        vec!["aaaaa-aa".to_string(), "ops-principal".to_string()];
    let check = sample_check(plan, sample_matching_inventory());
    let reconciliation = build_authority_reconciliation_plan(&check);
    let mut report = authority_report_from_plan_with_check_id(
        "authority-report-1",
        Some(check.check_id.clone()),
        &reconciliation,
    );
    report.automatic_actions.clear();

    let err = authority_dry_run_receipt_from_plan(
        &reconciliation,
        &report,
        Some(check.check_id),
        "authority-dry-run-1",
        "2026-05-23T00:00:00Z",
        Some("2026-05-23T00:00:01Z".to_string()),
    )
    .expect_err("mismatched report content should fail receipt construction");

    assert!(matches!(
        err,
        AuthorityEvidenceError::PlanReportContentMismatch {
            field: "automatic_actions",
        }
    ));
}

#[test]
fn authority_receipt_rejects_mismatched_check_id() {
    let check = sample_check(sample_plan(), sample_matching_inventory());
    let reconciliation = build_authority_reconciliation_plan(&check);
    let report = authority_report_from_plan_with_check_id(
        "authority-report-1",
        Some(check.check_id),
        &reconciliation,
    );

    let err = authority_dry_run_receipt_from_plan(
        &reconciliation,
        &report,
        Some("other-check".to_string()),
        "authority-dry-run-1",
        "2026-05-23T00:00:00Z",
        Some("2026-05-23T00:00:01Z".to_string()),
    )
    .expect_err("mismatched check id should fail receipt construction");

    assert!(matches!(
        err,
        AuthorityEvidenceError::CheckIdMismatch { .. }
    ));
}

#[test]
fn authority_receipt_rejects_unsupported_source_schema_version() {
    let check = sample_check(sample_plan(), sample_matching_inventory());
    let mut reconciliation = build_authority_reconciliation_plan(&check);
    let report = authority_report_from_plan_with_check_id(
        "authority-report-1",
        Some(check.check_id.clone()),
        &reconciliation,
    );
    reconciliation.schema_version = DEPLOYMENT_TRUTH_SCHEMA_VERSION + 1;

    let err = authority_dry_run_receipt_from_plan(
        &reconciliation,
        &report,
        Some(check.check_id),
        "authority-dry-run-1",
        "2026-05-23T00:00:00Z",
        Some("2026-05-23T00:00:01Z".to_string()),
    )
    .expect_err("unsupported plan schema should fail receipt construction");

    assert!(matches!(
        err,
        AuthorityEvidenceError::SchemaVersionMismatch {
            component: "plan",
            expected: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
            found
        } if found == DEPLOYMENT_TRUTH_SCHEMA_VERSION + 1
    ));
}

#[test]
fn authority_receipt_rejects_blank_receipt_identity_inputs() {
    let check = sample_check(sample_plan(), sample_matching_inventory());
    let reconciliation = build_authority_reconciliation_plan(&check);
    let report = authority_report_from_plan_with_check_id(
        "authority-report-1",
        Some(check.check_id.clone()),
        &reconciliation,
    );

    let err = authority_dry_run_receipt_from_plan(
        &reconciliation,
        &report,
        Some(check.check_id),
        " ",
        "2026-05-23T00:00:00Z",
        Some("2026-05-23T00:00:01Z".to_string()),
    )
    .expect_err("blank receipt operation id should fail receipt construction");

    assert!(matches!(
        err,
        AuthorityEvidenceError::MissingRequiredField {
            field: "receipt.operation_id",
        }
    ));
}

#[test]
fn authority_receipt_rejects_missing_report_check_provenance() {
    let check = sample_check(sample_plan(), sample_matching_inventory());
    let reconciliation = build_authority_reconciliation_plan(&check);
    let mut report = authority_report_from_plan_with_check_id(
        "authority-report-1",
        Some(check.check_id.clone()),
        &reconciliation,
    );
    report.check_id = None;

    let err = authority_dry_run_receipt_from_plan(
        &reconciliation,
        &report,
        Some(check.check_id),
        "authority-dry-run-1",
        "2026-05-23T00:00:00Z",
        Some("2026-05-23T00:00:01Z".to_string()),
    )
    .expect_err("receipt construction should require report check provenance");

    assert!(matches!(
        err,
        AuthorityEvidenceError::MissingRequiredField {
            field: "report.check_id",
        }
    ));
}

#[test]
fn authority_receipt_rejects_missing_finished_at() {
    let check = sample_check(sample_plan(), sample_matching_inventory());
    let reconciliation = build_authority_reconciliation_plan(&check);
    let report = authority_report_from_plan_with_check_id(
        "authority-report-1",
        Some(check.check_id.clone()),
        &reconciliation,
    );

    let err = authority_dry_run_receipt_from_plan(
        &reconciliation,
        &report,
        Some(check.check_id),
        "authority-dry-run-1",
        "2026-05-23T00:00:00Z",
        None,
    )
    .expect_err("completed dry-run receipt should require finished_at");

    assert!(matches!(
        err,
        AuthorityEvidenceError::MissingRequiredField {
            field: "receipt.finished_at",
        }
    ));
}

#[test]
fn authority_receipt_rejects_finished_before_started() {
    let check = sample_check(sample_plan(), sample_matching_inventory());
    let reconciliation = build_authority_reconciliation_plan(&check);
    let report = authority_report_from_plan_with_check_id(
        "authority-report-1",
        Some(check.check_id.clone()),
        &reconciliation,
    );

    let err = authority_dry_run_receipt_from_plan(
        &reconciliation,
        &report,
        Some(check.check_id),
        "authority-dry-run-1",
        "2026-05-23T00:00:02Z",
        Some("2026-05-23T00:00:01Z".to_string()),
    )
    .expect_err("receipt construction should reject invalid timestamp order");

    assert!(matches!(
        err,
        AuthorityEvidenceError::DryRunReceiptTimestampOrder {
            field: "receipt.started_at",
            other_field: "receipt.finished_at",
            ..
        }
    ));
}

#[test]
fn authority_dry_run_evidence_rejects_mismatched_nested_check_id() {
    let mut evidence = sample_authority_evidence();
    evidence.check_id = "other-check".to_string();

    let err = validate_authority_dry_run_evidence(&evidence)
        .expect_err("mismatched nested check id should fail evidence validation");

    assert!(matches!(
        err,
        AuthorityEvidenceError::EvidenceCheckIdMismatch {
            component: "report",
            ..
        }
    ));
}

#[test]
fn authority_dry_run_evidence_rejects_unsupported_schema_version() {
    let mut evidence = sample_authority_evidence();
    evidence.schema_version = DEPLOYMENT_TRUTH_SCHEMA_VERSION + 1;

    let err = validate_authority_dry_run_evidence(&evidence)
        .expect_err("unsupported evidence schema should fail validation");

    assert!(matches!(
        err,
        AuthorityEvidenceError::SchemaVersionMismatch {
            component: "evidence",
            expected: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
            found
        } if found == DEPLOYMENT_TRUTH_SCHEMA_VERSION + 1
    ));
}

#[test]
fn authority_dry_run_evidence_rejects_nested_schema_version_drift() {
    let mut evidence = sample_authority_evidence();
    evidence.authority_report.schema_version = DEPLOYMENT_TRUTH_SCHEMA_VERSION + 1;

    let err = validate_authority_dry_run_evidence(&evidence)
        .expect_err("nested schema drift should fail evidence validation");

    assert!(matches!(
        err,
        AuthorityEvidenceError::SchemaVersionMismatch {
            component: "report",
            ..
        }
    ));
}

#[test]
fn authority_dry_run_evidence_rejects_blank_required_identity() {
    let mut evidence = sample_authority_evidence();
    evidence.evidence_id = "  ".to_string();

    let err = validate_authority_dry_run_evidence(&evidence)
        .expect_err("blank evidence identity should fail validation");

    assert!(matches!(
        err,
        AuthorityEvidenceError::MissingRequiredField {
            field: "evidence.evidence_id"
        }
    ));
}

#[test]
fn authority_dry_run_evidence_rejects_missing_nested_check_provenance() {
    let mut evidence = sample_authority_evidence();
    evidence.authority_report.check_id = None;

    let err = validate_authority_dry_run_evidence(&evidence)
        .expect_err("full evidence should carry nested report check provenance");

    assert!(matches!(
        err,
        AuthorityEvidenceError::MissingRequiredField {
            field: "report.check_id"
        }
    ));
}

#[test]
fn authority_dry_run_evidence_rejects_mismatched_receipt_content() {
    let mut evidence = sample_authority_evidence();
    evidence
        .authority_receipt
        .hard_failures
        .push(SafetyFindingV1 {
            code: "extra".to_string(),
            message: "extra hard finding".to_string(),
            severity: SafetySeverityV1::HardFailure,
            subject: None,
        });

    let err = validate_authority_dry_run_evidence(&evidence)
        .expect_err("mismatched receipt content should fail evidence validation");

    assert!(matches!(
        err,
        AuthorityEvidenceError::PlanReportContentMismatch {
            field: "receipt.hard_failures",
        }
    ));
}

#[test]
fn authority_dry_run_evidence_rejects_mutated_report_counts() {
    let mut evidence = sample_authority_evidence();
    evidence.authority_report.counts.already_correct = 0;

    let err = validate_authority_dry_run_evidence(&evidence)
        .expect_err("mutated report counts should fail evidence validation");

    assert!(matches!(
        err,
        AuthorityEvidenceError::PlanReportContentMismatch {
            field: "report.counts",
        }
    ));
}

#[test]
fn authority_dry_run_evidence_rejects_mutated_report_readiness() {
    let mut evidence = sample_authority_evidence();
    evidence
        .authority_report
        .apply_readiness
        .can_apply_automatically = true;

    let err = validate_authority_dry_run_evidence(&evidence)
        .expect_err("mutated report readiness should fail evidence validation");

    assert!(matches!(
        err,
        AuthorityEvidenceError::PlanReportContentMismatch {
            field: "report.apply_readiness",
        }
    ));
}

#[test]
fn authority_dry_run_evidence_rejects_mutated_unsafe_blocker_readiness() {
    let mut evidence = sample_authority_evidence_from_check(sample_unknown_unsafe_check());
    assert_eq!(
        evidence.authority_report.apply_readiness.blockers,
        vec![AuthorityApplyBlockerV1::UnsafeBlocked]
    );

    evidence.authority_report.apply_readiness.blockers =
        vec![AuthorityApplyBlockerV1::HardFailures];

    let err = validate_authority_dry_run_evidence(&evidence)
        .expect_err("mutated unsafe blocker readiness should fail evidence validation");

    assert!(matches!(
        err,
        AuthorityEvidenceError::PlanReportContentMismatch {
            field: "report.apply_readiness",
        }
    ));
}

#[test]
fn authority_dry_run_evidence_rejects_attempted_actions() {
    let mut evidence = sample_authority_evidence();
    evidence
        .authority_receipt
        .attempted_actions
        .push(AuthorityAttemptedActionV1 {
            subject: "aaaaa-aa".to_string(),
            canister_id: Some("aaaaa-aa".to_string()),
            role: Some("root".to_string()),
            action: AuthorityActionV1::AddControllers,
            result: RolePhaseResultV1::NotAttempted,
            error: None,
        });

    let err = validate_authority_dry_run_evidence(&evidence)
        .expect_err("attempted dry-run actions should fail evidence validation");

    assert!(matches!(
        err,
        AuthorityEvidenceError::DryRunReceiptAttemptedActions { count: 1 }
    ));
}

#[test]
fn authority_dry_run_evidence_rejects_non_complete_receipt_status() {
    let mut evidence = sample_authority_evidence();
    evidence.authority_receipt.operation_status = DeploymentExecutionStatusV1::FailedBeforeMutation;

    let err = validate_authority_dry_run_evidence(&evidence)
        .expect_err("non-complete dry-run receipts should fail evidence validation");

    assert!(matches!(
        err,
        AuthorityEvidenceError::DryRunReceiptStatus {
            status: DeploymentExecutionStatusV1::FailedBeforeMutation
        }
    ));
}

#[test]
fn authority_dry_run_evidence_rejects_failed_receipt_command_result() {
    let mut evidence = sample_authority_evidence();
    evidence.authority_receipt.command_result = DeploymentCommandResultV1::Failed {
        code: "dry_run_failed".to_string(),
        message: "dry run failed".to_string(),
    };

    let err = validate_authority_dry_run_evidence(&evidence)
        .expect_err("failed dry-run command results should fail evidence validation");

    assert!(matches!(
        err,
        AuthorityEvidenceError::DryRunReceiptCommandResult {
            result: DeploymentCommandResultV1::Failed { .. }
        }
    ));
}

#[test]
fn authority_dry_run_evidence_rejects_complete_receipt_without_finished_at() {
    let mut evidence = sample_authority_evidence();
    evidence.authority_receipt.finished_at = None;

    let err = validate_authority_dry_run_evidence(&evidence)
        .expect_err("complete dry-run receipts should record finished_at");

    assert!(matches!(
        err,
        AuthorityEvidenceError::DryRunReceiptMissingFinishedAt
    ));
}

#[test]
fn authority_dry_run_evidence_rejects_generated_at_mismatch() {
    let mut evidence = sample_authority_evidence();
    evidence.generated_at = "2026-05-23T00:00:02Z".to_string();

    let err = validate_authority_dry_run_evidence(&evidence)
        .expect_err("evidence generated_at should match receipt completion time");

    assert!(matches!(
        err,
        AuthorityEvidenceError::EvidenceGeneratedAtMismatch {
            evidence_value,
            receipt_value,
        } if evidence_value == "2026-05-23T00:00:02Z"
            && receipt_value == "2026-05-23T00:00:01Z"
    ));
}

#[test]
fn authority_dry_run_evidence_rejects_receipt_finished_before_started() {
    let mut evidence = sample_authority_evidence();
    evidence.authority_receipt.started_at = "2026-05-23T00:00:02Z".to_string();

    let err = validate_authority_dry_run_evidence(&evidence)
        .expect_err("dry-run receipt finish time should not precede start time");

    assert!(matches!(
        err,
        AuthorityEvidenceError::DryRunReceiptTimestampOrder {
            field: "receipt.started_at",
            other_field: "receipt.finished_at",
            ..
        }
    ));
}

#[test]
fn authority_dry_run_evidence_rejects_mismatched_controller_observations() {
    let mut evidence = sample_authority_evidence();
    evidence
        .authority_receipt
        .verified_controller_observations
        .clear();

    let err = validate_authority_dry_run_evidence(&evidence)
        .expect_err("mismatched controller observations should fail evidence validation");

    assert!(matches!(
        err,
        AuthorityEvidenceError::PlanReportContentMismatch {
            field: "receipt.verified_controller_observations",
        }
    ));
}

#[test]
fn authority_reconciliation_marks_deployment_controlled_delta_as_automatic_dry_run() {
    let mut plan = sample_plan();
    plan.authority_profile.expected_controllers =
        vec!["aaaaa-aa".to_string(), "ops-principal".to_string()];
    let check = sample_check(plan, sample_matching_inventory());

    let reconciliation = build_authority_reconciliation_plan(&check);

    assert!(reconciliation.hard_failures.is_empty());
    assert!(reconciliation.external_actions_required.is_empty());
    assert_eq!(
        reconciliation.canister_actions[0].state,
        AuthorityReconciliationStateV1::CanApplyAutomatically
    );
    assert_eq!(
        reconciliation.canister_actions[0].action,
        AuthorityActionV1::AddControllers
    );
    assert!(reconciliation.canister_actions[0].can_apply);
    assert!(
        reconciliation.canister_actions[0]
            .reason
            .contains("ops-principal")
    );
    assert_eq!(reconciliation.automatic_actions.len(), 1);
    assert_eq!(reconciliation.automatic_actions[0].subject, "aaaaa-aa");
    assert_eq!(reconciliation.automatic_actions[0].canister_id, "aaaaa-aa");
    assert_eq!(
        reconciliation.automatic_actions[0].action,
        AuthorityActionV1::AddControllers
    );
    assert_eq!(
        reconciliation.automatic_actions[0].observed_controllers,
        vec!["aaaaa-aa".to_string()]
    );
    assert_eq!(
        reconciliation.automatic_actions[0].desired_controllers,
        vec!["aaaaa-aa".to_string(), "ops-principal".to_string()]
    );
    assert_eq!(
        reconciliation.automatic_actions[0].controller_delta,
        AuthorityControllerDeltaV1 {
            add_controllers: vec!["ops-principal".to_string()],
            remove_controllers: Vec::new(),
        }
    );

    let report = authority_report_from_plan("authority-report-1", &reconciliation);
    assert_eq!(report.status, SafetyStatusV1::Safe);
    assert_eq!(report.counts.can_apply_automatically, 1);
    assert_eq!(
        report.apply_readiness,
        AuthorityApplyReadinessV1 {
            can_apply_automatically: true,
            automatic_action_count: 1,
            blockers: Vec::new(),
        }
    );
    assert_eq!(
        report.action_counts,
        vec![AuthorityActionCountV1 {
            action: AuthorityActionV1::AddControllers,
            count: 1,
        }]
    );
    assert!(report.observation_gaps.is_empty());
    assert_eq!(report.automatic_actions, reconciliation.automatic_actions);
    assert_eq!(
        report.next_actions,
        vec![
            "review automatic authority dry-run actions before enabling an apply path".to_string()
        ]
    );
}

#[test]
fn authority_apply_readiness_blocks_automatic_candidates_when_external_actions_remain() {
    let mut plan = sample_plan();
    plan.authority_profile.expected_controllers =
        vec!["aaaaa-aa".to_string(), "ops-principal".to_string()];
    plan.expected_canisters.push(ExpectedCanisterV1 {
        role: "user_hub".to_string(),
        canister_id: Some("user-hub-canister".to_string()),
        control_class: CanisterControlClassV1::UserControlled,
    });
    let mut inventory = sample_matching_inventory();
    inventory.observed_canisters.push(ObservedCanisterV1 {
        canister_id: "user-hub-canister".to_string(),
        role: Some("user_hub".to_string()),
        control_class: CanisterControlClassV1::UserControlled,
        controllers: vec!["user-controller".to_string()],
        module_hash: None,
        status: Some("running".to_string()),
        root_trust_anchor: Some("aaaaa-aa".to_string()),
        canonical_embedded_config_digest: None,
        role_assignment_source: Some("icp_canister_status".to_string()),
    });
    let check = sample_check(plan, inventory);

    let reconciliation = build_authority_reconciliation_plan(&check);
    let report = authority_report_from_plan("authority-report-1", &reconciliation);

    assert_eq!(report.counts.can_apply_automatically, 1);
    assert_eq!(report.counts.requires_external_action, 1);
    assert_eq!(
        report.apply_readiness,
        AuthorityApplyReadinessV1 {
            can_apply_automatically: false,
            automatic_action_count: 1,
            blockers: vec![AuthorityApplyBlockerV1::ExternalActions],
        }
    );
    assert_eq!(
        report.next_actions,
        vec![
            "review external authority actions before applying controller changes",
            "review automatic authority dry-run actions before enabling an apply path",
        ]
    );
}

#[test]
fn authority_reconciliation_blocks_staging_or_emergency_controller_overlap() {
    let mut plan = sample_plan();
    plan.authority_profile.staging_controllers = vec!["aaaaa-aa".to_string()];
    plan.authority_profile.emergency_controllers = vec!["aaaaa-aa".to_string()];
    let check = sample_check(plan, sample_matching_inventory());

    let reconciliation = build_authority_reconciliation_plan(&check);

    assert_eq!(reconciliation.hard_failures.len(), 2);
    assert!(
        reconciliation
            .hard_failures
            .iter()
            .all(|finding| finding.code == "authority_profile_overlap"
                && finding.severity == SafetySeverityV1::HardFailure
                && finding.subject.as_deref() == Some("aaaaa-aa"))
    );
    assert_eq!(
        reconciliation.canister_actions[0].state,
        AuthorityReconciliationStateV1::AlreadyCorrect
    );

    let report = authority_report_from_plan("authority-report-1", &reconciliation);
    assert_eq!(report.status, SafetyStatusV1::Blocked);
    assert_eq!(report.counts.already_correct, 1);
    assert_eq!(report.counts.unsafe_blocked, 0);
    assert_eq!(report.counts.hard_failures, 2);
    assert_eq!(
        report.apply_readiness,
        AuthorityApplyReadinessV1 {
            can_apply_automatically: false,
            automatic_action_count: 0,
            blockers: vec![AuthorityApplyBlockerV1::HardFailures],
        }
    );
    assert_eq!(report.hard_failures, reconciliation.hard_failures);
    assert_eq!(
        report.next_actions,
        vec!["resolve hard authority findings before applying controller changes"]
    );
}

#[test]
fn authority_reconciliation_requires_external_action_for_user_controlled_drift() {
    let mut plan = sample_plan();
    plan.authority_profile.expected_controllers = vec!["aaaaa-aa".to_string()];
    let mut inventory = sample_matching_inventory();
    inventory.observed_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    inventory.observed_canisters[0].controllers = vec!["user-controller".to_string()];
    let check = sample_check(plan, inventory);

    let reconciliation = build_authority_reconciliation_plan(&check);

    assert!(reconciliation.hard_failures.is_empty());
    assert_eq!(reconciliation.external_actions_required.len(), 1);
    let external = &reconciliation.external_actions_required[0];
    assert_eq!(external.subject, "aaaaa-aa");
    assert_eq!(external.canister_id.as_deref(), Some("aaaaa-aa"));
    assert_eq!(external.role.as_deref(), Some("root"));
    assert_eq!(
        external.control_classification,
        CanisterControlClassV1::UserControlled
    );
    assert_eq!(
        external.state,
        AuthorityReconciliationStateV1::RequiresExternalAction
    );
    assert_eq!(
        external.action,
        AuthorityActionV1::RequiresExternalController
    );
    assert_eq!(
        external.observed_controllers,
        vec!["user-controller".to_string()]
    );
    assert_eq!(external.desired_controllers, vec!["aaaaa-aa".to_string()]);
    assert_eq!(
        external.controller_delta,
        AuthorityControllerDeltaV1 {
            add_controllers: vec!["aaaaa-aa".to_string()],
            remove_controllers: vec!["user-controller".to_string()],
        }
    );
    assert_eq!(
        reconciliation.canister_actions[0].state,
        AuthorityReconciliationStateV1::RequiresExternalAction
    );
    assert_eq!(
        reconciliation.canister_actions[0].action,
        AuthorityActionV1::RequiresExternalController
    );
    assert!(!reconciliation.canister_actions[0].can_apply);

    let report = authority_report_from_plan("authority-report-1", &reconciliation);
    assert_eq!(report.status, SafetyStatusV1::Warning);
    assert_eq!(report.counts.requires_external_action, 1);
    assert_eq!(
        report.apply_readiness,
        AuthorityApplyReadinessV1 {
            can_apply_automatically: false,
            automatic_action_count: 0,
            blockers: vec![AuthorityApplyBlockerV1::ExternalActions],
        }
    );
    assert_eq!(report.external_actions_required.len(), 1);
    assert_eq!(report.external_actions_required[0], *external);
    assert_eq!(
        report.next_actions,
        vec!["review external authority actions before applying controller changes"]
    );
}

#[test]
fn authority_dry_run_receipt_records_observations_without_attempts() {
    let mut plan = sample_plan();
    plan.authority_profile.expected_controllers = vec!["aaaaa-aa".to_string()];
    let mut inventory = sample_matching_inventory();
    inventory.observed_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    inventory.observed_canisters[0].controllers = vec!["user-controller".to_string()];
    let check = sample_check(plan, inventory);
    let reconciliation = build_authority_reconciliation_plan(&check);
    let report = authority_report_from_plan_with_check_id(
        "authority-report-1",
        Some(check.check_id.clone()),
        &reconciliation,
    );

    let receipt = authority_dry_run_receipt_from_plan(
        &reconciliation,
        &report,
        Some(check.check_id.clone()),
        "authority-dry-run-1",
        "2026-05-23T00:00:00Z",
        Some("2026-05-23T00:00:01Z".to_string()),
    )
    .expect("build authority receipt");

    assert_eq!(receipt.operation_id, "authority-dry-run-1");
    assert_eq!(receipt.check_id.as_deref(), Some("check-1"));
    assert_eq!(receipt.reconciliation_plan_id, "plan-local-root");
    assert_eq!(receipt.authority_report_id, "authority-report-1");
    assert_eq!(receipt.inventory_id, "inventory-1");
    assert_eq!(receipt.authority_profile_hash.as_deref(), Some("authority"));
    assert_eq!(
        receipt.operation_status,
        DeploymentExecutionStatusV1::Complete
    );
    assert_eq!(receipt.command_result, DeploymentCommandResultV1::Succeeded);
    assert!(receipt.attempted_actions.is_empty());
    assert_eq!(receipt.verified_controller_observations.len(), 1);
    assert_eq!(
        receipt.verified_controller_observations[0],
        AuthorityControllerObservationV1 {
            subject: "aaaaa-aa".to_string(),
            canister_id: Some("aaaaa-aa".to_string()),
            role: Some("root".to_string()),
            state: AuthorityReconciliationStateV1::RequiresExternalAction,
            action: AuthorityActionV1::RequiresExternalController,
            observed_controllers: vec!["user-controller".to_string()],
            desired_controllers: vec!["aaaaa-aa".to_string()],
            controller_delta: AuthorityControllerDeltaV1 {
                add_controllers: vec!["aaaaa-aa".to_string()],
                remove_controllers: vec!["user-controller".to_string()],
            },
        }
    );
    assert_eq!(
        receipt.unresolved_external_actions,
        report.external_actions_required
    );
    assert_eq!(receipt.hard_failures, report.hard_failures);
    assert_eq!(receipt.unresolved_observation_gaps, report.observation_gaps);

    let evidence = AuthorityDryRunEvidenceV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        evidence_id: "authority-evidence-1".to_string(),
        check_id: check.check_id,
        generated_at: "2026-05-23T00:00:01Z".to_string(),
        reconciliation_plan: reconciliation,
        authority_report: report,
        authority_receipt: receipt,
    };

    assert_json_round_trip(&evidence);
}

#[test]
fn authority_v1_json_schema_shape_is_stable() {
    let evidence = sample_authority_evidence();
    let value = serde_json::to_value(&evidence).expect("encode authority evidence");

    assert_object_keys(
        &value,
        &[
            "schema_version",
            "evidence_id",
            "check_id",
            "generated_at",
            "reconciliation_plan",
            "authority_report",
            "authority_receipt",
        ],
    );

    assert_object_keys(
        &value["reconciliation_plan"],
        &[
            "schema_version",
            "plan_id",
            "inventory_id",
            "authority_profile_hash",
            "canister_actions",
            "automatic_actions",
            "hard_failures",
            "external_actions_required",
        ],
    );
    assert_object_keys(
        &value["authority_report"],
        &[
            "schema_version",
            "report_id",
            "check_id",
            "reconciliation_plan_id",
            "inventory_id",
            "authority_profile_hash",
            "status",
            "summary",
            "counts",
            "apply_readiness",
            "action_counts",
            "control_class_counts",
            "observation_gaps",
            "automatic_actions",
            "hard_failures",
            "external_actions_required",
            "next_actions",
        ],
    );
    assert_object_keys(
        &value["authority_receipt"],
        &[
            "schema_version",
            "operation_id",
            "check_id",
            "reconciliation_plan_id",
            "authority_report_id",
            "inventory_id",
            "authority_profile_hash",
            "operation_status",
            "started_at",
            "finished_at",
            "attempted_actions",
            "verified_controller_observations",
            "hard_failures",
            "unresolved_observation_gaps",
            "unresolved_external_actions",
            "command_result",
        ],
    );

    assert_eq!(value["authority_report"]["status"], "Safe");
    assert_eq!(
        value["reconciliation_plan"]["canister_actions"][0]["state"],
        "AlreadyCorrect"
    );
    assert_eq!(
        value["reconciliation_plan"]["canister_actions"][0]["action"],
        "None"
    );
    assert_eq!(
        value["reconciliation_plan"]["canister_actions"][0]["control_classification"],
        "DeploymentControlled"
    );
    assert_eq!(value["authority_receipt"]["operation_status"], "Complete");
    assert_eq!(value["authority_receipt"]["command_result"], "Succeeded");
}

#[test]
fn deployment_truth_authority_paths_have_no_controller_mutation_primitives() {
    for (path, source) in [
        ("authority.rs", include_str!("authority.rs")),
        ("receipt.rs", include_str!("receipt.rs")),
        ("text.rs", include_str!("text.rs")),
    ] {
        for forbidden in [
            "update_settings",
            "install_code",
            "create_canister",
            "delete_canister",
            "stop_canister",
            "uninstall_code",
            "provisional_create_canister",
            "dfx",
        ] {
            assert!(
                !source.contains(forbidden),
                "deployment truth authority path {path} must stay dry-run; found forbidden token {forbidden}"
            );
        }
    }
}

#[test]
fn authority_dry_run_receipt_preserves_hard_findings() {
    let mut plan = sample_plan();
    plan.authority_profile.staging_controllers = vec!["aaaaa-aa".to_string()];
    let check = sample_check(plan, sample_matching_inventory());
    let reconciliation = build_authority_reconciliation_plan(&check);
    let report = authority_report_from_plan_with_check_id(
        "authority-report-1",
        Some(check.check_id.clone()),
        &reconciliation,
    );

    let receipt = authority_dry_run_receipt_from_plan(
        &reconciliation,
        &report,
        Some(check.check_id),
        "authority-dry-run-1",
        "2026-05-23T00:00:00Z",
        Some("2026-05-23T00:00:01Z".to_string()),
    )
    .expect("build authority receipt");

    assert_eq!(report.status, SafetyStatusV1::Blocked);
    assert_eq!(report.hard_failures.len(), 1);
    assert_eq!(receipt.hard_failures, report.hard_failures);
    assert!(receipt.unresolved_observation_gaps.is_empty());
    assert!(receipt.attempted_actions.is_empty());
    assert_eq!(receipt.verified_controller_observations.len(), 1);
}

#[test]
fn authority_reconciliation_blocks_unknown_unsafe_canister() {
    let check = sample_unknown_unsafe_check();

    let reconciliation = build_authority_reconciliation_plan(&check);

    assert_eq!(reconciliation.hard_failures.len(), 1);
    assert_eq!(
        reconciliation.hard_failures[0].code,
        "authority_unsafe_blocked"
    );
    assert!(reconciliation.canister_actions.iter().any(|action| {
        action.canister_id.as_deref() == Some("unsafe-canister")
            && action.state == AuthorityReconciliationStateV1::UnsafeBlocked
            && action.action == AuthorityActionV1::BlockedByPolicy
    }));

    let report = authority_report_from_plan("authority-report-1", &reconciliation);
    assert_eq!(report.status, SafetyStatusV1::Blocked);
    assert_eq!(report.counts.unsafe_blocked, 1);
    assert_eq!(report.counts.hard_failures, 0);
    assert_eq!(
        report.apply_readiness,
        AuthorityApplyReadinessV1 {
            can_apply_automatically: false,
            automatic_action_count: 0,
            blockers: vec![AuthorityApplyBlockerV1::UnsafeBlocked],
        }
    );
    assert!(report.external_actions_required.is_empty());
    assert_eq!(
        report.control_class_counts,
        vec![
            AuthorityControlClassCountV1 {
                control_class: CanisterControlClassV1::DeploymentControlled,
                count: 1,
            },
            AuthorityControlClassCountV1 {
                control_class: CanisterControlClassV1::UnknownUnsafe,
                count: 1,
            },
        ]
    );
    assert_eq!(
        report.next_actions,
        vec!["resolve unsafe canister authority findings before applying controller changes"]
    );
    let report_text = authority_report_text(&report);
    assert!(report_text.contains("    - unsafe_blocked"));
    assert!(!report_text.contains("    - hard_failures"));
}

#[test]
fn unsafe_authority_receipt_preserves_finding_without_hard_readiness_double_count() {
    let check = sample_unknown_unsafe_check();
    let reconciliation = build_authority_reconciliation_plan(&check);
    let report = authority_report_from_plan_with_check_id(
        "authority-report-1",
        Some(check.check_id.clone()),
        &reconciliation,
    );

    let receipt = authority_dry_run_receipt_from_plan(
        &reconciliation,
        &report,
        Some(check.check_id),
        "authority-dry-run-1",
        "2026-05-23T00:00:00Z",
        Some("2026-05-23T00:00:01Z".to_string()),
    )
    .expect("build authority receipt");

    assert_eq!(report.counts.unsafe_blocked, 1);
    assert_eq!(report.counts.hard_failures, 0);
    assert_eq!(
        report.apply_readiness.blockers,
        vec![AuthorityApplyBlockerV1::UnsafeBlocked]
    );
    assert_eq!(receipt.hard_failures, report.hard_failures);
    assert_eq!(receipt.hard_failures.len(), 1);
    assert_eq!(receipt.hard_failures[0].code, "authority_unsafe_blocked");
}

#[test]
fn authority_report_distinguishes_unsafe_and_hard_authority_blockers() {
    let mut plan = sample_plan();
    plan.authority_profile.staging_controllers = vec!["aaaaa-aa".to_string()];
    let mut inventory = sample_matching_inventory();
    inventory.observed_canisters.push(ObservedCanisterV1 {
        canister_id: "unsafe-canister".to_string(),
        role: Some("surprise".to_string()),
        control_class: CanisterControlClassV1::UnknownUnsafe,
        controllers: vec!["unknown-controller".to_string()],
        module_hash: None,
        status: None,
        root_trust_anchor: None,
        canonical_embedded_config_digest: None,
        role_assignment_source: Some("icp_canister_status".to_string()),
    });
    let check = sample_check(plan, inventory);

    let reconciliation = build_authority_reconciliation_plan(&check);
    let report = authority_report_from_plan("authority-report-1", &reconciliation);

    assert_eq!(reconciliation.hard_failures.len(), 2);
    assert_eq!(report.status, SafetyStatusV1::Blocked);
    assert_eq!(report.counts.unsafe_blocked, 1);
    assert_eq!(report.counts.hard_failures, 1);
    assert_eq!(
        report.apply_readiness,
        AuthorityApplyReadinessV1 {
            can_apply_automatically: false,
            automatic_action_count: 0,
            blockers: vec![
                AuthorityApplyBlockerV1::UnsafeBlocked,
                AuthorityApplyBlockerV1::HardFailures,
            ],
        }
    );
    assert_eq!(
        report.next_actions,
        vec![
            "resolve unsafe canister authority findings before applying controller changes",
            "resolve hard authority findings before applying controller changes",
        ]
    );
}

#[test]
fn blocked_authority_report_keeps_external_and_gap_next_actions() {
    let mut plan = sample_plan();
    plan.authority_profile.expected_controllers =
        vec!["aaaaa-aa".to_string(), "ops-principal".to_string()];
    plan.authority_profile.staging_controllers = vec!["aaaaa-aa".to_string()];
    plan.expected_canisters.push(ExpectedCanisterV1 {
        role: "user_hub".to_string(),
        canister_id: Some("user-hub-canister".to_string()),
        control_class: CanisterControlClassV1::UserControlled,
    });
    plan.expected_pool.push(ExpectedPoolCanisterV1 {
        pool: "user-shards".to_string(),
        canister_id: Some("pool-canister".to_string()),
        role: Some("user_shard".to_string()),
    });
    let mut inventory = sample_matching_inventory();
    inventory.observed_canisters.push(ObservedCanisterV1 {
        canister_id: "user-hub-canister".to_string(),
        role: Some("user_hub".to_string()),
        control_class: CanisterControlClassV1::UserControlled,
        controllers: vec!["user-controller".to_string()],
        module_hash: None,
        status: Some("running".to_string()),
        root_trust_anchor: Some("aaaaa-aa".to_string()),
        canonical_embedded_config_digest: None,
        role_assignment_source: Some("icp_canister_status".to_string()),
    });
    inventory.observed_pool.push(ObservedPoolCanisterV1 {
        pool: "user-shards".to_string(),
        canister_id: "pool-canister".to_string(),
        role: Some("user_shard".to_string()),
        control_class: CanisterControlClassV1::CanicManagedPool,
    });
    let check = sample_check(plan, inventory);

    let reconciliation = build_authority_reconciliation_plan(&check);
    let report = authority_report_from_plan("authority-report-1", &reconciliation);

    assert_eq!(report.status, SafetyStatusV1::Blocked);
    assert_eq!(
        report.summary,
        "authority reconciliation is blocked by 0 unsafe canister(s) and 1 hard authority finding(s); also requires 1 external action(s) and has 1 unknown observation(s)"
    );
    assert_eq!(report.counts.hard_failures, 1);
    assert_eq!(report.counts.requires_external_action, 1);
    assert_eq!(report.counts.unknown, 1);
    assert_eq!(
        report.next_actions,
        vec![
            "resolve hard authority findings before applying controller changes",
            "review external authority actions before applying controller changes",
            "collect missing controller observations before applying controller changes",
            "review automatic authority dry-run actions before enabling an apply path",
        ]
    );
}

#[test]
fn authority_reconciliation_reports_expected_pool_controller_observation_gap() {
    let mut plan = sample_plan();
    plan.expected_pool.push(ExpectedPoolCanisterV1 {
        pool: "user-shards".to_string(),
        canister_id: Some("pool-canister".to_string()),
        role: Some("user_shard".to_string()),
    });
    let mut inventory = sample_matching_inventory();
    inventory.observed_pool.push(ObservedPoolCanisterV1 {
        pool: "user-shards".to_string(),
        canister_id: "pool-canister".to_string(),
        role: Some("user_shard".to_string()),
        control_class: CanisterControlClassV1::CanicManagedPool,
    });
    let check = sample_check(plan, inventory);

    let reconciliation = build_authority_reconciliation_plan(&check);

    let pool_action = reconciliation
        .canister_actions
        .iter()
        .find(|action| action.canister_id.as_deref() == Some("pool-canister"))
        .expect("pool action should be reported");
    assert_eq!(pool_action.state, AuthorityReconciliationStateV1::Unknown);
    assert_eq!(pool_action.action, AuthorityActionV1::UnknownObservation);
    assert_eq!(
        pool_action.reason,
        "pool canister controller set was not observed"
    );
    assert!(reconciliation.external_actions_required.is_empty());
    let report = authority_report_from_plan_with_check_id(
        "authority-report-1",
        Some(check.check_id.clone()),
        &reconciliation,
    );
    assert_eq!(report.counts.unknown, 1);
    assert!(report.external_actions_required.is_empty());
    assert_eq!(
        report.apply_readiness,
        AuthorityApplyReadinessV1 {
            can_apply_automatically: false,
            automatic_action_count: 0,
            blockers: vec![AuthorityApplyBlockerV1::ObservationGaps],
        }
    );
    assert_eq!(report.observation_gaps.len(), 1);
    assert_eq!(
        report.observation_gaps[0],
        DeploymentObservationGapV1 {
            key: "authority.controllers.pool-canister".to_string(),
            description: "pool canister controller set was not observed".to_string(),
        }
    );
    let receipt = authority_dry_run_receipt_from_plan(
        &reconciliation,
        &report,
        Some(check.check_id),
        "authority-dry-run-1",
        "2026-05-23T00:00:00Z",
        Some("2026-05-23T00:00:01Z".to_string()),
    )
    .expect("build authority receipt");
    assert_eq!(receipt.unresolved_observation_gaps, report.observation_gaps);
    assert!(receipt.unresolved_external_actions.is_empty());
    assert_eq!(
        report.action_counts,
        vec![
            AuthorityActionCountV1 {
                action: AuthorityActionV1::None,
                count: 1,
            },
            AuthorityActionCountV1 {
                action: AuthorityActionV1::UnknownObservation,
                count: 1,
            },
        ]
    );
    assert_eq!(
        report.control_class_counts,
        vec![
            AuthorityControlClassCountV1 {
                control_class: CanisterControlClassV1::DeploymentControlled,
                count: 1,
            },
            AuthorityControlClassCountV1 {
                control_class: CanisterControlClassV1::CanicManagedPool,
                count: 1,
            },
        ]
    );
    assert_eq!(
        report.next_actions,
        vec!["collect missing controller observations before applying controller changes"]
    );
}

#[test]
fn authority_reconciliation_reports_unplanned_pool_canister_for_external_action() {
    let mut inventory = sample_matching_inventory();
    inventory.observed_pool.push(ObservedPoolCanisterV1 {
        pool: "user-shards".to_string(),
        canister_id: "unplanned-pool".to_string(),
        role: Some("user_shard".to_string()),
        control_class: CanisterControlClassV1::CanicManagedPool,
    });
    let check = sample_check(sample_plan(), inventory);

    let reconciliation = build_authority_reconciliation_plan(&check);

    let pool_action = reconciliation
        .canister_actions
        .iter()
        .find(|action| action.canister_id.as_deref() == Some("unplanned-pool"))
        .expect("unplanned pool action should be reported");
    assert_eq!(
        pool_action.state,
        AuthorityReconciliationStateV1::RequiresExternalAction
    );
    assert_eq!(pool_action.action, AuthorityActionV1::AdoptPlanAvailable);
    assert!(
        reconciliation
            .external_actions_required
            .iter()
            .any(|external| {
                external.subject == "unplanned-pool"
                    && external.action == AuthorityActionV1::AdoptPlanAvailable
                    && external.reason
                        == "observed pool canister is not present in the expected pool plan"
            })
    );
}

const SAMPLE_CONFIG: &str = r#"
controllers = []
app_index = []

[fleet]
name = "demo"

[app]
init_mode = "enabled"
[app.whitelist]

[subnets.prime.canisters.root]
kind = "root"

[subnets.prime.canisters.user_hub]
kind = "singleton"
"#;

fn assert_json_round_trip<T>(value: &T)
where
    T: Clone + std::fmt::Debug + Eq + serde::de::DeserializeOwned + Serialize,
{
    let encoded = serde_json::to_string(value).expect("value should encode");
    let decoded = serde_json::from_str::<T>(&encoded).expect("value should decode");
    assert_eq!(decoded, *value);
}

fn assert_object_keys(value: &serde_json::Value, expected: &[&str]) {
    let object = value.as_object().expect("value should be a JSON object");
    let mut actual = object.keys().map(String::as_str).collect::<Vec<_>>();
    actual.sort_unstable();
    let mut expected = expected.to_vec();
    expected.sort_unstable();
    assert_eq!(actual, expected);
}

fn sample_identity() -> DeploymentIdentityV1 {
    DeploymentIdentityV1 {
        deployment_name: "local-root".to_string(),
        network: "local".to_string(),
        root_principal: Some("aaaaa-aa".to_string()),
        authority_profile_hash: Some("authority".to_string()),
        role_topology_hash: Some("topology".to_string()),
        deployment_manifest_digest: Some("manifest".to_string()),
        canonical_runtime_config_digest: Some("runtime".to_string()),
        role_embedded_config_set_digest: Some("embedded".to_string()),
        artifact_set_digest: Some("artifacts".to_string()),
        pool_identity_set_digest: None,
        canic_version: Some("0.41.0".to_string()),
        ic_memory_version: Some("0.6.1".to_string()),
    }
}

fn sample_role_artifact() -> RoleArtifactV1 {
    RoleArtifactV1 {
        role: "root".to_string(),
        source: ArtifactSourceV1::LocalBuild,
        build_profile: "fast".to_string(),
        wasm_path: Some("root.wasm".to_string()),
        wasm_gz_path: Some("root.wasm.gz".to_string()),
        wasm_gz_size_bytes: Some(42),
        wasm_sha256: Some("wasm".to_string()),
        wasm_gz_sha256: Some("gzip".to_string()),
        wasm_gz_sha256_source: Some(ArtifactDigestSourceV1::ReleaseSetManifest),
        observed_wasm_gz_file_sha256: Some("file".to_string()),
        observed_wasm_gz_file_sha256_source: Some(ArtifactDigestSourceV1::ObservedFileDigest),
        installed_module_hash: Some("module".to_string()),
        candid_path: Some("root.did".to_string()),
        candid_sha256: Some("did".to_string()),
        raw_config_sha256: Some("raw".to_string()),
        canonical_embedded_config_sha256: Some("canonical".to_string()),
        embedded_topology_sha256: Some("topology".to_string()),
        builder_version: Some("0.41.0".to_string()),
        rust_toolchain: Some("stable".to_string()),
        package_version: Some("0.41.0".to_string()),
    }
}

fn sample_plan() -> DeploymentPlanV1 {
    DeploymentPlanV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        plan_id: "plan-local-root".to_string(),
        deployment_identity: sample_identity(),
        trust_domain: TrustDomainV1 {
            root_trust_anchor: Some("aaaaa-aa".to_string()),
            migration_from: None,
        },
        fleet_template: "root".to_string(),
        runtime_variant: "local".to_string(),
        authority_profile: AuthorityProfileV1 {
            profile_id: "local-default".to_string(),
            expected_controllers: vec!["aaaaa-aa".to_string()],
            staging_controllers: Vec::new(),
            emergency_controllers: Vec::new(),
        },
        role_artifacts: vec![sample_role_artifact()],
        expected_canisters: vec![ExpectedCanisterV1 {
            role: "root".to_string(),
            canister_id: Some("aaaaa-aa".to_string()),
            control_class: CanisterControlClassV1::DeploymentControlled,
        }],
        expected_pool: Vec::new(),
        expected_verifier_readiness: VerifierReadinessExpectationV1 {
            required: true,
            expected_role_epochs: vec![RoleEpochExpectationV1 {
                role: "root".to_string(),
                minimum_epoch: 1,
            }],
        },
        unresolved_assumptions: Vec::new(),
    }
}

fn sample_matching_inventory() -> DeploymentInventoryV1 {
    DeploymentInventoryV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        inventory_id: "inventory-1".to_string(),
        observed_at: "2026-05-22T00:00:00Z".to_string(),
        observed_identity: Some(sample_identity()),
        local_config: LocalDeploymentConfigV1 {
            config_path: Some("canic.toml".to_string()),
            raw_config_sha256: Some("raw".to_string()),
            canonical_embedded_config_sha256: Some("runtime".to_string()),
        },
        observed_canisters: vec![ObservedCanisterV1 {
            canister_id: "aaaaa-aa".to_string(),
            role: Some("root".to_string()),
            control_class: CanisterControlClassV1::DeploymentControlled,
            controllers: vec!["aaaaa-aa".to_string()],
            module_hash: Some("module".to_string()),
            status: Some("running".to_string()),
            root_trust_anchor: Some("aaaaa-aa".to_string()),
            canonical_embedded_config_digest: Some("canonical".to_string()),
            role_assignment_source: Some("icp_canister_status".to_string()),
        }],
        observed_pool: Vec::new(),
        observed_artifacts: vec![ObservedArtifactV1 {
            role: "root".to_string(),
            artifact_path: "root.wasm.gz".to_string(),
            file_sha256: Some("file".to_string()),
            file_sha256_source: Some(ArtifactDigestSourceV1::ObservedFileDigest),
            payload_sha256: Some("gzip".to_string()),
            payload_size_bytes: Some(42),
            source: ArtifactSourceV1::LocalBuild,
        }],
        observed_verifier_readiness: VerifierReadinessObservationV1 {
            status: ObservationStatusV1::Observed,
            role_epochs: vec![RoleEpochObservationV1 {
                role: "root".to_string(),
                observed_epoch: Some(1),
                status: ObservationStatusV1::Observed,
            }],
        },
        unresolved_observations: Vec::new(),
    }
}

fn sample_check(plan: DeploymentPlanV1, inventory: DeploymentInventoryV1) -> DeploymentCheckV1 {
    let diff = compare_plan_to_inventory(&plan, &inventory);
    let report = safety_report_from_diff("report-1", Some("diff-1".to_string()), &diff);
    DeploymentCheckV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        check_id: "check-1".to_string(),
        plan,
        inventory,
        diff,
        report,
    }
}

fn sample_authority_evidence() -> AuthorityDryRunEvidenceV1 {
    sample_authority_evidence_from_check(sample_check(sample_plan(), sample_matching_inventory()))
}

fn sample_authority_evidence_from_check(check: DeploymentCheckV1) -> AuthorityDryRunEvidenceV1 {
    authority_dry_run_evidence_from_check(
        &check,
        "authority-evidence-1",
        "authority-report-1",
        "authority-dry-run-1",
        "2026-05-23T00:00:01Z",
    )
    .expect("build authority evidence")
}

fn sample_unknown_unsafe_check() -> DeploymentCheckV1 {
    let mut inventory = sample_matching_inventory();
    inventory.observed_canisters.push(ObservedCanisterV1 {
        canister_id: "unsafe-canister".to_string(),
        role: Some("surprise".to_string()),
        control_class: CanisterControlClassV1::UnknownUnsafe,
        controllers: vec!["unknown-controller".to_string()],
        module_hash: None,
        status: None,
        root_trust_anchor: None,
        canonical_embedded_config_digest: None,
        role_assignment_source: Some("icp_canister_status".to_string()),
    });

    sample_check(sample_plan(), inventory)
}

fn sample_receipt_with_phase(
    plan_id: &str,
    root_principal: Option<&str>,
    postcondition: ObservationStatusV1,
    role_result: RolePhaseResultV1,
) -> DeploymentReceiptV1 {
    DeploymentReceiptV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        operation_id: "operation-1".to_string(),
        plan_id: plan_id.to_string(),
        execution_context: None,
        operation_status: DeploymentExecutionStatusV1::Complete,
        started_at: "2026-05-22T00:00:00Z".to_string(),
        finished_at: Some("2026-05-22T00:00:01Z".to_string()),
        operator_principal: None,
        root_principal: root_principal.map(str::to_string),
        previous_observed_deployment_epoch: None,
        phase_receipts: vec![PhaseReceiptV1 {
            phase: "materialize_artifacts".to_string(),
            started_at: "2026-05-22T00:00:00Z".to_string(),
            finished_at: Some("2026-05-22T00:00:01Z".to_string()),
            attempted_action: "verify configured role artifacts are materialized".to_string(),
            verified_postcondition: VerifiedPostconditionV1 {
                status: postcondition,
                evidence: vec!["artifact:root:sha256:file".to_string()],
            },
        }],
        role_phase_receipts: vec![RolePhaseReceiptV1 {
            role: "root".to_string(),
            phase: "materialize_artifacts".to_string(),
            result: role_result,
            previous_module_hash: None,
            target_module_hash: Some("module".to_string()),
            observed_module_hash_after: None,
            artifact_digest: Some("file".to_string()),
            canonical_embedded_config_sha256: Some("canonical".to_string()),
            error: (role_result == RolePhaseResultV1::Failed)
                .then(|| "artifact_missing: missing observed artifact for role root".to_string()),
        }],
        final_inventory_id: Some("inventory-1".to_string()),
        command_result: DeploymentCommandResultV1::Succeeded,
    }
}

fn sample_role_phase_receipt(result: RolePhaseResultV1) -> RolePhaseReceiptV1 {
    RolePhaseReceiptV1 {
        role: "root".to_string(),
        phase: "install_root".to_string(),
        result,
        previous_module_hash: None,
        target_module_hash: Some("module".to_string()),
        observed_module_hash_after: (result == RolePhaseResultV1::Applied)
            .then(|| "module".to_string()),
        artifact_digest: Some("file".to_string()),
        canonical_embedded_config_sha256: Some("canonical".to_string()),
        error: (result == RolePhaseResultV1::Failed).then(|| "install failed".to_string()),
    }
}

fn assert_sha256_len(value: Option<&String>) {
    assert_eq!(value.map(String::len), Some(64));
}

struct TempWorkspace {
    path: std::path::PathBuf,
}

impl TempWorkspace {
    fn new(name: &str) -> Self {
        let path = temp_dir(name);
        fs::create_dir_all(&path).expect("create temp dir");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempWorkspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn write_artifact(icp_root: &Path, role: &str, bytes: &[u8]) {
    let path = icp_root
        .join(".icp")
        .join("local")
        .join("canisters")
        .join(role)
        .join(format!("{role}.wasm.gz"));
    fs::create_dir_all(path.parent().expect("artifact parent")).expect("create artifact dir");
    fs::write(path, bytes).expect("write artifact");
}

fn write_release_set_manifest(icp_root: &Path) {
    let path = icp_root
        .join(".icp")
        .join("local")
        .join("canisters")
        .join("root")
        .join(ROOT_RELEASE_SET_MANIFEST_FILE);
    let manifest = serde_json::json!({
        "release_version": "0.41.1",
        "entries": [{
            "role": "user_hub",
            "template_id": "embedded:user_hub",
            "artifact_relative_path": ".icp/local/canisters/user_hub/user_hub.wasm.gz",
            "payload_size_bytes": 17,
            "payload_sha256_hex": "user-hub-hash",
            "chunk_size_bytes": 1_048_576,
            "chunk_sha256_hex": ["user-hub-hash"]
        }]
    });
    fs::create_dir_all(path.parent().expect("manifest parent")).expect("create manifest dir");
    fs::write(
        path,
        serde_json::to_vec_pretty(&manifest).expect("encode manifest"),
    )
    .expect("write manifest");
}

fn sample_install_state(root_canister_id: &str) -> InstallState {
    InstallState {
        schema_version: 1,
        fleet: "demo".to_string(),
        installed_at_unix_secs: 1,
        network: "local".to_string(),
        root_target: "root".to_string(),
        root_canister_id: root_canister_id.to_string(),
        root_build_target: "root".to_string(),
        workspace_root: "/workspace".to_string(),
        icp_root: "/workspace".to_string(),
        config_path: "fleets/canic.toml".to_string(),
        release_set_manifest_path: ".icp/local/canisters/root/release-set.json".to_string(),
    }
}
