use crate::{
    cdk::types::Principal,
    dto::auth::SignedRoleAttestation,
    log,
    log::Topic,
    ops::{
        auth::DelegatedTokenOpsError,
        runtime::metrics::auth::{
            record_attestation_epoch_rejected, record_attestation_unknown_key_id,
            record_attestation_verify_failed,
        },
    },
};
use std::future::Future;

#[derive(Debug)]
pub(super) enum RoleAttestationVerifyFlowError {
    Initial(DelegatedTokenOpsError),
    Refresh {
        trigger: DelegatedTokenOpsError,
        source: crate::InternalError,
    },
    PostRefresh(DelegatedTokenOpsError),
}

pub(super) async fn verify_role_attestation_with_single_refresh<Verify, Refresh, RefreshFuture>(
    mut verify: Verify,
    mut refresh: Refresh,
) -> Result<(), RoleAttestationVerifyFlowError>
where
    Verify: FnMut() -> Result<(), DelegatedTokenOpsError>,
    Refresh: FnMut() -> RefreshFuture,
    RefreshFuture: Future<Output = Result<(), crate::InternalError>>,
{
    match verify() {
        Ok(()) => Ok(()),
        Err(err @ DelegatedTokenOpsError::AttestationUnknownKeyId { .. }) => {
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

pub(super) fn record_attestation_verifier_rejection(err: &DelegatedTokenOpsError) {
    record_attestation_verify_failed();
    match err {
        DelegatedTokenOpsError::AttestationUnknownKeyId { .. } => {
            record_attestation_unknown_key_id();
        }
        DelegatedTokenOpsError::AttestationEpochRejected { .. } => {
            record_attestation_epoch_rejected();
        }
        _ => {}
    }
}

pub(super) fn log_attestation_verifier_rejection(
    err: &DelegatedTokenOpsError,
    attestation: &SignedRoleAttestation,
    caller: Principal,
    self_pid: Principal,
    phase: &str,
) {
    log!(
        Topic::Auth,
        Warn,
        "role attestation rejected phase={} local={} caller={} subject={} role={} key_id={} audience={:?} subnet={:?} issued_at={} expires_at={} epoch={} error={}",
        phase,
        self_pid,
        caller,
        attestation.payload.subject,
        attestation.payload.role,
        attestation.key_id,
        attestation.payload.audience,
        attestation.payload.subnet_id,
        attestation.payload.issued_at,
        attestation.payload.expires_at,
        attestation.payload.epoch,
        err
    );
}
