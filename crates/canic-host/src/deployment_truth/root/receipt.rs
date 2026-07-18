use super::super::*;
use super::{
    digest::deployment_root_verification_receipt_digest,
    error::DeploymentRootVerificationReceiptError,
};

/// Validate archived root-verification receipt consistency and digest
/// stability.
pub fn validate_deployment_root_verification_receipt(
    receipt: &DeploymentRootVerificationReceiptV1,
) -> Result<(), DeploymentRootVerificationReceiptError> {
    if receipt.schema_version != DEPLOYMENT_TRUTH_SCHEMA_VERSION {
        return Err(
            DeploymentRootVerificationReceiptError::SchemaVersionMismatch {
                expected: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
                actual: receipt.schema_version,
            },
        );
    }
    ensure_root_verification_receipt_field("receipt_id", receipt.receipt_id.as_str())?;
    ensure_root_verification_receipt_sha256("receipt_digest", receipt.receipt_digest.as_str())?;
    ensure_root_verification_receipt_field("deployment_name", receipt.deployment_name.as_str())?;
    ensure_root_verification_receipt_field("environment", receipt.environment.as_str())?;
    ensure_root_verification_receipt_field("fleet_template", receipt.fleet_template.as_str())?;
    ensure_root_verification_receipt_field("root_principal", receipt.root_principal.as_str())?;
    ensure_root_verification_receipt_field("source_report_id", receipt.source_report_id.as_str())?;
    ensure_root_verification_receipt_sha256(
        "source_report_digest",
        receipt.source_report_digest.as_str(),
    )?;
    ensure_root_verification_receipt_field(
        "source_report_requested_at",
        receipt.source_report_requested_at.as_str(),
    )?;
    ensure_root_verification_receipt_timestamp(
        "source_report_requested_at",
        receipt.source_report_requested_at.as_str(),
    )?;
    ensure_root_verification_receipt_field(
        "source_observed_root_canister_id",
        receipt.source_observed_root_canister_id.as_str(),
    )?;
    if receipt.source_report_evidence_status
        != DeploymentRootVerificationEvidenceStatusV1::EvidenceSatisfied
        || receipt.source_report_source != DeploymentRootVerificationSourceV1::DeploymentTruthCheck
        || receipt.source_report_current_root_verification != receipt.previous_root_verification
        || receipt.source_root_observation_source
            != DeploymentRootObservationSourceV1::IcpCanisterStatus
        || receipt.source_observed_root_canister_id != receipt.root_principal
        || receipt.source_report_state_transition != source_report_transition_for_receipt(receipt)
        || !source_report_timestamp_matches_receipt(receipt)
    {
        return Err(DeploymentRootVerificationReceiptError::SourceEvidenceMismatch);
    }
    ensure_root_verification_receipt_field("source_check_id", receipt.source_check_id.as_str())?;
    ensure_root_verification_receipt_sha256(
        "source_check_digest",
        receipt.source_check_digest.as_str(),
    )?;
    ensure_root_verification_receipt_field(
        "source_deployment_plan_id",
        receipt.source_deployment_plan_id.as_str(),
    )?;
    ensure_root_verification_receipt_sha256(
        "source_deployment_plan_digest",
        receipt.source_deployment_plan_digest.as_str(),
    )?;
    ensure_root_verification_receipt_field(
        "source_inventory_id",
        receipt.source_inventory_id.as_str(),
    )?;
    ensure_root_verification_receipt_sha256(
        "source_inventory_digest",
        receipt.source_inventory_digest.as_str(),
    )?;
    ensure_root_verification_receipt_field("local_state_path", receipt.local_state_path.as_str())?;
    ensure_root_verification_receipt_sha256(
        "local_state_digest_before",
        receipt.local_state_digest_before.as_str(),
    )?;
    ensure_root_verification_receipt_sha256(
        "local_state_digest_after",
        receipt.local_state_digest_after.as_str(),
    )?;

    if receipt.new_root_verification != DeploymentRootVerificationStateV1::Verified
        || receipt.state_transition != receipt_state_transition(receipt)
    {
        return Err(DeploymentRootVerificationReceiptError::StateTransitionMismatch);
    }
    if !receipt_local_state_digest_transition_is_valid(receipt) {
        return Err(DeploymentRootVerificationReceiptError::LocalStateDigestMismatch);
    }
    if receipt.receipt_digest != deployment_root_verification_receipt_digest(receipt) {
        return Err(DeploymentRootVerificationReceiptError::DigestMismatch {
            field: "receipt_digest",
        });
    }
    Ok(())
}
const fn receipt_state_transition(
    receipt: &DeploymentRootVerificationReceiptV1,
) -> DeploymentRootVerificationStateTransitionV1 {
    match receipt.previous_root_verification {
        DeploymentRootVerificationStateV1::NotVerified => {
            DeploymentRootVerificationStateTransitionV1::PromotedNotVerifiedToVerified
        }
        DeploymentRootVerificationStateV1::Verified => {
            DeploymentRootVerificationStateTransitionV1::NoStateChange
        }
    }
}

const fn source_report_transition_for_receipt(
    receipt: &DeploymentRootVerificationReceiptV1,
) -> DeploymentRootVerificationStateTransitionV1 {
    match receipt.previous_root_verification {
        DeploymentRootVerificationStateV1::NotVerified => {
            DeploymentRootVerificationStateTransitionV1::WouldPromoteNotVerifiedToVerified
        }
        DeploymentRootVerificationStateV1::Verified => {
            DeploymentRootVerificationStateTransitionV1::NoStateChange
        }
    }
}

fn receipt_local_state_digest_transition_is_valid(
    receipt: &DeploymentRootVerificationReceiptV1,
) -> bool {
    match receipt.state_transition {
        DeploymentRootVerificationStateTransitionV1::PromotedNotVerifiedToVerified => {
            receipt.local_state_digest_before != receipt.local_state_digest_after
        }
        DeploymentRootVerificationStateTransitionV1::NoStateChange => {
            receipt.local_state_digest_before == receipt.local_state_digest_after
        }
        DeploymentRootVerificationStateTransitionV1::NotAttempted
        | DeploymentRootVerificationStateTransitionV1::Blocked
        | DeploymentRootVerificationStateTransitionV1::WouldPromoteNotVerifiedToVerified => false,
    }
}
const fn ensure_root_verification_receipt_field(
    field: &'static str,
    value: &str,
) -> Result<(), DeploymentRootVerificationReceiptError> {
    if value.is_empty() {
        Err(DeploymentRootVerificationReceiptError::MissingRequiredField { field })
    } else {
        Ok(())
    }
}

fn ensure_root_verification_receipt_sha256(
    field: &'static str,
    value: &str,
) -> Result<(), DeploymentRootVerificationReceiptError> {
    if value.is_empty() {
        return Err(DeploymentRootVerificationReceiptError::MissingRequiredField { field });
    }
    if is_lower_hex_sha256(value) {
        Ok(())
    } else {
        Err(DeploymentRootVerificationReceiptError::InvalidSha256Digest { field })
    }
}

fn ensure_root_verification_receipt_timestamp(
    field: &'static str,
    value: &str,
) -> Result<(), DeploymentRootVerificationReceiptError> {
    if value.is_empty() {
        return Err(DeploymentRootVerificationReceiptError::MissingRequiredField { field });
    }
    if is_supported_root_verification_timestamp_label(value) {
        Ok(())
    } else {
        Err(DeploymentRootVerificationReceiptError::InvalidTimestampLabel { field })
    }
}

fn is_supported_root_verification_timestamp_label(value: &str) -> bool {
    if let Some(unix_value) = value.strip_prefix("unix:") {
        return !unix_value.is_empty() && unix_value.bytes().all(|byte| byte.is_ascii_digit());
    }
    value.len() >= "1970-01-01T00:00:00Z".len() && value.contains('T') && value.ends_with('Z')
}

fn source_report_timestamp_matches_receipt(receipt: &DeploymentRootVerificationReceiptV1) -> bool {
    let Some(unix_value) = receipt.source_report_requested_at.strip_prefix("unix:") else {
        return true;
    };
    unix_value.parse::<u64>() == Ok(receipt.verified_at_unix_secs)
}

fn is_lower_hex_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}
