use super::super::*;

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
        observed_root: None,
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
