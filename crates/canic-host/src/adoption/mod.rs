//! Passive adoption profile and onboarding reports.

mod evidence;
mod findings;
mod model;
mod recommendations;
mod report;
mod summary;

pub use model::{
    ADOPTION_REPORT_SCHEMA_VERSION, AdoptionArtifactStateV1, AdoptionAuthorityStateV1,
    AdoptionClassificationV1, AdoptionDeclarationStateV1, AdoptionMatchConfidenceV1,
    AdoptionObservationStateV1, AdoptionObservedCanisterFindingV1,
    AdoptionOperatorActionRequirementV1, AdoptionPackageMetadataV1, AdoptionPackageStateV1,
    AdoptionProfileV1, AdoptionRecommendationSeverityV1, AdoptionRecommendationV1,
    AdoptionReportError, AdoptionReportInputsV1, AdoptionReportRequest, AdoptionReportSummaryV1,
    AdoptionReportV1, AdoptionRoleFindingV1, AdoptionSuggestedActionEffectV1,
    AdoptionSuggestedActionSupportV1, AdoptionTopologyStateV1,
};
pub use report::adoption_report_from_config_source;

#[cfg(test)]
mod tests;
