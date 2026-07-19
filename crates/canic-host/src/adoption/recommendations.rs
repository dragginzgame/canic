use super::model::{
    AdoptionAuthorityStateV1, AdoptionOperatorActionRequirementV1, AdoptionProfileV1,
    AdoptionRecommendationSeverityV1, AdoptionRecommendationV1, AdoptionSuggestedActionEffectV1,
    AdoptionSuggestedActionSupportV1,
};

pub(super) fn observed_only_recommendations(
    profile: AdoptionProfileV1,
    fleet: &str,
    role: &str,
    authority_state: AdoptionAuthorityStateV1,
) -> Vec<AdoptionRecommendationV1> {
    if profile == AdoptionProfileV1::LeafOnly && is_leaf_only_authority_sensitive_role(role) {
        return Vec::new();
    }

    if authority_state != AdoptionAuthorityStateV1::CanicAuthorized {
        return vec![review_authority_before_declaration_recommendation(
            fleet,
            role,
            authority_state,
        )];
    }

    vec![declare_role_recommendation(fleet, role)]
}

pub(super) fn observed_only_warnings(profile: AdoptionProfileV1, role: &str) -> Vec<String> {
    if profile == AdoptionProfileV1::LeafOnly && is_leaf_only_authority_sensitive_role(role) {
        return vec![
            "leaf-only profile leaves authority-sensitive observed roles external".to_string(),
        ];
    }

    Vec::new()
}

pub(super) fn is_leaf_only_authority_sensitive_role(role: &str) -> bool {
    matches!(role, "root" | "governance" | "governance_root")
}

fn declare_role_recommendation(fleet: &str, role: &str) -> AdoptionRecommendationV1 {
    AdoptionRecommendationV1 {
        kind: "declare_role".to_string(),
        severity: AdoptionRecommendationSeverityV1::Info,
        description: format!("declare observed role candidate {fleet}.{role} before attachment"),
        suggested_action: Some(format!(
            "canic fleet role declare {fleet} {role} --package <path>"
        )),
        suggested_action_effect: AdoptionSuggestedActionEffectV1::MutatesState,
        suggested_action_support: AdoptionSuggestedActionSupportV1::UnsupportedByAdoption,
        operator_action_requirement: AdoptionOperatorActionRequirementV1::Required,
    }
}

fn review_authority_before_declaration_recommendation(
    fleet: &str,
    role: &str,
    authority_state: AdoptionAuthorityStateV1,
) -> AdoptionRecommendationV1 {
    AdoptionRecommendationV1 {
        kind: "review_authority_before_declaration".to_string(),
        severity: AdoptionRecommendationSeverityV1::Warning,
        description: format!(
            "review {fleet}.{role} authority before declaring observed role candidate ({})",
            adoption_authority_state_label(authority_state)
        ),
        suggested_action: None,
        suggested_action_effect: AdoptionSuggestedActionEffectV1::ReadOnly,
        suggested_action_support: AdoptionSuggestedActionSupportV1::SupportedByAdoption,
        operator_action_requirement: AdoptionOperatorActionRequirementV1::Required,
    }
}

const fn adoption_authority_state_label(authority_state: AdoptionAuthorityStateV1) -> &'static str {
    match authority_state {
        AdoptionAuthorityStateV1::CanicAuthorized => "canic-authorized",
        AdoptionAuthorityStateV1::UserControlled => "user-controlled",
        AdoptionAuthorityStateV1::External => "external",
        AdoptionAuthorityStateV1::Unknown => "unknown",
    }
}

pub(super) fn attach_later_recommendation(fleet: &str, role: &str) -> AdoptionRecommendationV1 {
    AdoptionRecommendationV1 {
        kind: "attach_role_later".to_string(),
        severity: AdoptionRecommendationSeverityV1::Info,
        description: format!("attach {fleet}.{role} explicitly only when topology is ready"),
        suggested_action: Some(format!(
            "canic fleet role attach {fleet} {role} --subnet <subnet>"
        )),
        suggested_action_effect: AdoptionSuggestedActionEffectV1::MutatesState,
        suggested_action_support: AdoptionSuggestedActionSupportV1::UnsupportedByAdoption,
        operator_action_requirement: AdoptionOperatorActionRequirementV1::Required,
    }
}

pub(super) fn blocked_actions() -> Vec<String> {
    [
        "controller changes",
        "topology attachment",
        "pool import",
        "install",
        "upgrade",
        "reinstall",
        "stop",
        "start",
        "delete",
        "deploy",
        "promote",
        "rollback",
        "artifact registry import",
    ]
    .into_iter()
    .map(str::to_string)
    .collect()
}
