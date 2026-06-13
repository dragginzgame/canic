use super::*;

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
