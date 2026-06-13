use super::super::{RoleArtifactSourceKindV1, RoleArtifactSourceV1};
use super::error::{
    ArtifactPromotionExecutionReceiptError, ArtifactPromotionPlanError,
    ArtifactPromotionProvenanceReportError, PromotionArtifactIdentityReportError,
    PromotionArtifactSourceError, PromotionMaterializationIdentityError,
    PromotionMaterializationIdentityReportError, PromotionPlanTransformError,
    PromotionPlanTransformEvidenceError, PromotionPolicyCheckError, PromotionReadinessError,
    PromotionTargetExecutionLineageError, PromotionWasmStoreCatalogVerificationError,
    PromotionWasmStoreIdentityReportError,
};

pub(super) fn ensure_locator_requirement(
    source: &RoleArtifactSourceV1,
) -> Result<(), PromotionArtifactSourceError> {
    match source.kind {
        RoleArtifactSourceKindV1::CanonicalWasmStoreDefault => Ok(()),
        _ => ensure_option_field("locator", source.locator.as_deref()),
    }
}

pub(super) const fn ensure_previous_receipt_requirement(
    source: &RoleArtifactSourceV1,
) -> Result<(), PromotionArtifactSourceError> {
    match (source.kind, source.previous_receipt_kind) {
        (RoleArtifactSourceKindV1::PreviousReceiptArtifact, Some(_)) => Ok(()),
        (RoleArtifactSourceKindV1::PreviousReceiptArtifact, None) => {
            Err(PromotionArtifactSourceError::MissingPreviousReceiptKind)
        }
        (_, Some(_)) => {
            Err(PromotionArtifactSourceError::UnexpectedPreviousReceiptKind { kind: source.kind })
        }
        (_, None) => Ok(()),
    }
}

pub(super) const fn ensure_previous_receipt_lineage_digest_requirement(
    source: &RoleArtifactSourceV1,
) -> Result<(), PromotionArtifactSourceError> {
    match (source.kind, source.previous_receipt_lineage_digest.as_ref()) {
        (RoleArtifactSourceKindV1::PreviousReceiptArtifact, Some(_)) => Ok(()),
        (RoleArtifactSourceKindV1::PreviousReceiptArtifact, None) => {
            Err(PromotionArtifactSourceError::MissingPreviousReceiptLineageDigest)
        }
        (_, Some(_)) => Err(
            PromotionArtifactSourceError::UnexpectedPreviousReceiptLineageDigest {
                kind: source.kind,
            },
        ),
        (_, None) => Ok(()),
    }
}

pub(super) const fn ensure_digest_requirement(
    source: &RoleArtifactSourceV1,
) -> Result<(), PromotionArtifactSourceError> {
    let has_digest =
        source.expected_wasm_sha256.is_some() || source.expected_wasm_gz_sha256.is_some();
    match source.kind {
        RoleArtifactSourceKindV1::LocalWasm if source.expected_wasm_sha256.is_none() => {
            Err(PromotionArtifactSourceError::MissingDigestPin { kind: source.kind })
        }
        RoleArtifactSourceKindV1::LocalWasmGz if source.expected_wasm_gz_sha256.is_none() => {
            Err(PromotionArtifactSourceError::MissingDigestPin { kind: source.kind })
        }
        RoleArtifactSourceKindV1::PublishedPackage
        | RoleArtifactSourceKindV1::PreviousReceiptArtifact
            if !has_digest =>
        {
            Err(PromotionArtifactSourceError::MissingDigestPin { kind: source.kind })
        }
        _ => Ok(()),
    }
}

fn ensure_option_field(
    field: &'static str,
    value: Option<&str>,
) -> Result<(), PromotionArtifactSourceError> {
    match value {
        Some(value) => ensure_field(field, value),
        None => Err(PromotionArtifactSourceError::MissingRequiredField { field }),
    }
}

pub(super) fn ensure_field(
    field: &'static str,
    value: &str,
) -> Result<(), PromotionArtifactSourceError> {
    if value.trim().is_empty() {
        return Err(PromotionArtifactSourceError::MissingRequiredField { field });
    }
    Ok(())
}

pub(super) fn ensure_optional_sha256(
    field: &'static str,
    value: Option<&str>,
) -> Result<(), PromotionArtifactSourceError> {
    let Some(value) = value else {
        return Ok(());
    };
    if is_lower_hex_sha256(value) {
        Ok(())
    } else {
        Err(PromotionArtifactSourceError::InvalidSha256Digest { field })
    }
}

fn is_lower_hex_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

pub(super) fn ensure_policy_field(
    field: &'static str,
    value: &str,
) -> Result<(), PromotionPolicyCheckError> {
    if value.trim().is_empty() {
        return Err(PromotionPolicyCheckError::MissingRequiredField { field });
    }
    Ok(())
}

pub(super) fn ensure_policy_sha256(
    field: &'static str,
    value: &str,
) -> Result<(), PromotionPolicyCheckError> {
    if is_lower_hex_sha256(value) {
        Ok(())
    } else {
        Err(PromotionPolicyCheckError::InvalidSha256Digest { field })
    }
}

pub(super) fn ensure_identity_report_field(
    field: &'static str,
    value: &str,
) -> Result<(), PromotionArtifactIdentityReportError> {
    if value.trim().is_empty() {
        return Err(PromotionArtifactIdentityReportError::MissingRequiredField { field });
    }
    Ok(())
}

pub(super) fn ensure_identity_optional_sha256(
    field: &'static str,
    value: Option<&str>,
) -> Result<(), PromotionArtifactIdentityReportError> {
    let Some(value) = value else {
        return Ok(());
    };
    if is_lower_hex_sha256(value) {
        Ok(())
    } else {
        Err(PromotionArtifactIdentityReportError::InvalidSha256Digest { field })
    }
}

pub(super) fn ensure_identity_report_sha256(
    field: &'static str,
    value: &str,
) -> Result<(), PromotionArtifactIdentityReportError> {
    if is_lower_hex_sha256(value) {
        Ok(())
    } else {
        Err(PromotionArtifactIdentityReportError::InvalidSha256Digest { field })
    }
}

pub(super) fn ensure_wasm_store_identity_report_field(
    field: &'static str,
    value: &str,
) -> Result<(), PromotionWasmStoreIdentityReportError> {
    if value.trim().is_empty() {
        return Err(PromotionWasmStoreIdentityReportError::MissingRequiredField { field });
    }
    Ok(())
}

pub(super) fn ensure_wasm_store_identity_report_sha256(
    field: &'static str,
    value: &str,
) -> Result<(), PromotionWasmStoreIdentityReportError> {
    if is_lower_hex_sha256(value) {
        Ok(())
    } else {
        Err(PromotionWasmStoreIdentityReportError::InvalidSha256Digest { field })
    }
}

pub(super) fn ensure_wasm_store_catalog_verification_field(
    field: &'static str,
    value: &str,
) -> Result<(), PromotionWasmStoreCatalogVerificationError> {
    if value.trim().is_empty() {
        return Err(PromotionWasmStoreCatalogVerificationError::MissingRequiredField { field });
    }
    Ok(())
}

pub(super) fn ensure_wasm_store_catalog_verification_sha256(
    field: &'static str,
    value: &str,
) -> Result<(), PromotionWasmStoreCatalogVerificationError> {
    if is_lower_hex_sha256(value) {
        Ok(())
    } else {
        Err(PromotionWasmStoreCatalogVerificationError::InvalidSha256Digest { field })
    }
}

pub(super) fn ensure_materialization_report_field(
    field: &'static str,
    value: &str,
) -> Result<(), PromotionMaterializationIdentityReportError> {
    if value.trim().is_empty() {
        return Err(PromotionMaterializationIdentityReportError::MissingRequiredField { field });
    }
    Ok(())
}

pub(super) fn ensure_provenance_report_field(
    field: &'static str,
    value: &str,
) -> Result<(), ArtifactPromotionProvenanceReportError> {
    if value.trim().is_empty() {
        return Err(ArtifactPromotionProvenanceReportError::MissingRequiredField { field });
    }
    Ok(())
}

pub(super) fn ensure_provenance_report_sha256(
    field: &'static str,
    value: &str,
) -> Result<(), ArtifactPromotionProvenanceReportError> {
    if is_lower_hex_sha256(value) {
        Ok(())
    } else {
        Err(ArtifactPromotionProvenanceReportError::InvalidSha256Digest { field })
    }
}

pub(super) fn ensure_execution_receipt_field(
    field: &'static str,
    value: &str,
) -> Result<(), ArtifactPromotionExecutionReceiptError> {
    if value.trim().is_empty() {
        return Err(ArtifactPromotionExecutionReceiptError::MissingRequiredField { field });
    }
    Ok(())
}

pub(super) fn ensure_execution_receipt_sha256(
    field: &'static str,
    value: &str,
) -> Result<(), ArtifactPromotionExecutionReceiptError> {
    if is_lower_hex_sha256(value) {
        Ok(())
    } else {
        Err(ArtifactPromotionExecutionReceiptError::LinkageMismatch { field })
    }
}

pub(super) fn ensure_materialization_report_sha256(
    field: &'static str,
    value: &str,
) -> Result<(), PromotionMaterializationIdentityReportError> {
    ensure_materialization_report_field(field, value)?;
    if is_lower_hex_sha256(value) {
        Ok(())
    } else {
        Err(
            PromotionMaterializationIdentityReportError::Materialization(
                PromotionMaterializationIdentityError::InvalidSha256Digest { field },
            ),
        )
    }
}

pub(super) fn ensure_materialization_field(
    field: &'static str,
    value: &str,
) -> Result<(), PromotionMaterializationIdentityError> {
    if value.trim().is_empty() {
        return Err(PromotionMaterializationIdentityError::MissingRequiredField { field });
    }
    Ok(())
}

pub(super) fn ensure_materialization_sha256(
    field: &'static str,
    value: &str,
) -> Result<(), PromotionMaterializationIdentityError> {
    ensure_materialization_field(field, value)?;
    if is_lower_hex_sha256(value) {
        Ok(())
    } else {
        Err(PromotionMaterializationIdentityError::InvalidSha256Digest { field })
    }
}

pub(super) const fn ensure_materialization_link(
    field: &'static str,
    valid: bool,
) -> Result<(), PromotionMaterializationIdentityError> {
    if valid {
        Ok(())
    } else {
        Err(PromotionMaterializationIdentityError::LinkageMismatch { field })
    }
}

pub(super) fn ensure_readiness_field(
    field: &'static str,
    value: &str,
) -> Result<(), PromotionReadinessError> {
    if value.trim().is_empty() {
        return Err(PromotionReadinessError::MissingRequiredField { field });
    }
    Ok(())
}

pub(super) fn ensure_readiness_sha256(
    field: &'static str,
    value: &str,
) -> Result<(), PromotionReadinessError> {
    if is_lower_hex_sha256(value) {
        Ok(())
    } else {
        Err(PromotionReadinessError::InvalidSha256Digest { field })
    }
}

pub(super) fn ensure_readiness_optional_sha256(
    field: &'static str,
    value: Option<&str>,
) -> Result<(), PromotionReadinessError> {
    let Some(value) = value else {
        return Ok(());
    };
    if is_lower_hex_sha256(value) {
        Ok(())
    } else {
        Err(PromotionReadinessError::InvalidSha256Digest { field })
    }
}

pub(super) fn ensure_transform_field(
    field: &'static str,
    value: &str,
) -> Result<(), PromotionPlanTransformError> {
    if value.trim().is_empty() {
        return Err(PromotionPlanTransformError::MissingRequiredField { field });
    }
    Ok(())
}

pub(super) fn ensure_evidence_field(
    field: &'static str,
    value: &str,
) -> Result<(), PromotionPlanTransformEvidenceError> {
    if value.trim().is_empty() {
        return Err(PromotionPlanTransformEvidenceError::MissingRequiredField { field });
    }
    Ok(())
}

pub(super) fn ensure_evidence_sha256(
    field: &'static str,
    value: &str,
) -> Result<(), PromotionPlanTransformEvidenceError> {
    if is_lower_hex_sha256(value) {
        Ok(())
    } else {
        Err(PromotionPlanTransformEvidenceError::InvalidSha256Digest { field })
    }
}

pub(super) fn ensure_artifact_promotion_plan_field(
    field: &'static str,
    value: &str,
) -> Result<(), ArtifactPromotionPlanError> {
    if value.trim().is_empty() {
        return Err(ArtifactPromotionPlanError::MissingRequiredField { field });
    }
    Ok(())
}

pub(super) fn ensure_artifact_promotion_plan_sha256(
    field: &'static str,
    value: &str,
) -> Result<(), ArtifactPromotionPlanError> {
    if is_lower_hex_sha256(value) {
        Ok(())
    } else {
        Err(ArtifactPromotionPlanError::InvalidSha256Digest { field })
    }
}

pub(super) fn ensure_target_execution_lineage_field(
    field: &'static str,
    value: &str,
) -> Result<(), PromotionTargetExecutionLineageError> {
    if value.trim().is_empty() {
        return Err(PromotionTargetExecutionLineageError::MissingRequiredField { field });
    }
    Ok(())
}

pub(super) fn ensure_target_execution_lineage_sha256(
    field: &'static str,
    value: &str,
) -> Result<(), PromotionTargetExecutionLineageError> {
    if is_lower_hex_sha256(value) {
        Ok(())
    } else {
        Err(PromotionTargetExecutionLineageError::InvalidSha256Digest { field })
    }
}
