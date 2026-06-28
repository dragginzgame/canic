//! Module: ops::auth::delegation::root_issuer_renewal::retrieval
//!
//! Responsibility: retrieve, validate, and expire scheduled renewal proof batches.
//! Does not own: template administration, prepare scheduling, or install outcome mapping.

use super::install::record_scheduled_renewal_attempt_failure;
use crate::{
    InternalError,
    domain::policy::auth::{
        RootDelegationRenewalBatch, RootIssuerRenewalAttempt,
        RootIssuerRenewalAttemptStatus as PolicyRenewalAttemptStatus,
        RootIssuerRenewalOutcome as PolicyRenewalOutcome,
    },
    dto::auth::{
        RootDelegationProofBatchGetRequest, RootDelegationProofBatchGetResponse,
        RootDelegationProofBatchProofRef, RootDelegationRenewalProofBatchGetRequest,
    },
    log::Topic,
    ops::{
        ic::IcOps,
        runtime::metrics::delegated_auth::{DelegatedAuthMetricReason, DelegatedAuthMetrics},
        storage::auth::AuthStateOps,
    },
};

pub(in crate::ops::auth::delegation) fn ensure_delegation_renewal_batch_scheduled(
    batch_id: [u8; 32],
    now_ns: u64,
) -> Result<(), InternalError> {
    let batch = AuthStateOps::root_delegation_renewal_batch(batch_id).ok_or_else(|| {
        InternalError::invalid_input(
            "renewal provisioner may install only scheduled root delegation renewal batches",
        )
    })?;
    let attempts = scheduled_renewal_batch_install_attempts(&batch)?;
    if scheduled_renewal_batch_install_deadline_expired(&attempts, now_ns) {
        record_scheduled_renewal_batch_install_deadline_expired(attempts, now_ns);
        prune_expired_renewal_batches(now_ns);
        return Err(InternalError::invalid_input(
            "root delegation renewal batch install deadline expired",
        ));
    }

    Ok(())
}

pub(in crate::ops::auth::delegation) fn get_delegation_renewal_proof_batch(
    request: RootDelegationRenewalProofBatchGetRequest,
) -> Result<RootDelegationProofBatchGetResponse, InternalError> {
    get_delegation_renewal_proof_batch_with_getter(request, IcOps::now_nanos(), |request| {
        super::super::batch::get_delegation_proof_batch(request)
    })
}

pub(super) fn get_delegation_renewal_proof_batch_with_getter(
    request: RootDelegationRenewalProofBatchGetRequest,
    now_ns: u64,
    get_batch: impl FnOnce(
        RootDelegationProofBatchGetRequest,
    ) -> Result<RootDelegationProofBatchGetResponse, InternalError>,
) -> Result<RootDelegationProofBatchGetResponse, InternalError> {
    let Some(batch) = AuthStateOps::root_delegation_renewal_batch(request.batch_id) else {
        DelegatedAuthMetrics::record_renewal_proof_retrieve_failed(
            DelegatedAuthMetricReason::InvalidState,
        );
        return Err(InternalError::invalid_input(
            "root delegation renewal batch is not scheduled",
        ));
    };
    let proof_refs = match scheduled_renewal_batch_proof_refs(&batch, now_ns) {
        Ok(proof_refs) => proof_refs,
        Err(err) => {
            DelegatedAuthMetrics::record_renewal_proof_retrieve_failed(
                DelegatedAuthMetricReason::InvalidState,
            );
            return Err(err);
        }
    };
    let response = match get_batch(RootDelegationProofBatchGetRequest {
        batch_id: batch.batch_id,
        entries: proof_refs,
    }) {
        Ok(response) => response,
        Err(err) => {
            DelegatedAuthMetrics::record_renewal_proof_retrieve_failed(
                DelegatedAuthMetricReason::RootProofInvalid,
            );
            return Err(err);
        }
    };
    DelegatedAuthMetrics::record_renewal_proof_retrieve_completed();
    crate::log!(
        Topic::Auth,
        Info,
        "root delegated-proof renewal retrieved batch_id={:?} proofs={}",
        response.batch_id,
        response.proofs.len()
    );
    Ok(response)
}

fn scheduled_renewal_batch_proof_refs(
    batch: &RootDelegationRenewalBatch,
    now_ns: u64,
) -> Result<Vec<RootDelegationProofBatchProofRef>, InternalError> {
    let attempts = scheduled_renewal_batch_attempts(batch, now_ns)?;
    Ok(attempts
        .into_iter()
        .map(|attempt| RootDelegationProofBatchProofRef {
            issuer_pid: attempt.proof_ref.issuer_pid,
            cert_hash: attempt.proof_ref.cert_hash,
        })
        .collect())
}

pub(super) fn scheduled_renewal_batch_attempts(
    batch: &RootDelegationRenewalBatch,
    now_ns: u64,
) -> Result<Vec<RootIssuerRenewalAttempt>, InternalError> {
    if batch.attempt_ids.is_empty() {
        return Err(InternalError::invalid_input(
            "root delegation renewal batch has no scheduled attempts",
        ));
    }
    if now_ns >= batch.retrieval_expires_at_ns {
        return Err(InternalError::invalid_input(
            "root delegation renewal batch retrieval window expired",
        ));
    }

    let mut attempts = Vec::with_capacity(batch.attempt_ids.len());
    for attempt_id in &batch.attempt_ids {
        let attempt = AuthStateOps::root_issuer_renewal_attempt(*attempt_id).ok_or_else(|| {
            InternalError::invalid_input("root delegation renewal attempt is not scheduled")
        })?;
        if attempt.batch_id != batch.batch_id {
            return Err(InternalError::invariant(
                crate::InternalErrorOrigin::Ops,
                "root delegation renewal attempt batch mismatch",
            ));
        }
        if attempt.status != PolicyRenewalAttemptStatus::Prepared {
            return Err(InternalError::invalid_input(
                "root delegation renewal attempt is not prepared for retrieval",
            ));
        }
        if now_ns >= attempt.retrieval_expires_at_ns || now_ns >= attempt.install_deadline_ns {
            return Err(InternalError::invalid_input(
                "root delegation renewal attempt retrieval window expired",
            ));
        }
        attempts.push(attempt);
    }

    Ok(attempts)
}

pub(super) fn prune_expired_renewal_batches(now_ns: u64) {
    let pruned = AuthStateOps::prune_root_delegation_renewal_batches(now_ns);
    if pruned > 0 {
        crate::log!(
            Topic::Auth,
            Info,
            "root delegated-proof renewal pruned expired batches count={}",
            pruned
        );
    }
}

fn scheduled_renewal_batch_install_attempts(
    batch: &RootDelegationRenewalBatch,
) -> Result<Vec<RootIssuerRenewalAttempt>, InternalError> {
    if batch.attempt_ids.is_empty() {
        return Err(InternalError::invalid_input(
            "root delegation renewal batch has no scheduled attempts",
        ));
    }

    let mut attempts = Vec::with_capacity(batch.attempt_ids.len());
    for attempt_id in &batch.attempt_ids {
        let attempt = AuthStateOps::root_issuer_renewal_attempt(*attempt_id).ok_or_else(|| {
            InternalError::invalid_input("root delegation renewal attempt is not scheduled")
        })?;
        if attempt.batch_id != batch.batch_id {
            return Err(InternalError::invariant(
                crate::InternalErrorOrigin::Ops,
                "root delegation renewal attempt batch mismatch",
            ));
        }
        attempts.push(attempt);
    }

    Ok(attempts)
}

fn scheduled_renewal_batch_install_deadline_expired(
    attempts: &[RootIssuerRenewalAttempt],
    now_ns: u64,
) -> bool {
    attempts
        .iter()
        .any(|attempt| scheduled_renewal_attempt_install_deadline_expired(attempt, now_ns))
}

fn record_scheduled_renewal_batch_install_deadline_expired(
    attempts: Vec<RootIssuerRenewalAttempt>,
    now_ns: u64,
) {
    for attempt in attempts {
        if scheduled_renewal_attempt_install_deadline_expired(&attempt, now_ns) {
            record_scheduled_renewal_attempt_failure(
                attempt,
                PolicyRenewalOutcome::InstallDeadlineExpired,
                PolicyRenewalAttemptStatus::Expired,
                false,
                now_ns,
            );
        }
    }
}

const fn scheduled_renewal_attempt_install_deadline_expired(
    attempt: &RootIssuerRenewalAttempt,
    now_ns: u64,
) -> bool {
    matches!(
        attempt.status,
        PolicyRenewalAttemptStatus::Prepared
            | PolicyRenewalAttemptStatus::Installing
            | PolicyRenewalAttemptStatus::FailedRetryable
    ) && (now_ns >= attempt.install_deadline_ns || now_ns >= attempt.prepared_expires_at_ns)
}
