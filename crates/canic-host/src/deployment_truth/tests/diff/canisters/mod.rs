use super::super::*;
use crate::deployment_truth::report::{
    CANISTER_DUPLICATE_DIFF_CATEGORY, CANISTER_EXTRA_DIFF_CATEGORY, CANISTER_ID_ROLE_CONFLICT_CODE,
    CANISTER_ID_ROLE_CONFLICT_DIFF_CATEGORY, CANISTER_ROLE_AMBIGUOUS_CODE,
    CANISTER_ROLE_AMBIGUOUS_DIFF_CATEGORY, CANISTER_ROLE_MISMATCH_CODE, CANISTER_UNOBSERVED_CODE,
    CONTROLLERS_UNOBSERVED_CODE, DUPLICATE_CANISTER_OBSERVED_CODE,
    DUPLICATE_PLANNED_CANISTER_ROLE_CODE, EXPECTED_CONTROLLER_MISSING_CODE,
    EXTRA_CANISTER_OBSERVED_CODE, PLANNED_CANISTER_DUPLICATE_DIFF_CATEGORY,
    PLANNED_CANISTER_ID_CONFLICT_CODE, PLANNED_CANISTER_ID_CONFLICT_DIFF_CATEGORY,
    PLANNED_CANISTER_ROLE_CONFLICT_CODE, PLANNED_CANISTER_ROLE_CONFLICT_DIFF_CATEGORY,
    ROLE_MISMATCH_DIFF_CATEGORY,
};

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
            .any(|finding| finding.code == CANISTER_UNOBSERVED_CODE
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
    assert!(diff.hard_failures.iter().any(|finding| finding.code
        == PLANNED_CANISTER_ROLE_CONFLICT_CODE
        && finding.subject.as_deref() == Some("root")));
    assert!(diff.controller_diff.iter().any(|item| {
        item.category == PLANNED_CANISTER_ROLE_CONFLICT_DIFF_CATEGORY
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
            .any(|finding| finding.code == PLANNED_CANISTER_ID_CONFLICT_CODE
                && finding.subject.as_deref() == Some("aaaaa-aa"))
    );
    assert!(diff.controller_diff.iter().any(|item| {
        item.category == PLANNED_CANISTER_ID_CONFLICT_DIFF_CATEGORY
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
    assert!(diff.warnings.iter().any(|finding| finding.code
        == DUPLICATE_PLANNED_CANISTER_ROLE_CODE
        && finding.subject.as_deref() == Some("root")));
    assert!(diff.controller_diff.iter().any(|item| {
        item.category == PLANNED_CANISTER_DUPLICATE_DIFF_CATEGORY
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
            .any(|finding| finding.code == EXTRA_CANISTER_OBSERVED_CODE
                && finding.subject.as_deref() == Some("user_hub"))
    );
    assert!(diff.controller_diff.iter().any(|item| {
        item.category == CANISTER_EXTRA_DIFF_CATEGORY
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
            .any(|finding| finding.code == EXTRA_CANISTER_OBSERVED_CODE
                && finding.subject.as_deref() == Some("root"))
    );
    assert!(diff.controller_diff.iter().any(|item| {
        item.category == CANISTER_EXTRA_DIFF_CATEGORY
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
            .any(|finding| finding.code == CANISTER_ROLE_AMBIGUOUS_CODE
                && finding.subject.as_deref() == Some("user_hub"))
    );
    assert!(diff.controller_diff.iter().any(|item| {
        item.category == CANISTER_ROLE_AMBIGUOUS_DIFF_CATEGORY
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
            .any(|finding| finding.code == CANISTER_ROLE_MISMATCH_CODE
                && finding.subject.as_deref() == Some("root"))
    );
    assert!(diff.controller_diff.iter().any(|item| {
        item.category == ROLE_MISMATCH_DIFF_CATEGORY
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
            .any(|finding| finding.code == CANISTER_ID_ROLE_CONFLICT_CODE
                && finding.subject.as_deref() == Some("aaaaa-aa"))
    );
    assert!(diff.controller_diff.iter().any(|item| {
        item.category == CANISTER_ID_ROLE_CONFLICT_DIFF_CATEGORY
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
            .any(|finding| finding.code == DUPLICATE_CANISTER_OBSERVED_CODE
                && finding.subject.as_deref() == Some("aaaaa-aa"))
    );
    assert!(diff.controller_diff.iter().any(|item| {
        item.category == CANISTER_DUPLICATE_DIFF_CATEGORY
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
            .any(|finding| finding.code == EXPECTED_CONTROLLER_MISSING_CODE
                && finding.subject.as_deref() == Some("user_hub"))
    );
    assert!(
        diff.warnings
            .iter()
            .all(|finding| finding.code != CONTROLLERS_UNOBSERVED_CODE)
    );
}
