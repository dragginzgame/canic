//! Module: ops::auth::delegation::root_issuer_renewal::schedule
//!
//! Responsibility: select due templates and persist prepared renewal proof batches.
//! Does not own: template administration, provisioner ACLs, install calls, or DTO view conversion.

use super::{
    ROOT_DELEGATION_RENEWAL_RETRY_BACKOFF_NS,
    identity::{renewal_attempt_id, renewal_batch_id, renewal_template_fingerprint},
    install::delegated_auth_reason_from_renewal_attempt_outcome,
    retrieval::prune_expired_renewal_batches,
};
use crate::{
    InternalError,
    domain::policy::auth::{
        RootDelegationRenewalBatch, RootIssuerRenewalAttempt,
        RootIssuerRenewalAttemptStatus as PolicyRenewalAttemptStatus,
        RootIssuerRenewalOutcome as PolicyRenewalOutcome, RootIssuerRenewalProofRef,
        RootIssuerRenewalState, RootIssuerRenewalTemplate,
    },
    dto::auth::{
        AuthRequestMetadata, RootDelegationProofBatchPrepareEntry,
        RootDelegationProofBatchPrepareRequest, RootDelegationProofBatchPrepareResponse,
    },
    log::Topic,
    ops::{
        auth::delegation::{
            RootDelegationRenewalSweepResult,
            root_issuer_policy::{delegated_role_grant_views, delegation_audience_view},
        },
        runtime::metrics::delegated_auth::{
            DelegatedAuthMetricOutcome, DelegatedAuthMetricReason, DelegatedAuthMetrics,
        },
        storage::auth::AuthStateOps,
    },
};

const ROOT_DELEGATION_RENEWAL_RETRIEVAL_TTL_NS: u64 = 60_000_000_000;

pub(in crate::ops::auth::delegation) fn prepare_due_delegation_renewals(
    max_cert_ttl_ns: u64,
    now_ns: u64,
) -> Result<RootDelegationRenewalSweepResult, InternalError> {
    prepare_due_delegation_renewals_with_prepare(max_cert_ttl_ns, now_ns, |request| {
        super::super::batch::prepare_delegation_proof_batch(request, max_cert_ttl_ns, now_ns)
    })
}

pub(super) fn prepare_due_delegation_renewals_with_prepare(
    _max_cert_ttl_ns: u64,
    now_ns: u64,
    prepare_batch: impl FnOnce(
        RootDelegationProofBatchPrepareRequest,
    ) -> Result<RootDelegationProofBatchPrepareResponse, InternalError>,
) -> Result<RootDelegationRenewalSweepResult, InternalError> {
    prune_expired_renewal_batches(now_ns);

    let mut due_templates = due_renewal_templates(now_ns);
    due_templates.sort_by(|left, right| {
        left.template
            .issuer_pid
            .as_slice()
            .cmp(right.template.issuer_pid.as_slice())
    });

    if due_templates.is_empty() {
        return Ok(RootDelegationRenewalSweepResult {
            prepared_batch_id: None,
            prepared_attempts: 0,
            skipped_templates: enabled_template_count(),
        });
    }

    let batch_id = renewal_batch_id(
        now_ns,
        due_templates.len(),
        due_templates
            .iter()
            .map(|due| (due.template.issuer_pid, due.template_fingerprint)),
    );
    let request = RootDelegationProofBatchPrepareRequest {
        metadata: Some(AuthRequestMetadata {
            request_id: batch_id,
            ttl_ns: ROOT_DELEGATION_RENEWAL_RETRIEVAL_TTL_NS,
        }),
        entries: due_templates
            .iter()
            .map(|due| renewal_prepare_entry(&due.template))
            .collect(),
    };
    let response = match prepare_batch(request) {
        Ok(response) => response,
        Err(err) => {
            record_due_renewal_prepare_failure(
                now_ns,
                &due_templates,
                renewal_prepare_failure_outcome(&err),
            );
            return Err(err);
        }
    };
    persist_scheduled_renewal_batch(now_ns, &due_templates, &response)?;

    Ok(RootDelegationRenewalSweepResult {
        prepared_batch_id: Some(response.batch_id),
        prepared_attempts: response.entries.len(),
        skipped_templates: enabled_template_count().saturating_sub(response.entries.len()),
    })
}

fn record_due_renewal_prepare_failure(
    now_ns: u64,
    due_templates: &[DueRenewalTemplate],
    outcome: PolicyRenewalOutcome,
) {
    for due in due_templates {
        AuthStateOps::upsert_root_issuer_renewal_state(renewal_state_for_prepare_failure(
            now_ns, due, outcome,
        ));
        DelegatedAuthMetrics::record_renewal_attempt(
            DelegatedAuthMetricOutcome::Failed,
            delegated_auth_reason_from_renewal_attempt_outcome(outcome, false),
        );
        crate::log!(
            Topic::Auth,
            Warn,
            "root delegated-proof renewal prepare failed issuer={} outcome={:?}",
            due.template.issuer_pid,
            outcome
        );
    }
}

fn renewal_prepare_failure_outcome(err: &InternalError) -> PolicyRenewalOutcome {
    if err.is_public_resource_exhausted() {
        return PolicyRenewalOutcome::QuotaExceeded;
    }

    PolicyRenewalOutcome::PolicyRejected
}

#[derive(Clone)]
struct DueRenewalTemplate {
    template: RootIssuerRenewalTemplate,
    template_fingerprint: [u8; 32],
    existing_state: Option<RootIssuerRenewalState>,
}

fn due_renewal_templates(now_ns: u64) -> Vec<DueRenewalTemplate> {
    AuthStateOps::root_issuer_renewal_templates()
        .into_iter()
        .filter(|template| template.enabled)
        .filter_map(|template| {
            let template_fingerprint = renewal_template_fingerprint(&template);
            let existing_state = AuthStateOps::root_issuer_renewal_state(template.issuer_pid)
                .map(|state| expire_stale_active_renewal_attempt(now_ns, state));
            if renewal_template_due(now_ns, template_fingerprint, existing_state.as_ref()) {
                Some(DueRenewalTemplate {
                    template,
                    template_fingerprint,
                    existing_state,
                })
            } else {
                None
            }
        })
        .collect()
}

fn expire_stale_active_renewal_attempt(
    now_ns: u64,
    mut state: RootIssuerRenewalState,
) -> RootIssuerRenewalState {
    let Some(attempt_id) = state.active_attempt_id else {
        return state;
    };
    let Some(mut attempt) = AuthStateOps::root_issuer_renewal_attempt(attempt_id) else {
        state.active_attempt_id = None;
        state.updated_at_ns = now_ns;
        AuthStateOps::upsert_root_issuer_renewal_state(state.clone());
        return state;
    };
    if !matches!(
        attempt.status,
        PolicyRenewalAttemptStatus::Prepared
            | PolicyRenewalAttemptStatus::Installing
            | PolicyRenewalAttemptStatus::FailedRetryable
    ) || now_ns < attempt.install_deadline_ns
    {
        return state;
    }

    let outcome = if attempt.status == PolicyRenewalAttemptStatus::Prepared {
        PolicyRenewalOutcome::RetrievalExpired
    } else {
        PolicyRenewalOutcome::InstallDeadlineExpired
    };
    attempt.status = PolicyRenewalAttemptStatus::Expired;
    attempt.failure = Some(outcome);
    AuthStateOps::upsert_root_issuer_renewal_attempt(attempt.clone());

    state.active_attempt_id = None;
    state.last_outcome = outcome;
    state.consecutive_failures = state.consecutive_failures.saturating_add(1);
    state.next_attempt_after_ns = now_ns;
    state.updated_at_ns = now_ns;
    AuthStateOps::upsert_root_issuer_renewal_state(state.clone());

    match outcome {
        PolicyRenewalOutcome::RetrievalExpired => {
            DelegatedAuthMetrics::record_renewal_proof_retrieve_failed(
                DelegatedAuthMetricReason::CertExpired,
            );
        }
        _ => DelegatedAuthMetrics::record_renewal_install(
            DelegatedAuthMetricOutcome::Failed,
            DelegatedAuthMetricReason::CertExpired,
        ),
    }
    DelegatedAuthMetrics::record_renewal_attempt(
        DelegatedAuthMetricOutcome::Failed,
        delegated_auth_reason_from_renewal_attempt_outcome(outcome, false),
    );
    crate::log!(
        Topic::Auth,
        Warn,
        "root delegated-proof renewal attempt expired attempt_id={:?} issuer={} cert_hash={:?} outcome={:?}",
        attempt.attempt_id,
        attempt.issuer_pid,
        attempt.prepared_cert_hash,
        outcome
    );

    state
}

pub(super) fn renewal_template_due(
    now_ns: u64,
    template_fingerprint: [u8; 32],
    state: Option<&RootIssuerRenewalState>,
) -> bool {
    let Some(state) = state else {
        return true;
    };
    if active_attempt_blocks_new_prepare(now_ns, state.active_attempt_id) {
        return false;
    }
    if now_ns < state.next_attempt_after_ns {
        return false;
    }
    if state.template_fingerprint != template_fingerprint {
        return true;
    }
    state
        .last_installed_refresh_after_ns
        .is_none_or(|refresh_after_ns| now_ns >= refresh_after_ns)
}

fn active_attempt_blocks_new_prepare(now_ns: u64, active_attempt_id: Option<[u8; 32]>) -> bool {
    let Some(attempt_id) = active_attempt_id else {
        return false;
    };
    let Some(attempt) = AuthStateOps::root_issuer_renewal_attempt(attempt_id) else {
        return false;
    };
    matches!(
        attempt.status,
        PolicyRenewalAttemptStatus::Prepared
            | PolicyRenewalAttemptStatus::Installing
            | PolicyRenewalAttemptStatus::FailedRetryable
    ) && now_ns < attempt.install_deadline_ns
}

fn enabled_template_count() -> usize {
    AuthStateOps::root_issuer_renewal_templates()
        .into_iter()
        .filter(|template| template.enabled)
        .count()
}

fn renewal_prepare_entry(
    template: &RootIssuerRenewalTemplate,
) -> RootDelegationProofBatchPrepareEntry {
    RootDelegationProofBatchPrepareEntry {
        issuer_pid: template.issuer_pid,
        aud: delegation_audience_view(&template.audience),
        grants: delegated_role_grant_views(&template.grants),
        cert_ttl_ns: template.cert_ttl_ns,
    }
}

fn persist_scheduled_renewal_batch(
    now_ns: u64,
    due_templates: &[DueRenewalTemplate],
    response: &RootDelegationProofBatchPrepareResponse,
) -> Result<(), InternalError> {
    if due_templates.len() != response.entries.len() {
        return Err(InternalError::invariant(
            crate::InternalErrorOrigin::Ops,
            "root delegation renewal prepare response count mismatch",
        ));
    }

    let mut attempt_ids = Vec::with_capacity(response.entries.len());
    for (due, entry) in due_templates.iter().zip(&response.entries) {
        if due.template.issuer_pid != entry.issuer_pid {
            return Err(InternalError::invariant(
                crate::InternalErrorOrigin::Ops,
                "root delegation renewal prepare response issuer mismatch",
            ));
        }

        let attempt_id = renewal_attempt_id(response.batch_id, entry.issuer_pid, entry.cert_hash);
        let attempt = RootIssuerRenewalAttempt {
            attempt_id,
            issuer_pid: entry.issuer_pid,
            template_fingerprint: due.template_fingerprint,
            batch_id: response.batch_id,
            proof_ref: RootIssuerRenewalProofRef {
                issuer_pid: entry.issuer_pid,
                cert_hash: entry.cert_hash,
            },
            status: PolicyRenewalAttemptStatus::Prepared,
            prepared_at_ns: now_ns,
            retrieval_expires_at_ns: response.retrieval_expires_at_ns,
            install_deadline_ns: response.retrieval_expires_at_ns,
            prepared_cert_hash: entry.cert_hash,
            prepared_expires_at_ns: entry.expires_at_ns,
            prepared_refresh_after_ns: entry.refresh_after_ns,
            failure: None,
        };
        let state = renewal_state_for_prepared_attempt(now_ns, due, &attempt);

        AuthStateOps::upsert_root_issuer_renewal_attempt(attempt);
        AuthStateOps::upsert_root_issuer_renewal_state(state);
        DelegatedAuthMetrics::record_renewal_attempt(
            DelegatedAuthMetricOutcome::Started,
            DelegatedAuthMetricReason::Ok,
        );
        attempt_ids.push(attempt_id);
    }

    AuthStateOps::upsert_root_delegation_renewal_batch(RootDelegationRenewalBatch {
        batch_id: response.batch_id,
        attempt_ids,
        prepared_at_ns: now_ns,
        retrieval_expires_at_ns: response.retrieval_expires_at_ns,
    });

    Ok(())
}

fn renewal_state_for_prepared_attempt(
    now_ns: u64,
    due: &DueRenewalTemplate,
    attempt: &RootIssuerRenewalAttempt,
) -> RootIssuerRenewalState {
    let existing = due.existing_state.as_ref();
    RootIssuerRenewalState {
        issuer_pid: due.template.issuer_pid,
        template_fingerprint: due.template_fingerprint,
        last_installed_cert_hash: existing.and_then(|state| state.last_installed_cert_hash),
        last_installed_expires_at_ns: existing.and_then(|state| state.last_installed_expires_at_ns),
        last_installed_refresh_after_ns: existing
            .and_then(|state| state.last_installed_refresh_after_ns),
        active_attempt_id: Some(attempt.attempt_id),
        last_outcome: existing.map_or(PolicyRenewalOutcome::NeverRun, |state| state.last_outcome),
        consecutive_failures: existing.map_or(0, |state| state.consecutive_failures),
        next_attempt_after_ns: existing.map_or(0, |state| state.next_attempt_after_ns),
        updated_at_ns: now_ns,
    }
}

fn renewal_state_for_prepare_failure(
    now_ns: u64,
    due: &DueRenewalTemplate,
    outcome: PolicyRenewalOutcome,
) -> RootIssuerRenewalState {
    let existing = due.existing_state.as_ref();
    RootIssuerRenewalState {
        issuer_pid: due.template.issuer_pid,
        template_fingerprint: due.template_fingerprint,
        last_installed_cert_hash: existing.and_then(|state| state.last_installed_cert_hash),
        last_installed_expires_at_ns: existing.and_then(|state| state.last_installed_expires_at_ns),
        last_installed_refresh_after_ns: existing
            .and_then(|state| state.last_installed_refresh_after_ns),
        active_attempt_id: None,
        last_outcome: outcome,
        consecutive_failures: existing
            .map_or(0, |state| state.consecutive_failures)
            .saturating_add(1),
        next_attempt_after_ns: now_ns.saturating_add(ROOT_DELEGATION_RENEWAL_RETRY_BACKOFF_NS),
        updated_at_ns: now_ns,
    }
}
