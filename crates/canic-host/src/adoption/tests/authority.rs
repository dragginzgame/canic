use super::*;

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
                    == Some("canic app role declare demo candidate --package <path>")
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
            Some("canic app role declare demo candidate --package <path>"),
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
