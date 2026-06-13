use super::*;

#[test]
fn adoption_report_preserves_unresolved_evidence_gaps() {
    let mut inventory = inventory(Vec::new());
    inventory
        .unresolved_observations
        .push(DeploymentObservationGapV1 {
            key: "canister-status:api".to_string(),
            description: "status query failed".to_string(),
        });
    let mut manifest = external_api_artifact_manifest();
    manifest
        .unresolved_artifacts
        .push(DeploymentObservationGapV1 {
            key: "artifact:api".to_string(),
            description: "artifact file missing".to_string(),
        });

    let report = adoption_report_from_config_source(AdoptionReportRequest {
        report_id: "adoption-1",
        generated_at: "2026-05-30T00:00:00Z",
        profile: AdoptionProfileV1::Partial,
        config_source: CONFIG,
        inventory: Some(&inventory),
        artifact_manifest: Some(&manifest),
        package_metadata: Vec::new(),
    })
    .expect("adoption report");

    assert!(
        report
            .inputs
            .missing_or_stale_evidence
            .iter()
            .any(|evidence| {
                evidence
                    == "unresolved inventory observation canister-status:api: status query failed"
            })
    );
    assert!(
        report
            .inputs
            .missing_or_stale_evidence
            .iter()
            .any(|evidence| {
                evidence == "unresolved artifact evidence artifact:api: artifact file missing"
            })
    );
}

#[test]
fn adoption_report_marks_role_only_package_metadata_as_conflict() {
    let report = report(
        CONFIG,
        None,
        vec![AdoptionPackageMetadataV1 {
            package: "store".to_string(),
            fleet: None,
            role: Some("store".to_string()),
        }],
    );
    let store = role(&report, "store");

    assert_eq!(store.package_state, AdoptionPackageStateV1::MissingFleet);
    assert!(
        store
            .classifications
            .contains(&AdoptionClassificationV1::EvidenceConflict)
    );
}

#[test]
fn adoption_report_marks_duplicate_observed_role_as_evidence_conflict() {
    let inventory = inventory(vec![
        observed_canister(
            "aaaaa-aa",
            Some("api"),
            CanisterControlClassV1::DeploymentControlled,
            Some("api-hash-a"),
        ),
        observed_canister(
            "bbbbb-bb",
            Some("api"),
            CanisterControlClassV1::DeploymentControlled,
            Some("api-hash-b"),
        ),
    ]);
    let report = report(CONFIG, Some(&inventory), Vec::new());
    let api = role(&report, "api");

    assert_eq!(
        api.observation_state,
        AdoptionObservationStateV1::ConflictingMatch
    );
    assert!(
        api.classifications
            .contains(&AdoptionClassificationV1::EvidenceConflict)
    );
    assert_eq!(report.summary.evidence_conflicts, 1);
}

#[test]
fn adoption_report_marks_reverse_conflicting_artifact_evidence() {
    let mut inventory = inventory(Vec::new());
    inventory
        .observed_artifacts
        .push(observed_external_api_artifact());
    let manifest = RoleArtifactManifestV1 {
        schema_version: 1,
        manifest_id: "local-manifest-1".to_string(),
        network: "local".to_string(),
        artifact_root: None,
        role_artifacts: vec![RoleArtifactV1 {
            source: ArtifactSourceV1::LocalBuild,
            build_profile: "fast".to_string(),
            ..external_api_role_artifact()
        }],
        unresolved_artifacts: Vec::new(),
    };

    let report = adoption_report_from_config_source(AdoptionReportRequest {
        report_id: "artifact-conflict-2",
        generated_at: "2026-05-30T00:00:00Z",
        profile: AdoptionProfileV1::HybridExternalWasm,
        config_source: CONFIG,
        inventory: Some(&inventory),
        artifact_manifest: Some(&manifest),
        package_metadata: Vec::new(),
    })
    .expect("adoption report");
    let api = role(&report, "api");

    assert!(
        api.classifications
            .contains(&AdoptionClassificationV1::EvidenceConflict)
    );
    assert!(
        api.warnings
            .iter()
            .any(|warning| warning == "artifact evidence contains conflicting role facts")
    );
    assert_eq!(report.summary.evidence_conflicts, 1);
}

#[test]
fn adoption_report_marks_conflicting_artifact_evidence() {
    let mut inventory = inventory(Vec::new());
    inventory.observed_artifacts.push(ObservedArtifactV1 {
        role: "api".to_string(),
        artifact_path: "local/api.wasm.gz".to_string(),
        file_sha256: None,
        file_sha256_source: Some(ArtifactDigestSourceV1::ObservedFileDigest),
        payload_sha256: None,
        payload_size_bytes: None,
        source: ArtifactSourceV1::LocalBuild,
    });
    let manifest = external_api_artifact_manifest();
    let report = adoption_report_from_config_source(AdoptionReportRequest {
        report_id: "artifact-conflict-1",
        generated_at: "2026-05-30T00:00:00Z",
        profile: AdoptionProfileV1::HybridExternalWasm,
        config_source: CONFIG,
        inventory: Some(&inventory),
        artifact_manifest: Some(&manifest),
        package_metadata: Vec::new(),
    })
    .expect("adoption report");
    let api = role(&report, "api");

    assert!(
        api.classifications
            .contains(&AdoptionClassificationV1::EvidenceConflict)
    );
    assert!(
        api.warnings
            .iter()
            .any(|warning| warning == "artifact evidence contains conflicting role facts")
    );
    assert_eq!(report.summary.evidence_conflicts, 1);
}

#[test]
fn adoption_report_classifies_pool_candidates_as_resources() {
    let mut inventory = inventory(Vec::new());
    inventory.observed_pool.push(ObservedPoolCanisterV1 {
        pool: "users".to_string(),
        canister_id: "ccccc-cc".to_string(),
        role: Some("user_shard".to_string()),
        control_class: CanisterControlClassV1::UnknownUnsafe,
    });

    let report = report(CONFIG, Some(&inventory), Vec::new());
    let pool = report
        .observed_canisters
        .iter()
        .find(|finding| finding.canister_id == "ccccc-cc")
        .expect("pool candidate finding");

    assert!(
        pool.classifications
            .contains(&AdoptionClassificationV1::ImportedPoolCandidate)
    );
    assert_eq!(pool.matched_role.as_deref(), Some("user_shard"));
}

#[test]
fn hybrid_external_wasm_fixture_reports_hashes_without_import() {
    let mut inventory = inventory(vec![observed_canister(
        "bbbbb-bb",
        Some("api"),
        CanisterControlClassV1::DeploymentControlled,
        Some("api-module-hash"),
    )]);
    inventory
        .observed_artifacts
        .push(observed_external_api_artifact());
    let manifest = external_api_artifact_manifest();

    let report = adoption_report_from_config_source(AdoptionReportRequest {
        report_id: "hybrid-1",
        generated_at: "2026-05-30T00:00:00Z",
        profile: AdoptionProfileV1::HybridExternalWasm,
        config_source: CONFIG,
        inventory: Some(&inventory),
        artifact_manifest: Some(&manifest),
        package_metadata: Vec::new(),
    })
    .expect("adoption report");
    let api = role(&report, "api");
    let observed_api = report
        .observed_canisters
        .iter()
        .find(|finding| finding.matched_role.as_deref() == Some("api"))
        .expect("api observation");

    assert_eq!(api.artifact_state, AdoptionArtifactStateV1::ExternalWasm);
    assert!(
        api.evidence
            .iter()
            .any(|evidence| evidence == "observed canister module_hash=api-module-hash")
    );
    assert!(
        api.evidence
            .iter()
            .any(|evidence| evidence == "artifact manifest source=external")
    );
    assert!(
        api.evidence
            .iter()
            .any(|evidence| evidence
                == "artifact manifest installed_module_hash=api-installed-module")
    );
    assert!(
        api.evidence
            .iter()
            .any(|evidence| evidence == "observed artifact file_sha256=api-file-sha")
    );
    assert!(
        api.warnings
            .iter()
            .any(|warning| warning.contains("artifact registry import is outside"))
    );
    assert_eq!(
        observed_api.wasm_evidence.as_deref(),
        Some("module_hash=api-module-hash")
    );
    assert!(
        report
            .blocked_actions
            .contains(&"artifact registry import".to_string())
    );
    assert!(
        report
            .recommendations
            .iter()
            .all(|recommendation| !recommendation.kind.contains("artifact"))
    );
}
