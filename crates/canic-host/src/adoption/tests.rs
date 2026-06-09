use super::*;
use crate::deployment_truth::{
    ArtifactDigestSourceV1, DeploymentInventoryV1, DeploymentObservationGapV1,
    DeploymentRootObservationSourceV1, DeploymentRootObservationV1, LocalDeploymentConfigV1,
    ObservationStatusV1, ObservedArtifactV1, ObservedCanisterV1, ObservedPoolCanisterV1,
    RoleArtifactManifestV1, RoleArtifactV1, VerifierReadinessObservationV1,
};

const CONFIG: &str = r#"
controllers = []
app_index = []

[fleet]
name = "demo"

[roles.root]
kind = "root"
package = "root"

[roles.api]
kind = "canister"
package = "api"

[roles.store]
kind = "canister"
package = "store"

[subnets.prime.canisters.root]
kind = "root"

[subnets.prime.canisters.api]
kind = "service"
"#;

const BROWNFIELD_CONFIG: &str = r#"
controllers = []
app_index = []

[fleet]
name = "demo"

[roles.root]
kind = "root"
package = "root"

[subnets.prime.canisters.root]
kind = "root"
"#;

const STANDALONE_CONFIG: &str = r#"
controllers = []
app_index = []

[fleet]
name = "demo"

[roles.worker]
kind = "canister"
package = "worker"
"#;

const LEAF_ONLY_CONFIG: &str = r#"
controllers = []
app_index = []

[fleet]
name = "demo"

[roles.root]
kind = "root"
package = "root"

[roles.app]
kind = "canister"
package = "app"

[subnets.prime.canisters.app]
kind = "service"

[subnets.prime.canisters.root]
kind = "root"
"#;

#[test]
fn adoption_report_preserves_declared_only_as_non_deployable() {
    let report = report(CONFIG, None, Vec::new());
    let store = role(&report, "store");

    assert_eq!(
        store.declaration_state,
        AdoptionDeclarationStateV1::Declared
    );
    assert_eq!(store.topology_state, AdoptionTopologyStateV1::Unattached);
    assert!(
        store
            .classifications
            .contains(&AdoptionClassificationV1::DeclaredOnly)
    );
    assert_eq!(report.summary.declared_only_roles, 1);
    assert_eq!(report.summary.mutating_actions_performed, 0);
    assert!(store.recommendations.iter().all(|recommendation| {
        recommendation.suggested_action_availability
            == AdoptionSuggestedActionAvailabilityV1::BlockedIn0500
    }));
    assert!(report.blocked_actions.contains(&"install".to_string()));
}

#[test]
fn brownfield_fixture_reports_observed_roles_without_claiming_them() {
    let inventory = inventory(vec![
        observed_canister(
            "aaaaa-aa",
            Some("root"),
            CanisterControlClassV1::DeploymentControlled,
            None,
        ),
        observed_canister(
            "bbbbb-bb",
            Some("api"),
            CanisterControlClassV1::UserControlled,
            Some("api-hash"),
        ),
        observed_canister(
            "ccccc-cc",
            Some("store"),
            CanisterControlClassV1::ExternallyImported,
            Some("store-hash"),
        ),
        observed_canister(
            "ddddd-dd",
            None,
            CanisterControlClassV1::UnknownUnsafe,
            Some("unknown-hash"),
        ),
    ]);
    let report = report_with_profile(
        AdoptionProfileV1::Brownfield,
        BROWNFIELD_CONFIG,
        Some(&inventory),
        Vec::new(),
    );
    let api = role(&report, "api");
    let store = role(&report, "store");

    assert_eq!(report.profile, AdoptionProfileV1::Brownfield);
    assert_eq!(report.summary.managed_configured_roles, 1);
    assert_eq!(report.summary.observed_only_canisters, 3);
    assert_eq!(report.summary.user_controlled_canisters, 1);
    assert_eq!(report.summary.external_controller_required, 2);
    assert_eq!(report.summary.mutating_actions_performed, 0);
    assert!(
        api.classifications
            .contains(&AdoptionClassificationV1::ObservedOnly)
    );
    assert_eq!(
        api.authority_state,
        AdoptionAuthorityStateV1::UserControlled
    );
    assert!(
        store
            .classifications
            .contains(&AdoptionClassificationV1::ExternalControllerRequired)
    );
    assert_eq!(store.artifact_state, AdoptionArtifactStateV1::ExternalWasm);
    assert!(
        report
            .recommendations
            .iter()
            .filter(|recommendation| recommendation.kind == "declare_role")
            .all(|recommendation| recommendation.suggested_action_support
                == AdoptionSuggestedActionSupportV1::UnsupportedByAdoption
                && recommendation.suggested_action_availability
                    == AdoptionSuggestedActionAvailabilityV1::BlockedIn0500)
    );
}

#[test]
fn partial_fixture_preserves_managed_and_external_roles_separately() {
    let inventory = inventory(vec![
        observed_canister(
            "aaaaa-aa",
            Some("root"),
            CanisterControlClassV1::DeploymentControlled,
            None,
        ),
        observed_canister(
            "bbbbb-bb",
            Some("api"),
            CanisterControlClassV1::DeploymentControlled,
            Some("api-hash"),
        ),
        observed_canister(
            "ccccc-cc",
            Some("external_app"),
            CanisterControlClassV1::UserControlled,
            Some("external_app-hash"),
        ),
    ]);
    let report = report_with_profile(
        AdoptionProfileV1::Partial,
        CONFIG,
        Some(&inventory),
        Vec::new(),
    );
    let api = role(&report, "api");
    let store = role(&report, "store");
    let external_app = role(&report, "external_app");

    assert_eq!(report.profile, AdoptionProfileV1::Partial);
    assert_eq!(report.summary.managed_configured_roles, 2);
    assert_eq!(report.summary.declared_only_roles, 1);
    assert_eq!(report.summary.attached_unobserved_roles, 0);
    assert_eq!(report.summary.observed_only_canisters, 1);
    assert_eq!(api.observation_state, AdoptionObservationStateV1::Observed);
    assert!(
        api.classifications
            .contains(&AdoptionClassificationV1::Managed)
    );
    assert!(
        store
            .classifications
            .contains(&AdoptionClassificationV1::DeclaredOnly)
    );
    assert_eq!(
        external_app.authority_state,
        AdoptionAuthorityStateV1::UserControlled
    );
}

#[test]
fn external_controller_fixture_reports_external_action_boundary() {
    let inventory = inventory(vec![observed_canister(
        "bbbbb-bb",
        Some("api"),
        CanisterControlClassV1::JointlyControlled,
        Some("api-hash"),
    )]);
    let report = report(CONFIG, Some(&inventory), Vec::new());
    let api = role(&report, "api");

    assert!(
        api.classifications
            .contains(&AdoptionClassificationV1::Managed)
    );
    assert!(
        api.classifications
            .contains(&AdoptionClassificationV1::ExternalControllerRequired)
    );
    assert_eq!(api.authority_state, AdoptionAuthorityStateV1::External);
    assert_eq!(report.summary.external_controller_required, 1);
    assert!(
        report
            .blocked_actions
            .contains(&"controller changes".to_string())
    );
}

#[test]
fn observed_only_fixture_without_role_stays_unmatched() {
    let inventory = inventory(vec![observed_canister(
        "zzzzz-zz",
        None,
        CanisterControlClassV1::UnknownUnsafe,
        Some("unknown-hash"),
    )]);
    let report = report(BROWNFIELD_CONFIG, Some(&inventory), Vec::new());
    let observed = report
        .observed_canisters
        .iter()
        .find(|finding| finding.canister_id == "zzzzz-zz")
        .expect("observed-only canister finding");

    assert_eq!(report.summary.observed_only_canisters, 1);
    assert_eq!(observed.matched_role, None);
    assert_eq!(observed.confidence, AdoptionMatchConfidenceV1::None);
    assert!(
        observed
            .classifications
            .contains(&AdoptionClassificationV1::ObservedOnly)
    );
    assert!(
        observed.recommendations.is_empty(),
        "name-free observations must not invent declaration actions"
    );
}

#[test]
fn declared_only_fixture_reports_compile_only_role() {
    let report = report_with_profile(
        AdoptionProfileV1::Partial,
        CONFIG,
        None,
        matching_metadata(),
    );
    let store = role(&report, "store");

    assert_eq!(store.package_state, AdoptionPackageStateV1::Matches);
    assert_eq!(store.topology_state, AdoptionTopologyStateV1::Unattached);
    assert!(
        store
            .classifications
            .contains(&AdoptionClassificationV1::DeclaredOnly)
    );
    assert!(
        store
            .recommendations
            .iter()
            .any(|recommendation| recommendation.kind == "attach_role_later"
                && recommendation.suggested_action_effect
                    == AdoptionSuggestedActionEffectV1::MutatesState
                && recommendation.suggested_action_support
                    == AdoptionSuggestedActionSupportV1::UnsupportedByAdoption)
    );
}

#[test]
fn standalone_fixture_keeps_compile_only_role_unattached() {
    let report = report_with_profile(
        AdoptionProfileV1::Standalone,
        STANDALONE_CONFIG,
        None,
        vec![AdoptionPackageMetadataV1 {
            package: "worker".to_string(),
            fleet: Some("demo".to_string()),
            role: Some("worker".to_string()),
        }],
    );
    let worker = role(&report, "worker");

    assert_eq!(report.profile, AdoptionProfileV1::Standalone);
    assert_eq!(report.summary.managed_configured_roles, 0);
    assert_eq!(report.summary.declared_only_roles, 1);
    assert_eq!(report.summary.attached_unobserved_roles, 0);
    assert_eq!(worker.package_state, AdoptionPackageStateV1::Matches);
    assert_eq!(worker.topology_state, AdoptionTopologyStateV1::Unattached);
    assert_eq!(
        worker.observation_state,
        AdoptionObservationStateV1::Unobserved
    );
    assert!(
        worker
            .classifications
            .contains(&AdoptionClassificationV1::DeclaredOnly)
    );
    assert!(
        worker
            .evidence
            .iter()
            .any(|evidence| evidence == "no topology attachment exists")
    );
    assert!(
        report
            .blocked_actions
            .contains(&"topology attachment".to_string())
    );
}

#[test]
fn leaf_only_fixture_does_not_recommend_authority_hub_adoption() {
    let inventory = inventory(vec![
        observed_canister(
            "aaaaa-aa",
            Some("governance"),
            CanisterControlClassV1::UserControlled,
            Some("governance-hash"),
        ),
        observed_canister(
            "bbbbb-bb",
            Some("app"),
            CanisterControlClassV1::DeploymentControlled,
            Some("app-hash"),
        ),
    ]);
    let report = report_with_profile(
        AdoptionProfileV1::LeafOnly,
        LEAF_ONLY_CONFIG,
        Some(&inventory),
        Vec::new(),
    );
    let app = role(&report, "app");
    let governance = role(&report, "governance");
    let governance_observation = report
        .observed_canisters
        .iter()
        .find(|finding| finding.matched_role.as_deref() == Some("governance"))
        .expect("governance observation");

    assert_eq!(report.profile, AdoptionProfileV1::LeafOnly);
    assert!(
        app.classifications
            .contains(&AdoptionClassificationV1::Managed)
    );
    assert_eq!(app.observation_state, AdoptionObservationStateV1::Observed);
    assert!(
        governance
            .classifications
            .contains(&AdoptionClassificationV1::ObservedOnly)
    );
    assert!(
        governance
            .classifications
            .contains(&AdoptionClassificationV1::ExternalControllerRequired)
    );
    assert!(governance.recommendations.is_empty());
    assert!(
        governance
            .warnings
            .iter()
            .any(|warning| warning.contains("leaf-only profile"))
    );
    assert!(governance_observation.recommendations.is_empty());
    assert!(
        governance_observation
            .warnings
            .iter()
            .any(|warning| warning.contains("leaf-only profile"))
    );
    assert!(
        !report
            .recommendations
            .iter()
            .any(|recommendation| recommendation.suggested_action.as_deref()
                == Some("canic fleet role declare demo governance --package <path>"))
    );
}

#[test]
fn adoption_report_reports_attached_unobserved_without_teardown_inference() {
    let report = report(CONFIG, None, Vec::new());
    let api = role(&report, "api");

    assert!(
        api.classifications
            .contains(&AdoptionClassificationV1::Managed)
    );
    assert!(
        api.classifications
            .contains(&AdoptionClassificationV1::AttachedUnobserved)
    );
    assert_eq!(
        api.observation_state,
        AdoptionObservationStateV1::Unobserved
    );
    assert!(
        api.warnings
            .iter()
            .any(|warning| warning.contains("does not confirm"))
    );
}

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
fn adoption_report_classifies_observed_only_user_controlled_canister() {
    let inventory = inventory(vec![observed_canister(
        "aaaaa-aa",
        Some("external_app"),
        CanisterControlClassV1::UserControlled,
        Some("external_app-hash"),
    )]);
    let report = report(CONFIG, Some(&inventory), Vec::new());
    let external_app = role(&report, "external_app");

    assert_eq!(
        external_app.declaration_state,
        AdoptionDeclarationStateV1::Undeclared
    );
    assert!(
        external_app
            .classifications
            .contains(&AdoptionClassificationV1::ObservedOnly)
    );
    assert!(
        external_app
            .classifications
            .contains(&AdoptionClassificationV1::UserControlled)
    );
    assert!(
        external_app
            .classifications
            .contains(&AdoptionClassificationV1::ExternalControllerRequired)
    );
    assert_eq!(report.summary.observed_only_canisters, 1);
    assert_eq!(report.summary.user_controlled_canisters, 1);
    assert!(
        report
            .recommendations
            .iter()
            .any(
                |recommendation| recommendation.kind == "review_authority_before_declaration"
                    && recommendation.suggested_action.is_none()
                    && recommendation.suggested_action_effect
                        == AdoptionSuggestedActionEffectV1::ReadOnly
                    && recommendation.suggested_action_support
                        == AdoptionSuggestedActionSupportV1::SupportedByAdoption
            )
    );
    assert!(
        report
            .recommendations
            .iter()
            .all(|recommendation| recommendation.kind != "declare_role")
    );
}

#[test]
fn adoption_report_recommends_declaration_only_for_canic_authorized_observed_role() {
    let inventory = inventory(vec![observed_canister(
        "aaaaa-aa",
        Some("candidate"),
        CanisterControlClassV1::DeploymentControlled,
        Some("candidate-hash"),
    )]);
    let report = report(BROWNFIELD_CONFIG, Some(&inventory), Vec::new());
    let candidate = role(&report, "candidate");

    assert_eq!(
        candidate.authority_state,
        AdoptionAuthorityStateV1::CanicAuthorized
    );
    assert!(
        report
            .recommendations
            .iter()
            .any(|recommendation| recommendation.kind == "declare_role"
                && recommendation.suggested_action.as_deref()
                    == Some("canic fleet role declare demo candidate --package <path>")
                && recommendation.suggested_action_effect
                    == AdoptionSuggestedActionEffectV1::MutatesState
                && recommendation.suggested_action_support
                    == AdoptionSuggestedActionSupportV1::UnsupportedByAdoption)
    );
}

#[test]
fn adoption_report_authority_gates_observed_only_declaration_recommendations() {
    for (
        control_class,
        expected_authority,
        expected_recommendation_kind,
        expected_suggested_action,
    ) in [
        (
            CanisterControlClassV1::DeploymentControlled,
            AdoptionAuthorityStateV1::CanicAuthorized,
            "declare_role",
            Some("canic fleet role declare demo candidate --package <path>"),
        ),
        (
            CanisterControlClassV1::UserControlled,
            AdoptionAuthorityStateV1::UserControlled,
            "review_authority_before_declaration",
            None,
        ),
        (
            CanisterControlClassV1::ExternallyImported,
            AdoptionAuthorityStateV1::External,
            "review_authority_before_declaration",
            None,
        ),
        (
            CanisterControlClassV1::UnknownUnsafe,
            AdoptionAuthorityStateV1::Unknown,
            "review_authority_before_declaration",
            None,
        ),
    ] {
        let inventory = inventory(vec![observed_canister(
            "aaaaa-aa",
            Some("candidate"),
            control_class,
            Some("candidate-hash"),
        )]);
        let report = report(BROWNFIELD_CONFIG, Some(&inventory), Vec::new());
        let candidate = role(&report, "candidate");
        let recommendation = candidate
            .recommendations
            .first()
            .expect("authority-gated recommendation");

        assert_eq!(candidate.authority_state, expected_authority);
        assert_eq!(recommendation.kind, expected_recommendation_kind);
        assert_eq!(
            recommendation.suggested_action.as_deref(),
            expected_suggested_action
        );
        if expected_authority == AdoptionAuthorityStateV1::CanicAuthorized {
            assert_eq!(
                recommendation.suggested_action_availability,
                AdoptionSuggestedActionAvailabilityV1::BlockedIn0500
            );
            assert_eq!(
                recommendation.suggested_action_support,
                AdoptionSuggestedActionSupportV1::UnsupportedByAdoption
            );
        } else {
            assert!(
                candidate
                    .recommendations
                    .iter()
                    .all(|recommendation| recommendation.kind != "declare_role")
            );
        }
    }
}

#[test]
fn adoption_report_keeps_managed_separate_from_authority() {
    let inventory = inventory(vec![observed_canister(
        "aaaaa-aa",
        Some("api"),
        CanisterControlClassV1::UserControlled,
        Some("api-hash"),
    )]);
    let report = report(CONFIG, Some(&inventory), Vec::new());
    let api = role(&report, "api");

    assert!(
        api.classifications
            .contains(&AdoptionClassificationV1::Managed)
    );
    assert!(
        api.classifications
            .contains(&AdoptionClassificationV1::ExternalControllerRequired)
    );
    assert_eq!(
        api.authority_state,
        AdoptionAuthorityStateV1::UserControlled
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
fn adoption_report_round_trips_through_json() {
    let manifest = RoleArtifactManifestV1 {
        schema_version: 1,
        manifest_id: "manifest-1".to_string(),
        network: "local".to_string(),
        artifact_root: None,
        role_artifacts: vec![RoleArtifactV1 {
            role: "api".to_string(),
            source: ArtifactSourceV1::LocalBuild,
            build_profile: "fast".to_string(),
            wasm_path: None,
            wasm_gz_path: None,
            wasm_gz_size_bytes: None,
            wasm_sha256: None,
            wasm_gz_sha256: None,
            wasm_gz_sha256_source: None,
            observed_wasm_gz_file_sha256: None,
            observed_wasm_gz_file_sha256_source: None,
            installed_module_hash: None,
            candid_path: None,
            candid_sha256: None,
            raw_config_sha256: None,
            canonical_embedded_config_sha256: None,
            embedded_topology_sha256: None,
            builder_version: None,
            rust_toolchain: None,
            package_version: None,
        }],
        unresolved_artifacts: Vec::new(),
    };
    let report = adoption_report_from_config_source(AdoptionReportRequest {
        report_id: "adoption-1",
        generated_at: "2026-05-30T00:00:00Z",
        profile: AdoptionProfileV1::Brownfield,
        config_source: CONFIG,
        inventory: None,
        artifact_manifest: Some(&manifest),
        package_metadata: Vec::new(),
    })
    .expect("adoption report");

    let encoded = serde_json::to_string(&report).expect("encode report");
    let decoded = serde_json::from_str::<AdoptionReportV1>(&encoded).expect("decode report");

    assert_eq!(decoded, report);
    assert_eq!(
        role(&decoded, "api").artifact_state,
        AdoptionArtifactStateV1::CanicBuilt
    );
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

fn report(
    config_source: &str,
    inventory: Option<&DeploymentInventoryV1>,
    package_metadata: Vec<AdoptionPackageMetadataV1>,
) -> AdoptionReportV1 {
    report_with_profile(
        AdoptionProfileV1::Brownfield,
        config_source,
        inventory,
        package_metadata,
    )
}

fn report_with_profile(
    profile: AdoptionProfileV1,
    config_source: &str,
    inventory: Option<&DeploymentInventoryV1>,
    package_metadata: Vec<AdoptionPackageMetadataV1>,
) -> AdoptionReportV1 {
    adoption_report_from_config_source(AdoptionReportRequest {
        report_id: "adoption-1",
        generated_at: "2026-05-30T00:00:00Z",
        profile,
        config_source,
        inventory,
        artifact_manifest: None,
        package_metadata,
    })
    .expect("adoption report")
}

fn matching_metadata() -> Vec<AdoptionPackageMetadataV1> {
    ["root", "api", "store"]
        .into_iter()
        .map(|package| AdoptionPackageMetadataV1 {
            package: package.to_string(),
            fleet: Some("demo".to_string()),
            role: Some(package.to_string()),
        })
        .collect()
}

fn external_api_artifact_manifest() -> RoleArtifactManifestV1 {
    RoleArtifactManifestV1 {
        schema_version: 1,
        manifest_id: "external-manifest-1".to_string(),
        network: "local".to_string(),
        artifact_root: None,
        role_artifacts: vec![external_api_role_artifact()],
        unresolved_artifacts: Vec::new(),
    }
}

fn external_api_role_artifact() -> RoleArtifactV1 {
    RoleArtifactV1 {
        role: "api".to_string(),
        source: ArtifactSourceV1::External,
        build_profile: "external".to_string(),
        wasm_path: Some("external/api.wasm".to_string()),
        wasm_gz_path: Some("external/api.wasm.gz".to_string()),
        wasm_gz_size_bytes: Some(42),
        wasm_sha256: Some("api-wasm-sha".to_string()),
        wasm_gz_sha256: Some("api-wasm-gz-sha".to_string()),
        wasm_gz_sha256_source: Some(ArtifactDigestSourceV1::ObservedFileDigest),
        observed_wasm_gz_file_sha256: Some("api-file-sha".to_string()),
        observed_wasm_gz_file_sha256_source: Some(ArtifactDigestSourceV1::ObservedFileDigest),
        installed_module_hash: Some("api-installed-module".to_string()),
        candid_path: None,
        candid_sha256: None,
        raw_config_sha256: None,
        canonical_embedded_config_sha256: None,
        embedded_topology_sha256: None,
        builder_version: None,
        rust_toolchain: None,
        package_version: None,
    }
}

fn observed_external_api_artifact() -> ObservedArtifactV1 {
    ObservedArtifactV1 {
        role: "api".to_string(),
        artifact_path: "external/api.wasm.gz".to_string(),
        file_sha256: Some("api-file-sha".to_string()),
        file_sha256_source: Some(ArtifactDigestSourceV1::ObservedFileDigest),
        payload_sha256: Some("api-payload-sha".to_string()),
        payload_size_bytes: Some(42),
        source: ArtifactSourceV1::External,
    }
}

fn role<'a>(report: &'a AdoptionReportV1, role: &str) -> &'a AdoptionRoleFindingV1 {
    report
        .role_findings
        .iter()
        .find(|finding| finding.role == role)
        .expect("role finding")
}

fn inventory(observed_canisters: Vec<ObservedCanisterV1>) -> DeploymentInventoryV1 {
    DeploymentInventoryV1 {
        schema_version: 1,
        inventory_id: "inventory-1".to_string(),
        observed_at: "2026-05-30T00:00:00Z".to_string(),
        observed_identity: None,
        observed_root: Some(DeploymentRootObservationV1 {
            deployment_name: "demo-dev".to_string(),
            network: "local".to_string(),
            fleet_template: "demo".to_string(),
            root_principal: "aaaaa-aa".to_string(),
            observed_canister_id: "aaaaa-aa".to_string(),
            observation_source: DeploymentRootObservationSourceV1::LocalDeploymentState,
            control_class: CanisterControlClassV1::DeploymentControlled,
            controllers: vec!["aaaaa-aa".to_string()],
            module_hash: None,
            status: Some("running".to_string()),
            role_assignment_source: Some("local-state".to_string()),
        }),
        local_config: LocalDeploymentConfigV1 {
            config_path: Some("fleets/demo/canic.toml".to_string()),
            raw_config_sha256: None,
            canonical_embedded_config_sha256: None,
        },
        observed_canisters,
        observed_pool: Vec::new(),
        observed_artifacts: vec![ObservedArtifactV1 {
            role: "external_app".to_string(),
            artifact_path: "observed:external_app".to_string(),
            file_sha256: None,
            file_sha256_source: Some(ArtifactDigestSourceV1::InstalledModuleHash),
            payload_sha256: None,
            payload_size_bytes: None,
            source: ArtifactSourceV1::External,
        }],
        observed_verifier_readiness: VerifierReadinessObservationV1 {
            status: ObservationStatusV1::NotObserved,
            role_epochs: Vec::new(),
        },
        unresolved_observations: Vec::new(),
    }
}

fn observed_canister(
    canister_id: &str,
    role: Option<&str>,
    control_class: CanisterControlClassV1,
    module_hash: Option<&str>,
) -> ObservedCanisterV1 {
    ObservedCanisterV1 {
        canister_id: canister_id.to_string(),
        role: role.map(str::to_string),
        control_class,
        controllers: vec!["controller".to_string()],
        module_hash: module_hash.map(str::to_string),
        status: Some("running".to_string()),
        root_trust_anchor: Some("root".to_string()),
        canonical_embedded_config_digest: None,
        role_assignment_source: role.map(|_| "explicit-test-evidence".to_string()),
    }
}
