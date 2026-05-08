use crate::{
    cdk::types::Principal,
    dto::auth::SignedRoleAttestation,
    format::display_optional,
    log,
    log::Topic,
    ops::{
        auth::{AuthExpiryError, AuthOpsError, AuthValidationError},
        runtime::metrics::auth::{
            record_attestation_epoch_rejected, record_attestation_unknown_key_id,
            record_attestation_verify_failed,
        },
    },
};
use std::future::Future;

#[derive(Debug)]
pub(super) enum RoleAttestationVerifyFlowError {
    Initial(AuthOpsError),
    Refresh {
        trigger: AuthOpsError,
        source: crate::InternalError,
    },
    PostRefresh(AuthOpsError),
}

pub(super) async fn verify_role_attestation_with_single_refresh<Verify, Refresh, RefreshFuture>(
    mut verify: Verify,
    mut refresh: Refresh,
) -> Result<(), RoleAttestationVerifyFlowError>
where
    Verify: FnMut() -> Result<(), AuthOpsError>,
    Refresh: FnMut() -> RefreshFuture,
    RefreshFuture: Future<Output = Result<(), crate::InternalError>>,
{
    match verify() {
        Ok(()) => Ok(()),
        Err(
            err @ AuthOpsError::Validation(AuthValidationError::AttestationUnknownKeyId { .. }),
        ) => {
            refresh()
                .await
                .map_err(|source| RoleAttestationVerifyFlowError::Refresh {
                    trigger: err,
                    source,
                })?;
            verify().map_err(RoleAttestationVerifyFlowError::PostRefresh)
        }
        Err(err) => Err(RoleAttestationVerifyFlowError::Initial(err)),
    }
}

pub(super) fn resolve_min_accepted_epoch(explicit: u64, configured: Option<u64>) -> u64 {
    if explicit > 0 {
        explicit
    } else {
        configured.unwrap_or(0)
    }
}

pub(super) fn record_attestation_verifier_rejection(err: &AuthOpsError) {
    record_attestation_verify_failed();
    match err {
        AuthOpsError::Validation(AuthValidationError::AttestationUnknownKeyId { .. }) => {
            record_attestation_unknown_key_id();
        }
        AuthOpsError::Expiry(AuthExpiryError::AttestationEpochRejected { .. }) => {
            record_attestation_epoch_rejected();
        }
        _ => {}
    }
}

pub(super) fn log_attestation_verifier_rejection(
    err: &AuthOpsError,
    attestation: &SignedRoleAttestation,
    caller: Principal,
    self_pid: Principal,
    phase: &str,
) {
    log!(
        Topic::Auth,
        Warn,
        "role attestation rejected phase={} local={} caller={} subject={} role={} key_id={} audience={} subnet={} issued_at={} expires_at={} epoch={} error={}",
        phase,
        self_pid,
        caller,
        attestation.payload.subject,
        attestation.payload.role,
        attestation.key_id,
        attestation.payload.audience,
        display_optional(attestation.payload.subnet_id),
        attestation.payload.issued_at,
        attestation.payload.expires_at,
        attestation.payload.epoch,
        err
    );
}
