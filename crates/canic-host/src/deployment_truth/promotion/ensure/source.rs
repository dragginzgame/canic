use crate::deployment_truth::{RoleArtifactSourceKindV1, RoleArtifactSourceV1};

use super::super::error::PromotionArtifactSourceError;
use super::is_lower_hex_sha256;

pub(in crate::deployment_truth::promotion) fn ensure_locator_requirement(
    source: &RoleArtifactSourceV1,
) -> Result<(), PromotionArtifactSourceError> {
    match source.kind {
        RoleArtifactSourceKindV1::CanonicalWasmStoreDefault => Ok(()),
        _ => ensure_option_field("locator", source.locator.as_deref()),
    }
}

pub(in crate::deployment_truth::promotion) const fn ensure_previous_receipt_requirement(
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

pub(in crate::deployment_truth::promotion) const fn ensure_previous_receipt_lineage_digest_requirement(
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

pub(in crate::deployment_truth::promotion) const fn ensure_digest_requirement(
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

pub(in crate::deployment_truth::promotion) fn ensure_field(
    field: &'static str,
    value: &str,
) -> Result<(), PromotionArtifactSourceError> {
    if value.trim().is_empty() {
        return Err(PromotionArtifactSourceError::MissingRequiredField { field });
    }
    Ok(())
}

pub(in crate::deployment_truth::promotion) fn ensure_optional_sha256(
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
