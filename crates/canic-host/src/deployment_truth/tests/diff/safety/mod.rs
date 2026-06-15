use super::super::*;

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

    crate::deployment_truth::report::apply_root_canister_signature_subnet_check_with_source(
        &mut diff, &inventory, "ic", &root, &source,
    );

    let _ = fs::remove_dir_all(root);
    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Blocked);
    assert!(diff.hard_failures.iter().any(|finding| {
        finding.code == "root_auth_cloud_engine_subnet"
            && finding.subject.as_deref() == Some(root_canister)
    }));
}
