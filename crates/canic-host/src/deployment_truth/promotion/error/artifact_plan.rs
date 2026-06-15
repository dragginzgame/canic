use super::identity::PromotionArtifactIdentityReportError;
use super::readiness::PromotionReadinessError;
use super::transform::{PromotionPlanTransformError, PromotionTargetExecutionLineageError};
use crate::deployment_truth::PromotionReadinessStatusV1;
use crate::deployment_truth::executor::DeploymentExecutionPreflightError;
use thiserror::Error as ThisError;

///
/// ArtifactPromotionPlanError
///
#[derive(Debug, ThisError)]
pub enum ArtifactPromotionPlanError {
    #[error("artifact promotion plan schema mismatch: expected {expected}, found {found}")]
    SchemaVersionMismatch { expected: u32, found: u32 },
    #[error("artifact promotion plan is missing required field: {field}")]
    MissingRequiredField { field: &'static str },
    #[error(
        "artifact promotion plan status {status:?} does not match blocker count {blocker_count}"
    )]
    StatusBlockerMismatch {
        status: PromotionReadinessStatusV1,
        blocker_count: usize,
    },
    #[error("artifact promotion plan field {field} is inconsistent")]
    LinkageMismatch { field: &'static str },
    #[error("artifact promotion plan field {field} must be a lowercase sha256 hex digest")]
    InvalidSha256Digest { field: &'static str },
    #[error("artifact promotion plan readiness is invalid: {0}")]
    Readiness(#[from] PromotionReadinessError),
    #[error("artifact promotion plan artifact identity report is invalid: {0}")]
    ArtifactIdentityReport(#[from] PromotionArtifactIdentityReportError),
    #[error("artifact promotion plan transform is invalid: {0}")]
    Transform(#[from] PromotionPlanTransformError),
    #[error("artifact promotion plan target execution lineage is invalid: {0}")]
    TargetExecutionLineage(#[from] PromotionTargetExecutionLineageError),
    #[error(
        "artifact promotion plan requires target execution lineage for deployment check validation"
    )]
    MissingTargetExecutionLineage,
    #[error("artifact promotion plan target deployment check is invalid: {0}")]
    TargetCheck(#[source] DeploymentExecutionPreflightError),
}
