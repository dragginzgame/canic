//! Module: ops::auth::delegation::root_issuer_renewal
//!
//! Responsibility: map and validate root-managed issuer renewal boundary DTOs.
//! Does not own: renewal scheduling, proof retrieval, or issuer install calls.

mod identity;
mod install;
#[cfg(test)]
mod tests;
mod view;

use super::{
    RootDelegationRenewalSweepResult,
    errors::map_root_provisioning_policy_error,
    root_issuer_policy::{
        audience_policy, delegated_role_grant_views, delegation_audience_view, grant_policies,
    },
};
use crate::{
    InternalError,
    cdk::types::Principal,
    domain::policy::auth::{
        RootDelegationRenewalBatch, RootIssuerRenewalAttempt,
        RootIssuerRenewalAttemptStatus as PolicyRenewalAttemptStatus,
        RootIssuerRenewalOutcome as PolicyRenewalOutcome, RootIssuerRenewalProofRef,
        RootIssuerRenewalState, RootIssuerRenewalTemplate,
        validate_root_issuer_renewal_template_policy,
    },
    dto::auth::{
        AuthRequestMetadata, RootDelegationProofBatchGetRequest,
        RootDelegationProofBatchGetResponse, RootDelegationProofBatchPrepareEntry,
        RootDelegationProofBatchPrepareRequest, RootDelegationProofBatchPrepareResponse,
        RootDelegationProofBatchProof, RootDelegationProofBatchProofRef,
        RootDelegationProofInstallOutcome, RootDelegationRenewalProofBatchGetRequest,
        RootDelegationRenewalProvisionerListResponse, RootDelegationRenewalProvisionerResponse,
        RootDelegationRenewalProvisionerUpsertRequest, RootDelegationRenewalWorkListResponse,
        RootIssuerRenewalStatusRequest, RootIssuerRenewalStatusResponse,
        RootIssuerRenewalTemplateResponse, RootIssuerRenewalTemplateUpsertRequest,
    },
    log::Topic,
    ops::{
        ic::IcOps,
        runtime::metrics::delegated_auth::{
            DelegatedAuthMetricOutcome, DelegatedAuthMetricReason, DelegatedAuthMetrics,
        },
        storage::auth::{AuthStateOps, RootDelegationRenewalProvisioner},
    },
};

use identity::{renewal_attempt_id, renewal_batch_id, renewal_template_fingerprint};
use install::{
    delegated_auth_reason_from_renewal_attempt_outcome, record_scheduled_renewal_attempt_failure,
};
use view::{
    delegation_renewal_provisioner_view, root_delegation_renewal_batch_view,
    root_issuer_renewal_attempt_view, root_issuer_renewal_state_view,
    root_issuer_renewal_template_view,
};

const ROOT_DELEGATION_RENEWAL_RETRY_BACKOFF_NS: u64 = 60_000_000_000;
const ROOT_DELEGATION_RENEWAL_RETRIEVAL_TTL_NS: u64 = 60_000_000_000;

pub(super) fn upsert_root_issuer_renewal_template(
    request: RootIssuerRenewalTemplateUpsertRequest,
    now_ns: u64,
) -> Result<RootIssuerRenewalTemplateResponse, InternalError> {
    validate_root_issuer_renewal_template_upsert_request(&request)?;

    let template = root_issuer_renewal_template_from_request(request);
    let policy = AuthStateOps::root_issuer_policy(template.issuer_pid);
    validate_root_issuer_renewal_template_policy(policy.as_ref(), &template)
        .map_err(map_root_provisioning_policy_error)?;

    AuthStateOps::upsert_root_issuer_renewal_template(template.clone());
    if !template.enabled {
        disable_active_renewal_attempt(&template, now_ns);
    }
    crate::log!(
        Topic::Auth,
        Info,
        "root issuer renewal template updated issuer={} enabled={}",
        template.issuer_pid,
        template.enabled
    );

    Ok(RootIssuerRenewalTemplateResponse {
        template: root_issuer_renewal_template_view(&template),
    })
}

pub(super) fn root_issuer_renewal_status(
    request: RootIssuerRenewalStatusRequest,
) -> RootIssuerRenewalStatusResponse {
    let state = AuthStateOps::root_issuer_renewal_state(request.issuer_pid);
    let active_attempt = state
        .as_ref()
        .and_then(|state| state.active_attempt_id)
        .and_then(AuthStateOps::root_issuer_renewal_attempt)
        .map(|attempt| root_issuer_renewal_attempt_view(&attempt));

    RootIssuerRenewalStatusResponse {
        template: AuthStateOps::root_issuer_renewal_template(request.issuer_pid)
            .map(|template| root_issuer_renewal_template_view(&template)),
        state: state.map(|state| root_issuer_renewal_state_view(&state)),
        active_attempt,
    }
}

fn disable_active_renewal_attempt(template: &RootIssuerRenewalTemplate, now_ns: u64) {
    let Some(mut state) = AuthStateOps::root_issuer_renewal_state(template.issuer_pid) else {
        return;
    };
    if let Some(attempt_id) = state.active_attempt_id
        && let Some(mut attempt) = AuthStateOps::root_issuer_renewal_attempt(attempt_id)
        && matches!(
            attempt.status,
            PolicyRenewalAttemptStatus::Prepared
                | PolicyRenewalAttemptStatus::Installing
                | PolicyRenewalAttemptStatus::FailedRetryable
        )
    {
        attempt.status = PolicyRenewalAttemptStatus::Disabled;
        attempt.failure = Some(PolicyRenewalOutcome::TemplateDisabled);
        AuthStateOps::upsert_root_issuer_renewal_attempt(attempt);
        DelegatedAuthMetrics::record_renewal_attempt(
            DelegatedAuthMetricOutcome::Failed,
            DelegatedAuthMetricReason::Disabled,
        );
    }

    state.template_fingerprint = renewal_template_fingerprint(template);
    state.active_attempt_id = None;
    state.last_outcome = PolicyRenewalOutcome::TemplateDisabled;
    state.next_attempt_after_ns = now_ns;
    state.updated_at_ns = now_ns;
    AuthStateOps::upsert_root_issuer_renewal_state(state);
}

pub(super) fn has_enabled_root_issuer_renewal_templates() -> bool {
    AuthStateOps::root_issuer_renewal_templates()
        .iter()
        .any(|template| template.enabled)
}

pub(super) fn upsert_delegation_renewal_provisioner(
    request: RootDelegationRenewalProvisionerUpsertRequest,
) -> RootDelegationRenewalProvisionerResponse {
    let provisioner = RootDelegationRenewalProvisioner {
        principal: request.principal,
        enabled: request.enabled,
    };
    AuthStateOps::upsert_root_delegation_renewal_provisioner(provisioner);
    DelegatedAuthMetrics::record_renewal_provisioner_completed();
    crate::log!(
        Topic::Auth,
        Info,
        "root delegated-proof renewal provisioner updated principal={} enabled={}",
        provisioner.principal,
        provisioner.enabled
    );

    RootDelegationRenewalProvisionerResponse {
        provisioner: delegation_renewal_provisioner_view(provisioner),
    }
}

pub(super) fn delegation_renewal_provisioners() -> RootDelegationRenewalProvisionerListResponse {
    let mut provisioners = AuthStateOps::root_delegation_renewal_provisioners();
    provisioners.sort_by(|left, right| left.principal.as_slice().cmp(right.principal.as_slice()));

    RootDelegationRenewalProvisionerListResponse {
        provisioners: provisioners
            .into_iter()
            .map(delegation_renewal_provisioner_view)
            .collect(),
    }
}

pub(super) fn delegation_renewal_work(now_ns: u64) -> RootDelegationRenewalWorkListResponse {
    let mut batches = AuthStateOps::root_delegation_renewal_batches()
        .into_iter()
        .filter_map(|batch| root_delegation_renewal_batch_view(&batch, now_ns))
        .collect::<Vec<_>>();
    batches.sort_by(|left, right| {
        left.prepared_at_ns
            .cmp(&right.prepared_at_ns)
            .then_with(|| left.batch_id.cmp(&right.batch_id))
    });

    RootDelegationRenewalWorkListResponse { batches }
}

pub(super) fn is_delegation_renewal_provisioner(principal: Principal) -> bool {
    AuthStateOps::is_root_delegation_renewal_provisioner(principal)
}

pub(super) fn ensure_delegation_renewal_batch_scheduled(
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

pub(super) fn prepare_due_delegation_renewals(
    max_cert_ttl_ns: u64,
    now_ns: u64,
) -> Result<RootDelegationRenewalSweepResult, InternalError> {
    prepare_due_delegation_renewals_with_prepare(max_cert_ttl_ns, now_ns, |request| {
        super::batch::prepare_delegation_proof_batch(request, max_cert_ttl_ns, now_ns)
    })
}

fn prepare_due_delegation_renewals_with_prepare(
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

    let batch_id = renewal_batch_id(now_ns, &due_templates);
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

fn prune_expired_renewal_batches(now_ns: u64) {
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

pub(super) fn get_delegation_renewal_proof_batch(
    request: RootDelegationRenewalProofBatchGetRequest,
) -> Result<RootDelegationProofBatchGetResponse, InternalError> {
    get_delegation_renewal_proof_batch_with_getter(request, IcOps::now_nanos(), |request| {
        super::batch::get_delegation_proof_batch(request)
    })
}

fn get_delegation_renewal_proof_batch_with_getter(
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

fn scheduled_renewal_batch_attempts(
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

pub(super) fn preflight_delegation_renewal_proof_install(
    batch_id: [u8; 32],
    proof: &RootDelegationProofBatchProof,
    now_ns: u64,
) -> Result<Option<[u8; 32]>, RootDelegationProofInstallOutcome> {
    install::preflight_delegation_renewal_proof_install(batch_id, proof, now_ns)
}

pub(super) fn record_delegation_renewal_install_outcome(
    attempt_id: [u8; 32],
    outcome: RootDelegationProofInstallOutcome,
    now_ns: u64,
) {
    install::record_delegation_renewal_install_outcome(attempt_id, outcome, now_ns);
}

pub(super) fn record_delegation_renewal_install_preflight_outcome(
    batch_id: [u8; 32],
    issuer_pid: Principal,
    cert_hash: [u8; 32],
    outcome: RootDelegationProofInstallOutcome,
    now_ns: u64,
) {
    install::record_delegation_renewal_install_preflight_outcome(
        batch_id, issuer_pid, cert_hash, outcome, now_ns,
    );
}

pub(super) fn record_manual_delegation_renewal_install_outcome(
    proof: &RootDelegationProofBatchProof,
    outcome: RootDelegationProofInstallOutcome,
    now_ns: u64,
) {
    install::record_manual_delegation_renewal_install_outcome(proof, outcome, now_ns);
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

fn renewal_template_due(
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

fn validate_root_issuer_renewal_template_upsert_request(
    request: &RootIssuerRenewalTemplateUpsertRequest,
) -> Result<(), InternalError> {
    if request.cert_ttl_ns == 0 {
        return Err(InternalError::invalid_input(
            "root issuer renewal certificate TTL must be greater than zero",
        ));
    }
    if request.enabled && request.grants.is_empty() {
        return Err(InternalError::invalid_input(
            "enabled root issuer renewal template must include at least one grant",
        ));
    }
    Ok(())
}

fn root_issuer_renewal_template_from_request(
    request: RootIssuerRenewalTemplateUpsertRequest,
) -> RootIssuerRenewalTemplate {
    RootIssuerRenewalTemplate {
        issuer_pid: request.issuer_pid,
        enabled: request.enabled,
        audience: audience_policy(&request.aud),
        grants: grant_policies(&request.grants),
        cert_ttl_ns: request.cert_ttl_ns,
    }
}
