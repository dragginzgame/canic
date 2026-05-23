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
    assert_eq!(report.counts.already_correct, 1);
    assert_eq!(report.counts.can_apply_automatically, 0);
    assert_eq!(report.counts.requires_external_action, 0);
    assert_eq!(report.counts.unsafe_blocked, 0);
    assert_eq!(report.counts.unknown, 0);
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

    let report = authority_report_from_plan("authority-report-1", &reconciliation);
    assert_eq!(report.status, SafetyStatusV1::Safe);
    assert_eq!(report.counts.can_apply_automatically, 1);
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
    assert_eq!(report.external_actions_required.len(), 1);
    assert_eq!(report.external_actions_required[0], *external);
    assert_eq!(
        report.next_actions,
        vec!["review external authority actions before applying controller changes"]
    );
}

#[test]
fn authority_reconciliation_blocks_unknown_unsafe_canister() {
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
    let check = sample_check(sample_plan(), inventory);

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
    assert!(
        report
            .external_actions_required
            .iter()
            .any(|external| external.subject == "unsafe-canister"
                && external.control_classification == CanisterControlClassV1::UnknownUnsafe
                && external.state == AuthorityReconciliationStateV1::UnsafeBlocked
                && external.observed_controllers == vec!["unknown-controller".to_string()])
    );
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
        vec!["resolve unsafe authority findings before applying controller changes"]
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
    assert!(
        reconciliation
            .external_actions_required
            .iter()
            .any(|external| {
                external.subject == "pool-canister"
                    && external.role.as_deref() == Some("user_shard")
                    && external.control_classification == CanisterControlClassV1::CanicManagedPool
                    && external.state == AuthorityReconciliationStateV1::Unknown
            })
    );
    let report = authority_report_from_plan("authority-report-1", &reconciliation);
    assert_eq!(report.counts.unknown, 1);
    assert_eq!(report.observation_gaps.len(), 1);
    assert_eq!(
        report.observation_gaps[0],
        DeploymentObservationGapV1 {
            key: "authority.controllers.pool-canister".to_string(),
            description: "pool canister controller set was not observed".to_string(),
        }
    );
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
