use super::super::*;
use crate::deployment_truth::report::{
    ARTIFACT_DUPLICATE_DIFF_CATEGORY, ARTIFACT_FILE_DIGEST_MISMATCH_CODE,
    ARTIFACT_FILE_SHA256_DIFF_CATEGORY, ARTIFACT_ROLE_CONFLICT_CODE,
    ARTIFACT_ROLE_CONFLICT_DIFF_CATEGORY, DUPLICATE_ARTIFACT_OBSERVED_CODE,
    DUPLICATE_PLANNED_ARTIFACT_ROLE_CODE, PLANNED_ARTIFACT_DUPLICATE_DIFF_CATEGORY,
    PLANNED_ARTIFACT_ROLE_CONFLICT_CODE, PLANNED_ARTIFACT_ROLE_CONFLICT_DIFF_CATEGORY,
};

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
            .any(|finding| finding.code == ARTIFACT_FILE_DIGEST_MISMATCH_CODE)
    );
    assert!(diff.artifact_diff.iter().any(|item| {
        item.category == ARTIFACT_FILE_SHA256_DIFF_CATEGORY
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
            .any(|finding| finding.code == ARTIFACT_ROLE_CONFLICT_CODE
                && finding.subject.as_deref() == Some("root"))
    );
    assert!(diff.artifact_diff.iter().any(|item| {
        item.category == ARTIFACT_ROLE_CONFLICT_DIFF_CATEGORY
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
            .any(|finding| finding.code == DUPLICATE_ARTIFACT_OBSERVED_CODE
                && finding.subject.as_deref() == Some("root"))
    );
    assert!(diff.artifact_diff.iter().any(|item| {
        item.category == ARTIFACT_DUPLICATE_DIFF_CATEGORY
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
    assert!(diff.hard_failures.iter().any(|finding| finding.code
        == PLANNED_ARTIFACT_ROLE_CONFLICT_CODE
        && finding.subject.as_deref() == Some("root")));
    assert!(diff.artifact_diff.iter().any(|item| {
        item.category == PLANNED_ARTIFACT_ROLE_CONFLICT_DIFF_CATEGORY
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
    assert!(diff.warnings.iter().any(|finding| finding.code
        == DUPLICATE_PLANNED_ARTIFACT_ROLE_CODE
        && finding.subject.as_deref() == Some("root")));
    assert!(diff.artifact_diff.iter().any(|item| {
        item.category == PLANNED_ARTIFACT_DUPLICATE_DIFF_CATEGORY
            && item.subject == "root"
            && item.observed.as_deref() == Some("2")
            && item.severity == SafetySeverityV1::Warning
    }));
}
