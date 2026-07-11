use super::super::*;
use crate::deployment_truth::report::{
    ARTIFACT_FILE_SHA256_DIFF_CATEGORY, ARTIFACT_MISSING_CODE, CANISTER_UNOBSERVED_CODE,
    OBSERVATION_GAP_CODE, PLAN_ASSUMPTION_CODE, SUBNET_REGISTRY_ROLE_MISSING_CODE,
    UNSAFE_CONTROL_CLASS_CODE, UNVERIFIED_DEPLOYMENT_ROOT_CODE,
};

#[test]
fn deployment_diff_blocks_missing_artifacts_and_unsafe_control_class() {
    let plan = sample_plan();
    let inventory = DeploymentInventoryV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        inventory_id: "inventory-1".to_string(),
        observed_at: "2026-05-21T00:00:00Z".to_string(),
        observed_identity: Some(sample_identity()),
        observed_root: None,
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
            role_assignment_source: Some(
                RoleAssignmentSourceV1::LocalInstallState
                    .label()
                    .to_string(),
            ),
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
            .any(|item| item.code == ARTIFACT_MISSING_CODE)
    );
    assert!(
        diff.hard_failures
            .iter()
            .any(|item| item.code == UNSAFE_CONTROL_CLASS_CODE)
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
        observed_root: None,
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
            .any(|item| item.category == ARTIFACT_FILE_SHA256_DIFF_CATEGORY
                && item.severity == SafetySeverityV1::Info)
    );
    assert!(
        diff.warnings
            .iter()
            .any(|item| item.code == OBSERVATION_GAP_CODE)
    );
    assert_eq!(report.status, SafetyStatusV1::Warning);
}

#[test]
fn deployment_diff_blocks_missing_bootstrap_role_after_registry_observation() {
    let mut plan = sample_plan();
    plan.expected_canisters.push(ExpectedCanisterV1 {
        role: "user_hub".to_string(),
        canister_id: None,
        control_class: CanisterControlClassV1::DeploymentControlled,
    });
    let inventory = sample_matching_inventory();

    let diff = compare_plan_to_inventory(&plan, &inventory);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Blocked);
    assert!(diff.hard_failures.iter().any(|finding| {
        finding.code == SUBNET_REGISTRY_ROLE_MISSING_CODE
            && finding.subject.as_deref() == Some("user_hub")
    }));
}

#[test]
fn deployment_diff_does_not_claim_registry_corruption_when_registry_is_unobserved() {
    let mut plan = sample_plan();
    plan.expected_canisters.push(ExpectedCanisterV1 {
        role: "user_hub".to_string(),
        canister_id: None,
        control_class: CanisterControlClassV1::DeploymentControlled,
    });
    let mut inventory = sample_matching_inventory();
    inventory
        .unresolved_observations
        .push(DeploymentObservationGapV1 {
            key: "live_subnet_registry".to_string(),
            description: "registry query failed".to_string(),
        });

    let diff = compare_plan_to_inventory(&plan, &inventory);

    assert!(
        !diff
            .hard_failures
            .iter()
            .any(|finding| finding.code == SUBNET_REGISTRY_ROLE_MISSING_CODE)
    );
    assert!(
        diff.warnings
            .iter()
            .any(|finding| finding.code == CANISTER_UNOBSERVED_CODE)
    );
    assert!(
        diff.warnings
            .iter()
            .any(|finding| finding.code == OBSERVATION_GAP_CODE)
    );
}

#[test]
fn deployment_diff_warns_on_plan_assumptions_without_blocking() {
    let mut plan = sample_plan();
    plan.expected_canisters.clear();
    plan.role_artifacts[0].wasm_gz_sha256 = None;
    plan.expected_verifier_readiness.required = false;
    plan.unresolved_assumptions.push(DeploymentAssumptionV1 {
        key: DeploymentAssumptionKindV1::LocalStateMissing
            .key()
            .to_string(),
        description: "root identity is unknown until install".to_string(),
    });
    let inventory = DeploymentInventoryV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        inventory_id: "inventory-1".to_string(),
        observed_at: "2026-05-21T00:00:00Z".to_string(),
        observed_identity: Some(sample_identity()),
        observed_root: None,
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
            .any(|item| item.code == PLAN_ASSUMPTION_CODE
                && item.subject.as_deref()
                    == Some(DeploymentAssumptionKindV1::LocalStateMissing.key()))
    );
    assert_eq!(report.status, SafetyStatusV1::Warning);
}

#[test]
fn deployment_diff_blocks_unverified_registered_root_assumption() {
    let mut plan = sample_plan();
    plan.expected_canisters.clear();
    plan.role_artifacts[0].wasm_gz_sha256 = None;
    plan.expected_verifier_readiness.required = false;
    plan.unresolved_assumptions.push(DeploymentAssumptionV1 {
        key: "local_state.unverified_root_canister_id".to_string(),
        description: "registered root is not verified".to_string(),
    });
    let inventory = DeploymentInventoryV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        inventory_id: "inventory-1".to_string(),
        observed_at: "2026-05-21T00:00:00Z".to_string(),
        observed_identity: Some(sample_identity()),
        observed_root: None,
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

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Blocked);
    assert!(diff.warnings.is_empty());
    assert!(
        diff.hard_failures
            .iter()
            .any(|item| item.code == UNVERIFIED_DEPLOYMENT_ROOT_CODE
                && item.subject.as_deref() == Some("local_state.unverified_root_canister_id"))
    );
    assert_eq!(report.status, SafetyStatusV1::Blocked);
}
