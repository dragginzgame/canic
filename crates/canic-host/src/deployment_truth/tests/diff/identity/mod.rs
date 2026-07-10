use super::super::*;
use crate::deployment_truth::report::{
    DEPLOYMENT_MANIFEST_MISMATCH_CODE, INSTALLED_MODULE_HASH_AMBIGUOUS_CODE,
    INSTALLED_MODULE_HASH_AMBIGUOUS_DIFF_CATEGORY, INSTALLED_MODULE_HASH_DIFF_CATEGORY,
    INSTALLED_MODULE_HASH_MISMATCH_CODE, RAW_CONFIG_DIGEST_MISMATCH_CODE,
    RAW_CONFIG_SHA256_DIFF_CATEGORY,
};

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
        observed_root: None,
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
            .any(|finding| finding.code == DEPLOYMENT_MANIFEST_MISMATCH_CODE)
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
        observed_root: None,
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
            .any(|finding| finding.code == RAW_CONFIG_DIGEST_MISMATCH_CODE)
    );
    assert!(diff.embedded_config_diff.iter().any(|item| {
        item.category == RAW_CONFIG_SHA256_DIFF_CATEGORY
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
        observed_root: None,
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
            role_assignment_source: Some(
                RoleAssignmentSourceV1::IcpCanisterStatus
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
            .any(|finding| finding.code == INSTALLED_MODULE_HASH_MISMATCH_CODE)
    );
    assert!(diff.module_hash_diff.iter().any(|item| {
        item.category == INSTALLED_MODULE_HASH_DIFF_CATEGORY
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
        role_assignment_source: Some(
            RoleAssignmentSourceV1::SubnetRegistryAndIcpCanisterStatus
                .label()
                .to_string(),
        ),
    });

    let diff = compare_plan_to_inventory(&plan, &inventory);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Warning);
    assert!(
        diff.hard_failures
            .iter()
            .all(|finding| finding.code != INSTALLED_MODULE_HASH_MISMATCH_CODE)
    );
    assert!(
        diff.module_hash_diff
            .iter()
            .all(|item| item.category != INSTALLED_MODULE_HASH_DIFF_CATEGORY)
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
        role_assignment_source: Some(
            RoleAssignmentSourceV1::SubnetRegistryAndIcpCanisterStatus
                .label()
                .to_string(),
        ),
    });

    let diff = compare_plan_to_inventory(&plan, &inventory);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Blocked);
    assert!(diff.hard_failures.iter().any(|finding| finding.code
        == INSTALLED_MODULE_HASH_AMBIGUOUS_CODE
        && finding.subject.as_deref() == Some("root")));
    assert!(diff.module_hash_diff.iter().any(|item| {
        item.category == INSTALLED_MODULE_HASH_AMBIGUOUS_DIFF_CATEGORY
            && item.subject == "root"
            && item.observed.as_deref().is_some_and(|observed| {
                observed.contains("aaaaa-aa") && observed.contains("duplicate-root-id")
            })
            && item.severity == SafetySeverityV1::HardFailure
    }));
}
