use super::*;

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
            .any(|item| item.code == "plan_assumption"
                && item.subject.as_deref() == Some("local_state.root_canister_id"))
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
            .any(|item| item.code == "unverified_deployment_root"
                && item.subject.as_deref() == Some("local_state.unverified_root_canister_id"))
    );
    assert_eq!(report.status, SafetyStatusV1::Blocked);
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
fn mainnet_deployment_check_blocks_cloud_engine_root_auth_signer() {
    let root = temp_dir("canic-deployment-cloud-engine-root-auth");
    let root_canister = "ryjl3-tyaaa-aaaaa-aaaba-cai";
    let mut inventory = sample_matching_inventory();
    let observed_root = inventory
        .observed_root
        .as_mut()
        .expect("sample has observed root");
    observed_root.network = "ic".to_string();
    observed_root.observed_canister_id = root_canister.to_string();
    let mut diff = compare_plan_to_inventory(&sample_plan(), &sample_matching_inventory());
    let source = FixtureRootSubnetEvidenceSource {
        result: Ok(RootSubnetEvidence {
            subnet_principal: "subnet-cloud-engine".to_string(),
            subnet_kind: "cloud_engine".to_string(),
        }),
    };

    super::report::apply_root_canister_signature_subnet_check_with_source(
        &mut diff, &inventory, "ic", &root, &source,
    );

    let _ = fs::remove_dir_all(root);
    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Blocked);
    assert!(diff.hard_failures.iter().any(|finding| {
        finding.code == "root_auth_cloud_engine_subnet"
            && finding.subject.as_deref() == Some(root_canister)
    }));
}
