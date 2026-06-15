use crate::deployment_truth::RoleArtifactSourceKindV1;
use thiserror::Error as ThisError;

///
/// PromotionArtifactSourceError
///
#[derive(Debug, ThisError)]
pub enum PromotionArtifactSourceError {
    #[error("promotion artifact source is missing required field: {field}")]
    MissingRequiredField { field: &'static str },
    #[error("promotion artifact source field {field} must be a lowercase sha256 hex digest")]
    InvalidSha256Digest { field: &'static str },
    #[error("promotion artifact source kind {kind:?} requires a digest pin")]
    MissingDigestPin { kind: RoleArtifactSourceKindV1 },
    #[error("promotion artifact source kind {kind:?} cannot carry previous receipt kind")]
    UnexpectedPreviousReceiptKind { kind: RoleArtifactSourceKindV1 },
    #[error(
        "promotion artifact source kind PreviousReceiptArtifact requires an eligible receipt kind"
    )]
    MissingPreviousReceiptKind,
    #[error(
        "promotion artifact source kind PreviousReceiptArtifact requires a source receipt lineage digest"
    )]
    MissingPreviousReceiptLineageDigest,
    #[error("promotion artifact source kind {kind:?} cannot carry source receipt lineage digest")]
    UnexpectedPreviousReceiptLineageDigest { kind: RoleArtifactSourceKindV1 },
}
