//! Module: canic_cli::auth::codec
//!
//! Responsibility: decode delegated-auth responses and build small Candid arguments.
//! Does not own: command execution, transport, or operator rendering.

use super::{
    AuthCommandError, AuthIssuerObservedStatus, AuthRenewalBatchStatus, AuthRenewalStateStatus,
    AuthRenewalStatusSummary, AuthRenewalTemplateStatus,
};
use candid::{CandidType, Principal};
use canic_core::{
    cdk::utils::hash::hex_bytes as encode_hex,
    dto::{
        auth::{
            ActiveDelegationProofStatus, ActiveDelegationProofStatusResponse,
            RootIssuerRenewalBatchStatus, RootIssuerRenewalStatusResponse,
        },
        error::{Error as CanicError, ErrorCode},
    },
};
use canic_host::icp::{IcpJsonResponseError, decode_json_result_response};
use serde::de::DeserializeOwned;
use std::{error::Error, fmt};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum AuthResponseKind {
    IssuerStatus,
    RenewalStatus,
}

impl AuthResponseKind {
    const fn label(self) -> &'static str {
        match self {
            Self::IssuerStatus => "issuer status",
            Self::RenewalStatus => "renewal status",
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub(super) enum AuthResponseParseError {
    InvalidCandid {
        kind: AuthResponseKind,
        error: String,
    },
    InvalidJson {
        kind: AuthResponseKind,
        error: String,
    },
    InvalidPayload(AuthResponseKind),
    InvalidResponseBytes {
        kind: AuthResponseKind,
        error: String,
    },
    RemoteError {
        kind: AuthResponseKind,
        code: ErrorCode,
        message: String,
    },
}

impl fmt::Display for AuthResponseParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCandid { kind, error } => {
                write!(
                    formatter,
                    "{} response bytes contain invalid Candid: {error}",
                    kind.label()
                )
            }
            Self::InvalidJson { kind, error } => {
                write!(
                    formatter,
                    "{} response has invalid JSON: {error}",
                    kind.label()
                )
            }
            Self::InvalidPayload(kind) => {
                write!(
                    formatter,
                    "{} response is missing string `response_bytes`",
                    kind.label()
                )
            }
            Self::InvalidResponseBytes { kind, error } => {
                write!(
                    formatter,
                    "{} response_bytes is invalid hexadecimal: {error}",
                    kind.label()
                )
            }
            Self::RemoteError {
                kind,
                code,
                message,
            } => {
                write!(
                    formatter,
                    "{} response returned [{code:?}] {message}",
                    kind.label()
                )
            }
        }
    }
}

impl Error for AuthResponseParseError {}

pub(super) fn parse_issuer_principal(issuer: &str) -> Result<String, AuthCommandError> {
    Principal::from_text(issuer)
        .map(|principal| principal.to_text())
        .map_err(|_| AuthCommandError::InvalidIssuerPrincipal {
            issuer: issuer.to_string(),
        })
}

pub(super) fn parse_renewal_status_summary(
    output: &str,
) -> Result<AuthRenewalStatusSummary, AuthResponseParseError> {
    let kind = AuthResponseKind::RenewalStatus;
    let response = typed_response::<RootIssuerRenewalStatusResponse>(output, kind)?;

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
) -> Result<AuthIssuerObservedStatus, AuthResponseParseError> {
    let kind = AuthResponseKind::IssuerStatus;
    let response = typed_response::<ActiveDelegationProofStatusResponse>(output, kind)?;

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

fn remote_error(kind: AuthResponseKind, error: CanicError) -> AuthResponseParseError {
    AuthResponseParseError::RemoteError {
        kind,
        code: error.code,
        message: error.message,
    }
}

fn typed_response<T>(output: &str, kind: AuthResponseKind) -> Result<T, AuthResponseParseError>
where
    T: CandidType + DeserializeOwned,
{
    decode_json_result_response(output).map_err(|error| match error {
        IcpJsonResponseError::Candid(error) => AuthResponseParseError::InvalidCandid {
            kind,
            error: error.to_string(),
        },
        IcpJsonResponseError::Hex(error) => AuthResponseParseError::InvalidResponseBytes {
            kind,
            error: error.to_string(),
        },
        IcpJsonResponseError::Json(error) => AuthResponseParseError::InvalidJson {
            kind,
            error: error.to_string(),
        },
        IcpJsonResponseError::MissingResponseBytes => AuthResponseParseError::InvalidPayload(kind),
        IcpJsonResponseError::Rejected(error) => remote_error(kind, error),
    })
}

pub(super) fn root_issuer_renewal_status_arg(issuer_pid: &str) -> String {
    format!(r#"(record {{ issuer_pid = principal "{issuer_pid}" }})"#)
}
