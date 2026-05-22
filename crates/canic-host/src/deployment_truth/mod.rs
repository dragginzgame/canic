//! Passive deployment-truth model types for host-side planning and safety checks.

mod model;
mod observe;
mod plan;
mod report;
#[cfg(test)]
mod tests;

pub use model::{
    ArtifactDigestSourceV1, ArtifactSourceV1, AuthorityProfileV1, CanisterControlClassV1,
    DeploymentAssumptionV1, DeploymentCheckV1, DeploymentCommandResultV1, DeploymentDiffV1,
    DeploymentIdentityV1, DeploymentInventoryV1, DeploymentObservationGapV1, DeploymentPlanV1,
    DeploymentReceiptV1, DiffItemV1, ExpectedCanisterV1, ExpectedPoolCanisterV1,
    LocalDeploymentConfigV1, ObservationStatusV1, ObservedArtifactV1, ObservedCanisterV1,
    ObservedPoolCanisterV1, PhaseReceiptV1, ResumeSafetyV1, RoleArtifactManifestV1, RoleArtifactV1,
    RoleEpochExpectationV1, RoleEpochObservationV1, SafetyFindingV1, SafetyReportV1,
    SafetySeverityV1, SafetyStatusV1, TrustDomainV1, VerifiedPostconditionV1,
    VerifierReadinessExpectationV1, VerifierReadinessObservationV1,
};
pub use observe::{
    DeploymentTruthError, LocalArtifactManifestRequest, LocalInventoryRequest,
    collect_local_deployment_inventory, collect_local_role_artifact_manifest,
};
pub use plan::{LocalDeploymentPlanRequest, build_local_deployment_plan};
pub use report::{
    LocalDeploymentCheckRequest, check_local_deployment, compare_plan_to_inventory,
    safety_report_from_diff,
};

pub const DEPLOYMENT_TRUTH_SCHEMA_VERSION: u32 = 1;
