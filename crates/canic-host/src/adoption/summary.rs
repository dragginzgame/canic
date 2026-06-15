use super::model::{
    AdoptionClassificationV1, AdoptionObservedCanisterFindingV1, AdoptionReportSummaryV1,
    AdoptionRoleFindingV1,
};

pub(super) fn report_summary(
    role_findings: &[AdoptionRoleFindingV1],
    observed_findings: &[AdoptionObservedCanisterFindingV1],
) -> AdoptionReportSummaryV1 {
    AdoptionReportSummaryV1 {
        managed_configured_roles: role_findings
            .iter()
            .filter(|finding| {
                finding
                    .classifications
                    .contains(&AdoptionClassificationV1::Managed)
            })
            .count(),
        declared_only_roles: role_findings
            .iter()
            .filter(|finding| {
                finding
                    .classifications
                    .contains(&AdoptionClassificationV1::DeclaredOnly)
            })
            .count(),
        attached_unobserved_roles: role_findings
            .iter()
            .filter(|finding| {
                finding
                    .classifications
                    .contains(&AdoptionClassificationV1::AttachedUnobserved)
            })
            .count(),
        observed_only_canisters: observed_findings
            .iter()
            .filter(|finding| {
                finding
                    .classifications
                    .contains(&AdoptionClassificationV1::ObservedOnly)
            })
            .count(),
        user_controlled_canisters: observed_findings
            .iter()
            .filter(|finding| {
                finding
                    .classifications
                    .contains(&AdoptionClassificationV1::UserControlled)
            })
            .count(),
        external_controller_required: role_findings
            .iter()
            .filter(|finding| {
                finding
                    .classifications
                    .contains(&AdoptionClassificationV1::ExternalControllerRequired)
            })
            .count(),
        evidence_conflicts: role_findings
            .iter()
            .filter(|finding| {
                finding
                    .classifications
                    .contains(&AdoptionClassificationV1::EvidenceConflict)
            })
            .count(),
        mutating_actions_performed: 0,
    }
}
