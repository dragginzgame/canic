//! Module: canic_cli::auth::codec
//!
//! Responsibility: project native auth observations into operator reports.
//! Does not own: command execution, transport, or operator rendering.

use crate::auth::{
    AuthCommandError, AuthIssuerObservedStatus, AuthRenewalBatchStatus, AuthRenewalStateStatus,
    AuthRenewalStatusSummary, AuthRenewalTemplateStatus,
};
use canic_contracts::dto::{
    auth::{ActiveDelegationProofStatus, RootIssuerRenewalBatchStatus},
    issuer_renewal::RootIssuerRenewalStatusResponse,
};
use canic_core::cdk::utils::hash::hex_bytes as encode_hex;
use ic_platform::principal::ExternalPrincipal as Principal;
use serde::{Serialize, Serializer};

pub(super) fn parse_issuer_principal(issuer: &str) -> Result<String, AuthCommandError> {
    Principal::from_text(issuer)
        .map(|principal| principal.to_text())
        .map_err(|_| AuthCommandError::InvalidIssuerPrincipal {
            issuer: issuer.to_string(),
        })
}

pub(super) fn renewal_status_summary(
    response: RootIssuerRenewalStatusResponse,
) -> AuthRenewalStatusSummary {
    let template = response.template;
    let state = response.state;
    let latest_batch = response.latest_batch;

    AuthRenewalStatusSummary {
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
            status: latest_batch.as_ref().map(|batch| batch.status.clone()),
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
    }
}

pub(super) fn issuer_observed_status(
    response: canic_contracts::dto::issuer_proof::ActiveDelegationProofStatusResponse,
) -> AuthIssuerObservedStatus {
    AuthIssuerObservedStatus {
        status: response.status,
        cert_hash: response.cert_hash.map(encode_hex),
        expires_at_ns: response.expires_at_ns.map(|value| value.to_string()),
        refresh_after_ns: response.refresh_after_ns.map(|value| value.to_string()),
    }
}

pub(super) const fn active_proof_status_label(
    status: &ActiveDelegationProofStatus,
) -> &'static str {
    match status {
        ActiveDelegationProofStatus::Expired => "expired",
        ActiveDelegationProofStatus::Missing => "missing",
        ActiveDelegationProofStatus::RefreshNeeded => "refresh_needed",
        ActiveDelegationProofStatus::Valid => "valid",
    }
}

pub(super) const fn renewal_batch_status_label(
    status: &RootIssuerRenewalBatchStatus,
) -> &'static str {
    match status {
        RootIssuerRenewalBatchStatus::FailedRetryable => "failed_retryable",
        RootIssuerRenewalBatchStatus::Installed => "installed",
        RootIssuerRenewalBatchStatus::Installing => "installing",
        RootIssuerRenewalBatchStatus::Prepared => "prepared",
        RootIssuerRenewalBatchStatus::Signed => "signed",
        RootIssuerRenewalBatchStatus::Signing => "signing",
    }
}

#[expect(
    clippy::ref_option,
    reason = "Serde serialize_with passes a borrowed field"
)]
pub(super) fn serialize_issuer_status<S: Serializer>(
    status: &Option<ActiveDelegationProofStatus>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    issuer_status_label(status.as_ref()).serialize(serializer)
}

pub(super) fn issuer_status_label(status: Option<&ActiveDelegationProofStatus>) -> &'static str {
    status.map_or("unavailable", active_proof_status_label)
}

#[expect(
    clippy::ref_option,
    reason = "Serde serialize_with passes a borrowed field"
)]
pub(super) fn serialize_batch_status<S: Serializer>(
    status: &Option<RootIssuerRenewalBatchStatus>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    status
        .as_ref()
        .map(renewal_batch_status_label)
        .serialize(serializer)
}
