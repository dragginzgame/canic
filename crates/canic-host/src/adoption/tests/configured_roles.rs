use super::*;

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
