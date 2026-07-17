//! Module: canic_cli::auth::codec
//!
//! Responsibility: decode delegated-auth responses and build small Candid arguments.
//! Does not own: command execution, transport, or operator rendering.

use super::{
    AuthCommandError, AuthIssuerObservedStatus, AuthRenewalBatchStatus, AuthRenewalStateStatus,
    AuthRenewalStatusSummary, AuthRenewalTemplateStatus,
};
use candid::Principal;
use canic_core::{
    cdk::utils::hash::hex_bytes as encode_hex,
    dto::auth::{
        ActiveDelegationProofStatus, ActiveDelegationProofStatusResponse,
        RootIssuerRenewalBatchStatus, RootIssuerRenewalStatusResponse,
    },
};
use canic_host::icp::{IcpJsonResponseError, decode_json_result_response};

pub(super) fn parse_issuer_principal(issuer: &str) -> Result<String, AuthCommandError> {
    Principal::from_text(issuer)
        .map(|principal| principal.to_text())
        .map_err(|_| AuthCommandError::InvalidIssuerPrincipal {
            issuer: issuer.to_string(),
        })
}

pub(super) fn parse_renewal_status_summary(
    output: &str,
) -> Result<AuthRenewalStatusSummary, IcpJsonResponseError> {
    let response = decode_json_result_response::<RootIssuerRenewalStatusResponse>(output)?;

    let template = response.template;
    let state = response.state;
    let latest_batch = response.latest_batch;

    Ok(AuthRenewalStatusSummary {
        template: AuthRenewalTemplateStatus {
            present: template.is_some(),
            enabled: template.as_ref().map(|template| template.enabled),
            cert_ttl_ns: template
                .as_ref()
                .map(|template| template.cert_ttl_ns.to_string()),
        },
        state: AuthRenewalStateStatus {
            present: state.is_some(),
            last_installed_cert_hash: state
                .as_ref()
                .and_then(|state| state.last_installed_cert_hash)
                .map(encode_hex),
            last_installed_expires_at_ns: state
                .as_ref()
                .and_then(|state| state.last_installed_expires_at_ns)
                .map(|value| value.to_string()),
            last_installed_refresh_after_ns: state
                .as_ref()
                .and_then(|state| state.last_installed_refresh_after_ns)
                .map(|value| value.to_string()),
            next_attempt_after_ns: state
                .as_ref()
                .map(|state| state.next_attempt_after_ns.to_string()),
        },
        latest_batch: AuthRenewalBatchStatus {
            present: latest_batch.is_some(),
            status: latest_batch
                .as_ref()
                .map(|batch| renewal_batch_status_label(&batch.status).to_string()),
            batch_id: latest_batch
                .as_ref()
                .map(|batch| encode_hex(batch.batch_id)),
            cert_hash: latest_batch
                .as_ref()
                .map(|batch| encode_hex(batch.cert_hash)),
            proof_epoch: latest_batch.as_ref().map(|batch| batch.proof_epoch),
            expires_at_ns: latest_batch
                .as_ref()
                .map(|batch| batch.expires_at_ns.to_string()),
            installed_at_ns: latest_batch
                .as_ref()
                .and_then(|batch| batch.installed_at_ns)
                .map(|value| value.to_string()),
            retry_after_ns: latest_batch
                .as_ref()
                .and_then(|batch| batch.retry_after_ns)
                .map(|value| value.to_string()),
            failure: latest_batch.and_then(|batch| batch.failure),
        },
    })
}

pub(super) fn parse_issuer_observed_status(
    output: &str,
) -> Result<AuthIssuerObservedStatus, IcpJsonResponseError> {
    let response = decode_json_result_response::<ActiveDelegationProofStatusResponse>(output)?;

    Ok(AuthIssuerObservedStatus {
        status: active_proof_status_label(&response.status).to_string(),
        cert_hash: response.cert_hash.map(encode_hex),
        expires_at_ns: response.expires_at_ns.map(|value| value.to_string()),
        refresh_after_ns: response.refresh_after_ns.map(|value| value.to_string()),
    })
}

const fn active_proof_status_label(status: &ActiveDelegationProofStatus) -> &'static str {
    match status {
        ActiveDelegationProofStatus::Expired => "expired",
        ActiveDelegationProofStatus::Missing => "missing",
        ActiveDelegationProofStatus::RefreshNeeded => "refresh_needed",
        ActiveDelegationProofStatus::Valid => "valid",
    }
}

const fn renewal_batch_status_label(status: &RootIssuerRenewalBatchStatus) -> &'static str {
    match status {
        RootIssuerRenewalBatchStatus::FailedRetryable => "failed_retryable",
        RootIssuerRenewalBatchStatus::Installed => "installed",
        RootIssuerRenewalBatchStatus::Installing => "installing",
        RootIssuerRenewalBatchStatus::Prepared => "prepared",
        RootIssuerRenewalBatchStatus::Signed => "signed",
        RootIssuerRenewalBatchStatus::Signing => "signing",
    }
}

pub(super) fn root_issuer_renewal_status_arg(issuer_pid: &str) -> String {
    format!(r#"(record {{ issuer_pid = principal "{issuer_pid}" }})"#)
}
