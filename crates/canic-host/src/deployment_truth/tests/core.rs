use super::*;

#[test]
fn deployment_truth_keeps_the_single_current_pre_one_schema_identifier() {
    assert_eq!(DEPLOYMENT_TRUTH_SCHEMA_VERSION, 1);
}

#[test]
fn plan_round_trips_through_json() {
    let plan = DeploymentPlanV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        plan_id: "plan-local-root".to_string(),
        plan_digest: None,
        deployment_identity: sample_identity(),
        trust_domain: TrustDomainV1 {
            root_trust_anchor: Some("aaaaa-aa".to_string()),
        },
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
        observed_root: Some(DeploymentRootObservationV1 {
            canonical_network_id: CanonicalNetworkId::ic_mainnet(),
            fleet_id: FleetId::from_generated_bytes([7; 32]),
            fleet_name: "demo".to_string(),
            environment: "local".to_string(),
            app: "root".to_string(),
            root_principal: "aaaaa-aa".to_string(),
            observed_canister_id: "aaaaa-aa".to_string(),
            observation_source: DeploymentRootObservationSourceV1::IcpCanisterStatus,
            control_class: CanisterControlClassV1::DeploymentControlled,
            controllers: vec!["aaaaa-aa".to_string()],
            module_hash: Some("module".to_string()),
            status: Some("running".to_string()),
            role_assignment_source: Some(
                RoleAssignmentSourceV1::IcpCanisterStatus
                    .label()
                    .to_string(),
            ),
        }),
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
            role_assignment_source: Some(
                RoleAssignmentSourceV1::ComponentRegistry
                    .label()
                    .to_string(),
            ),
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
        fresh_fleet_decision: None,
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
