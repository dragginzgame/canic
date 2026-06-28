//! Module: ops::auth::delegation::root_issuer_renewal
//!
//! Responsibility: map and validate root-managed issuer renewal boundary DTOs.
//! Does not own: renewal scheduling, proof retrieval, or issuer install calls.

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
        RootDelegatedRoleGrantPolicy, RootDelegationAudiencePolicy,
        RootDelegationProofPreparePolicyDecision, RootDelegationProofPreparePolicyInput,
        RootDelegationRenewalBatch, RootIssuerRenewalAttempt,
        RootIssuerRenewalAttemptStatus as PolicyRenewalAttemptStatus,
        RootIssuerRenewalOutcome as PolicyRenewalOutcome, RootIssuerRenewalProofRef,
        RootIssuerRenewalState, RootIssuerRenewalTemplate,
        validate_root_delegation_proof_prepare_policy,
        validate_root_issuer_renewal_template_policy,
    },
    dto::auth::{
        AuthRequestMetadata, RootDelegationProofBatchGetRequest,
        RootDelegationProofBatchGetResponse, RootDelegationProofBatchPrepareEntry,
        RootDelegationProofBatchPrepareRequest, RootDelegationProofBatchPrepareResponse,
        RootDelegationProofBatchProof, RootDelegationProofBatchProofRef,
        RootDelegationProofInstallOutcome, RootDelegationRenewalBatchView,
        RootDelegationRenewalProofBatchGetRequest, RootDelegationRenewalProvisionerListResponse,
        RootDelegationRenewalProvisionerResponse, RootDelegationRenewalProvisionerUpsertRequest,
        RootDelegationRenewalProvisionerView, RootDelegationRenewalWorkListResponse,
        RootIssuerRenewalAttemptStatus, RootIssuerRenewalAttemptView, RootIssuerRenewalOutcome,
        RootIssuerRenewalStateView, RootIssuerRenewalStatusRequest,
        RootIssuerRenewalStatusResponse, RootIssuerRenewalTemplateResponse,
        RootIssuerRenewalTemplateUpsertRequest, RootIssuerRenewalTemplateView,
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
use sha2::{Digest, Sha256};

const ROOT_DELEGATION_RENEWAL_RETRY_BACKOFF_NS: u64 = 60_000_000_000;
const ROOT_DELEGATION_RENEWAL_RETRIEVAL_TTL_NS: u64 = 60_000_000_000;
const ROOT_ISSUER_RENEWAL_TEMPLATE_FINGERPRINT_DOMAIN: &[u8] =
    b"canic-root-issuer-renewal-template:v1";
const ROOT_DELEGATION_RENEWAL_BATCH_ID_DOMAIN: &[u8] = b"canic-root-delegation-renewal-batch:v1";
const ROOT_ISSUER_RENEWAL_ATTEMPT_ID_DOMAIN: &[u8] = b"canic-root-issuer-renewal-attempt:v1";

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
    let Some(batch) = AuthStateOps::root_delegation_renewal_batch(batch_id) else {
        return Ok(None);
    };
    let Some(mut attempt) =
        scheduled_renewal_attempt_for_proof(&batch, proof.issuer_pid, proof.cert_hash)
    else {
        return Err(RootDelegationProofInstallOutcome::ProofMismatch);
    };

    if attempt.status == PolicyRenewalAttemptStatus::Installed {
        return Err(RootDelegationProofInstallOutcome::AlreadyInstalled);
    }

    if let Err(rejection) = validate_scheduled_renewal_install(&attempt, proof, now_ns) {
        let public_outcome = rejection.public_outcome.clone();
        record_scheduled_renewal_attempt_rejection(attempt, rejection, now_ns);
        return Err(public_outcome);
    }

    attempt.status = PolicyRenewalAttemptStatus::Installing;
    attempt.failure = None;
    AuthStateOps::upsert_root_issuer_renewal_attempt(attempt.clone());
    upsert_state_for_installing_attempt(&attempt, now_ns);

    Ok(Some(attempt.attempt_id))
}

pub(super) fn record_delegation_renewal_install_outcome(
    attempt_id: [u8; 32],
    outcome: RootDelegationProofInstallOutcome,
    now_ns: u64,
) {
    let Some(attempt) = AuthStateOps::root_issuer_renewal_attempt(attempt_id) else {
        return;
    };
    record_scheduled_renewal_install_outcome(attempt, outcome, now_ns);
}

pub(super) fn record_delegation_renewal_install_preflight_outcome(
    batch_id: [u8; 32],
    issuer_pid: Principal,
    cert_hash: [u8; 32],
    outcome: RootDelegationProofInstallOutcome,
    now_ns: u64,
) {
    let Some(batch) = AuthStateOps::root_delegation_renewal_batch(batch_id) else {
        return;
    };
    let Some(attempt) = scheduled_renewal_attempt_for_proof(&batch, issuer_pid, cert_hash) else {
        return;
    };
    record_scheduled_renewal_install_outcome(attempt, outcome, now_ns);
}

pub(super) fn record_manual_delegation_renewal_install_outcome(
    proof: &RootDelegationProofBatchProof,
    outcome: RootDelegationProofInstallOutcome,
    now_ns: u64,
) {
    if !matches!(
        outcome,
        RootDelegationProofInstallOutcome::Installed
            | RootDelegationProofInstallOutcome::AlreadyInstalled
    ) {
        return;
    }
    let Some(template) = AuthStateOps::root_issuer_renewal_template(proof.issuer_pid) else {
        return;
    };
    let Some(decision) = manual_installed_proof_matches_template(&template, proof) else {
        return;
    };
    let existing_state = AuthStateOps::root_issuer_renewal_state(proof.issuer_pid);
    if existing_state
        .as_ref()
        .is_some_and(|state| state.active_attempt_id.is_some())
    {
        return;
    }

    let template_fingerprint = renewal_template_fingerprint(&template);
    let mut state = existing_state.unwrap_or(RootIssuerRenewalState {
        issuer_pid: proof.issuer_pid,
        template_fingerprint,
        last_installed_cert_hash: None,
        last_installed_expires_at_ns: None,
        last_installed_refresh_after_ns: None,
        active_attempt_id: None,
        last_outcome: PolicyRenewalOutcome::NeverRun,
        consecutive_failures: 0,
        next_attempt_after_ns: 0,
        updated_at_ns: now_ns,
    });
    state.template_fingerprint = template_fingerprint;
    state.last_installed_cert_hash = Some(proof.cert_hash);
    state.last_installed_expires_at_ns = Some(decision.expires_at_ns);
    state.last_installed_refresh_after_ns = Some(decision.refresh_after_ns);
    state.active_attempt_id = None;
    state.last_outcome = if outcome == RootDelegationProofInstallOutcome::AlreadyInstalled {
        PolicyRenewalOutcome::AlreadyInstalled
    } else {
        PolicyRenewalOutcome::Installed
    };
    state.consecutive_failures = 0;
    state.next_attempt_after_ns = decision.refresh_after_ns;
    state.updated_at_ns = now_ns;
    AuthStateOps::upsert_root_issuer_renewal_state(state);
    DelegatedAuthMetrics::record_renewal_install(
        DelegatedAuthMetricOutcome::Completed,
        DelegatedAuthMetricReason::Ok,
    );
    crate::log!(
        Topic::Auth,
        Info,
        "root delegated-proof renewal state updated from manual install issuer={} cert_hash={:?} outcome={:?}",
        proof.issuer_pid,
        proof.cert_hash,
        outcome
    );
}

fn manual_installed_proof_matches_template(
    template: &RootIssuerRenewalTemplate,
    proof: &RootDelegationProofBatchProof,
) -> Option<RootDelegationProofPreparePolicyDecision> {
    if !template.enabled
        || template.issuer_pid != proof.issuer_pid
        || proof.proof.cert.issuer_pid != proof.issuer_pid
        || audience_policy(&proof.proof.cert.aud) != template.audience
        || grant_policies(&proof.proof.cert.grants) != template.grants
    {
        return None;
    }
    let cert_ttl_ns = proof
        .proof
        .cert
        .expires_at_ns
        .checked_sub(proof.proof.cert.issued_at_ns)?;
    if cert_ttl_ns != template.cert_ttl_ns {
        return None;
    }

    let policy = AuthStateOps::root_issuer_policy(template.issuer_pid);
    let decision = validate_root_delegation_proof_prepare_policy(
        policy.as_ref(),
        RootDelegationProofPreparePolicyInput {
            issuer_pid: template.issuer_pid,
            audience: &template.audience,
            grants: &template.grants,
            cert_ttl_ns,
            issued_at_ns: proof.proof.cert.issued_at_ns,
        },
    )
    .ok()?;
    if decision.expires_at_ns != proof.proof.cert.expires_at_ns {
        return None;
    }

    Some(decision)
}

fn scheduled_renewal_attempt_for_proof(
    batch: &RootDelegationRenewalBatch,
    issuer_pid: Principal,
    cert_hash: [u8; 32],
) -> Option<RootIssuerRenewalAttempt> {
    batch
        .attempt_ids
        .iter()
        .filter_map(|attempt_id| AuthStateOps::root_issuer_renewal_attempt(*attempt_id))
        .find(|attempt| {
            attempt.batch_id == batch.batch_id
                && attempt.issuer_pid == issuer_pid
                && attempt.proof_ref.issuer_pid == issuer_pid
                && attempt.proof_ref.cert_hash == cert_hash
        })
}

#[derive(Clone)]
struct ScheduledRenewalInstallRejection {
    public_outcome: RootDelegationProofInstallOutcome,
    renewal_outcome: PolicyRenewalOutcome,
    status: PolicyRenewalAttemptStatus,
}

fn validate_scheduled_renewal_install(
    attempt: &RootIssuerRenewalAttempt,
    proof: &RootDelegationProofBatchProof,
    now_ns: u64,
) -> Result<(), ScheduledRenewalInstallRejection> {
    if attempt.status == PolicyRenewalAttemptStatus::Disabled {
        return Err(scheduled_renewal_rejection(
            RootDelegationProofInstallOutcome::ExpiredOrSuperseded,
            PolicyRenewalOutcome::TemplateDisabled,
            PolicyRenewalAttemptStatus::Disabled,
        ));
    }
    if attempt.status == PolicyRenewalAttemptStatus::Expired
        || attempt.status == PolicyRenewalAttemptStatus::FailedTerminal
    {
        return Err(scheduled_renewal_rejection(
            RootDelegationProofInstallOutcome::ExpiredOrSuperseded,
            PolicyRenewalOutcome::InstallDeadlineExpired,
            attempt.status,
        ));
    }
    if !matches!(
        attempt.status,
        PolicyRenewalAttemptStatus::Prepared
            | PolicyRenewalAttemptStatus::Installing
            | PolicyRenewalAttemptStatus::FailedRetryable
    ) {
        return Err(scheduled_renewal_rejection(
            RootDelegationProofInstallOutcome::ExpiredOrSuperseded,
            PolicyRenewalOutcome::InstallDeadlineExpired,
            PolicyRenewalAttemptStatus::Expired,
        ));
    }
    if now_ns >= attempt.install_deadline_ns || now_ns >= attempt.prepared_expires_at_ns {
        return Err(scheduled_renewal_rejection(
            RootDelegationProofInstallOutcome::ExpiredOrSuperseded,
            PolicyRenewalOutcome::InstallDeadlineExpired,
            PolicyRenewalAttemptStatus::Expired,
        ));
    }
    if attempt.prepared_cert_hash != proof.cert_hash
        || attempt.prepared_expires_at_ns != proof.proof.cert.expires_at_ns
        || proof.proof.cert.issuer_pid != proof.issuer_pid
        || attempt.issuer_pid != proof.issuer_pid
    {
        return Err(scheduled_renewal_rejection(
            RootDelegationProofInstallOutcome::ProofMismatch,
            PolicyRenewalOutcome::ProofMismatch,
            PolicyRenewalAttemptStatus::FailedTerminal,
        ));
    }

    let Some(template) = AuthStateOps::root_issuer_renewal_template(attempt.issuer_pid) else {
        return Err(scheduled_renewal_rejection(
            RootDelegationProofInstallOutcome::ExpiredOrSuperseded,
            PolicyRenewalOutcome::TemplateDisabled,
            PolicyRenewalAttemptStatus::Disabled,
        ));
    };
    if !template.enabled {
        return Err(scheduled_renewal_rejection(
            RootDelegationProofInstallOutcome::ExpiredOrSuperseded,
            PolicyRenewalOutcome::TemplateDisabled,
            PolicyRenewalAttemptStatus::Disabled,
        ));
    }
    if renewal_template_fingerprint(&template) != attempt.template_fingerprint {
        return Err(scheduled_renewal_rejection(
            RootDelegationProofInstallOutcome::ExpiredOrSuperseded,
            PolicyRenewalOutcome::TemplateChanged,
            PolicyRenewalAttemptStatus::FailedTerminal,
        ));
    }

    Ok(())
}

const fn scheduled_renewal_rejection(
    public_outcome: RootDelegationProofInstallOutcome,
    renewal_outcome: PolicyRenewalOutcome,
    status: PolicyRenewalAttemptStatus,
) -> ScheduledRenewalInstallRejection {
    ScheduledRenewalInstallRejection {
        public_outcome,
        renewal_outcome,
        status,
    }
}

fn record_scheduled_renewal_attempt_rejection(
    attempt: RootIssuerRenewalAttempt,
    rejection: ScheduledRenewalInstallRejection,
    now_ns: u64,
) {
    record_scheduled_renewal_attempt_failure(
        attempt,
        rejection.renewal_outcome,
        rejection.status,
        false,
        now_ns,
    );
}

fn record_scheduled_renewal_install_outcome(
    attempt: RootIssuerRenewalAttempt,
    outcome: RootDelegationProofInstallOutcome,
    now_ns: u64,
) {
    match outcome {
        RootDelegationProofInstallOutcome::Installed
        | RootDelegationProofInstallOutcome::AlreadyInstalled => {
            record_scheduled_renewal_attempt_success(attempt, outcome, now_ns);
        }
        RootDelegationProofInstallOutcome::CallFailed => {
            record_scheduled_renewal_attempt_failure(
                attempt,
                PolicyRenewalOutcome::IssuerCallFailed,
                PolicyRenewalAttemptStatus::FailedRetryable,
                true,
                now_ns,
            );
        }
        RootDelegationProofInstallOutcome::RejectedBySigner => {
            record_scheduled_renewal_attempt_failure(
                attempt,
                PolicyRenewalOutcome::RejectedByIssuer,
                PolicyRenewalAttemptStatus::FailedTerminal,
                false,
                now_ns,
            );
        }
        RootDelegationProofInstallOutcome::ProofMismatch => {
            record_scheduled_renewal_attempt_failure(
                attempt,
                PolicyRenewalOutcome::ProofMismatch,
                PolicyRenewalAttemptStatus::FailedTerminal,
                false,
                now_ns,
            );
        }
        RootDelegationProofInstallOutcome::ExpiredOrSuperseded => {
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

fn record_scheduled_renewal_attempt_success(
    mut attempt: RootIssuerRenewalAttempt,
    outcome: RootDelegationProofInstallOutcome,
    now_ns: u64,
) {
    let was_installed = attempt.status == PolicyRenewalAttemptStatus::Installed;
    attempt.status = PolicyRenewalAttemptStatus::Installed;
    attempt.failure = None;
    AuthStateOps::upsert_root_issuer_renewal_attempt(attempt.clone());

    let mut state = renewal_state_for_attempt(&attempt, now_ns);
    state.last_installed_cert_hash = Some(attempt.prepared_cert_hash);
    state.last_installed_expires_at_ns = Some(attempt.prepared_expires_at_ns);
    state.last_installed_refresh_after_ns = Some(attempt.prepared_refresh_after_ns);
    state.active_attempt_id = None;
    state.last_outcome = match outcome {
        RootDelegationProofInstallOutcome::AlreadyInstalled => {
            PolicyRenewalOutcome::AlreadyInstalled
        }
        _ => PolicyRenewalOutcome::Installed,
    };
    state.consecutive_failures = 0;
    state.next_attempt_after_ns = attempt.prepared_refresh_after_ns;
    state.updated_at_ns = now_ns;
    AuthStateOps::upsert_root_issuer_renewal_state(state);
    DelegatedAuthMetrics::record_renewal_install(
        DelegatedAuthMetricOutcome::Completed,
        DelegatedAuthMetricReason::Ok,
    );
    if !was_installed {
        DelegatedAuthMetrics::record_renewal_attempt(
            DelegatedAuthMetricOutcome::Completed,
            DelegatedAuthMetricReason::Ok,
        );
    }
    crate::log!(
        Topic::Auth,
        Info,
        "root delegated-proof renewal install completed attempt_id={:?} issuer={} cert_hash={:?} outcome={:?}",
        attempt.attempt_id,
        attempt.issuer_pid,
        attempt.prepared_cert_hash,
        outcome
    );
}

fn record_scheduled_renewal_attempt_failure(
    mut attempt: RootIssuerRenewalAttempt,
    outcome: PolicyRenewalOutcome,
    status: PolicyRenewalAttemptStatus,
    retryable: bool,
    now_ns: u64,
) {
    attempt.status = status;
    attempt.failure = Some(outcome);
    AuthStateOps::upsert_root_issuer_renewal_attempt(attempt.clone());

    let mut state = renewal_state_for_attempt(&attempt, now_ns);
    state.active_attempt_id = if retryable && now_ns < attempt.install_deadline_ns {
        Some(attempt.attempt_id)
    } else {
        None
    };
    state.last_outcome = outcome;
    state.consecutive_failures = state.consecutive_failures.saturating_add(1);
    state.next_attempt_after_ns = retry_after_ns(now_ns, attempt.install_deadline_ns);
    state.updated_at_ns = now_ns;
    AuthStateOps::upsert_root_issuer_renewal_state(state);
    DelegatedAuthMetrics::record_renewal_install(
        DelegatedAuthMetricOutcome::Failed,
        delegated_auth_reason_from_renewal_outcome(outcome),
    );
    DelegatedAuthMetrics::record_renewal_attempt(
        DelegatedAuthMetricOutcome::Failed,
        delegated_auth_reason_from_renewal_attempt_outcome(outcome, retryable),
    );
    crate::log!(
        Topic::Auth,
        Warn,
        "root delegated-proof renewal install failed attempt_id={:?} issuer={} cert_hash={:?} outcome={:?} status={:?} retryable={}",
        attempt.attempt_id,
        attempt.issuer_pid,
        attempt.prepared_cert_hash,
        outcome,
        status,
        retryable
    );
}

fn upsert_state_for_installing_attempt(attempt: &RootIssuerRenewalAttempt, now_ns: u64) {
    let mut state = renewal_state_for_attempt(attempt, now_ns);
    state.active_attempt_id = Some(attempt.attempt_id);
    state.updated_at_ns = now_ns;
    AuthStateOps::upsert_root_issuer_renewal_state(state);
}

fn renewal_state_for_attempt(
    attempt: &RootIssuerRenewalAttempt,
    now_ns: u64,
) -> RootIssuerRenewalState {
    AuthStateOps::root_issuer_renewal_state(attempt.issuer_pid).map_or_else(
        || RootIssuerRenewalState {
            issuer_pid: attempt.issuer_pid,
            template_fingerprint: attempt.template_fingerprint,
            last_installed_cert_hash: None,
            last_installed_expires_at_ns: None,
            last_installed_refresh_after_ns: None,
            active_attempt_id: Some(attempt.attempt_id),
            last_outcome: PolicyRenewalOutcome::NeverRun,
            consecutive_failures: 0,
            next_attempt_after_ns: 0,
            updated_at_ns: now_ns,
        },
        |mut state| {
            state.template_fingerprint = attempt.template_fingerprint;
            state
        },
    )
}

fn retry_after_ns(now_ns: u64, install_deadline_ns: u64) -> u64 {
    now_ns
        .saturating_add(ROOT_DELEGATION_RENEWAL_RETRY_BACKOFF_NS)
        .min(install_deadline_ns)
}

const fn delegated_auth_reason_from_renewal_outcome(
    outcome: PolicyRenewalOutcome,
) -> DelegatedAuthMetricReason {
    match outcome {
        PolicyRenewalOutcome::AlreadyInstalled
        | PolicyRenewalOutcome::Installed
        | PolicyRenewalOutcome::NeverRun => DelegatedAuthMetricReason::Ok,
        PolicyRenewalOutcome::DriftDetected
        | PolicyRenewalOutcome::PolicyRejected
        | PolicyRenewalOutcome::QuotaExceeded
        | PolicyRenewalOutcome::TemplateChanged => DelegatedAuthMetricReason::InvalidState,
        PolicyRenewalOutcome::InstallDeadlineExpired | PolicyRenewalOutcome::RetrievalExpired => {
            DelegatedAuthMetricReason::CertExpired
        }
        PolicyRenewalOutcome::IssuerCallFailed => DelegatedAuthMetricReason::IssuerProofUnavailable,
        PolicyRenewalOutcome::ProofMismatch => DelegatedAuthMetricReason::CertHashMismatch,
        PolicyRenewalOutcome::RejectedByIssuer => DelegatedAuthMetricReason::IssuerProofInvalid,
        PolicyRenewalOutcome::TemplateDisabled => DelegatedAuthMetricReason::Disabled,
    }
}

const fn delegated_auth_reason_from_renewal_attempt_outcome(
    outcome: PolicyRenewalOutcome,
    retryable: bool,
) -> DelegatedAuthMetricReason {
    if retryable {
        return DelegatedAuthMetricReason::RetryScheduled;
    }

    match outcome {
        PolicyRenewalOutcome::AlreadyInstalled
        | PolicyRenewalOutcome::Installed
        | PolicyRenewalOutcome::NeverRun => DelegatedAuthMetricReason::Ok,
        PolicyRenewalOutcome::DriftDetected => DelegatedAuthMetricReason::DriftDetected,
        PolicyRenewalOutcome::InstallDeadlineExpired => {
            DelegatedAuthMetricReason::InstallDeadlineExpired
        }
        PolicyRenewalOutcome::RetrievalExpired => DelegatedAuthMetricReason::RetrievalExpired,
        PolicyRenewalOutcome::IssuerCallFailed => DelegatedAuthMetricReason::IssuerProofUnavailable,
        PolicyRenewalOutcome::PolicyRejected
        | PolicyRenewalOutcome::QuotaExceeded
        | PolicyRenewalOutcome::TemplateChanged => DelegatedAuthMetricReason::InvalidState,
        PolicyRenewalOutcome::ProofMismatch => DelegatedAuthMetricReason::CertHashMismatch,
        PolicyRenewalOutcome::RejectedByIssuer => DelegatedAuthMetricReason::IssuerProofInvalid,
        PolicyRenewalOutcome::TemplateDisabled => DelegatedAuthMetricReason::Disabled,
    }
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

fn renewal_template_fingerprint(template: &RootIssuerRenewalTemplate) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hash_renewal_bytes(&mut hasher, ROOT_ISSUER_RENEWAL_TEMPLATE_FINGERPRINT_DOMAIN);
    hash_renewal_principal(&mut hasher, template.issuer_pid);
    hash_renewal_bool(&mut hasher, template.enabled);
    hash_renewal_policy_audience(&mut hasher, &template.audience);
    hash_renewal_policy_grants(&mut hasher, &template.grants);
    hash_renewal_u64(&mut hasher, template.cert_ttl_ns);
    hasher.finalize().into()
}

fn renewal_batch_id(now_ns: u64, due_templates: &[DueRenewalTemplate]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hash_renewal_bytes(&mut hasher, ROOT_DELEGATION_RENEWAL_BATCH_ID_DOMAIN);
    hash_renewal_u64(&mut hasher, now_ns);
    hash_renewal_u64(&mut hasher, due_templates.len() as u64);
    for due in due_templates {
        hash_renewal_principal(&mut hasher, due.template.issuer_pid);
        hash_renewal_bytes(&mut hasher, &due.template_fingerprint);
    }
    hasher.finalize().into()
}

fn renewal_attempt_id(batch_id: [u8; 32], issuer_pid: Principal, cert_hash: [u8; 32]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hash_renewal_bytes(&mut hasher, ROOT_ISSUER_RENEWAL_ATTEMPT_ID_DOMAIN);
    hash_renewal_bytes(&mut hasher, &batch_id);
    hash_renewal_principal(&mut hasher, issuer_pid);
    hash_renewal_bytes(&mut hasher, &cert_hash);
    hasher.finalize().into()
}

fn hash_renewal_policy_audience(hasher: &mut Sha256, audience: &RootDelegationAudiencePolicy) {
    match audience {
        RootDelegationAudiencePolicy::Canister(canister) => {
            hash_renewal_bytes(hasher, b"canister");
            hash_renewal_principal(hasher, *canister);
        }
        RootDelegationAudiencePolicy::CanicSubnet(subnet) => {
            hash_renewal_bytes(hasher, b"canic_subnet");
            hash_renewal_principal(hasher, *subnet);
        }
        RootDelegationAudiencePolicy::Project(project) => {
            hash_renewal_bytes(hasher, b"project");
            hash_renewal_bytes(hasher, project.as_bytes());
        }
    }
}

fn hash_renewal_policy_grants(hasher: &mut Sha256, grants: &[RootDelegatedRoleGrantPolicy]) {
    hash_renewal_u64(hasher, grants.len() as u64);
    for grant in grants {
        hash_renewal_bytes(hasher, grant.target.as_str().as_bytes());
        hash_renewal_u64(hasher, grant.scopes.len() as u64);
        for scope in &grant.scopes {
            hash_renewal_bytes(hasher, scope.as_bytes());
        }
    }
}

fn hash_renewal_bool(hasher: &mut Sha256, value: bool) {
    hasher.update([u8::from(value)]);
}

fn hash_renewal_principal(hasher: &mut Sha256, principal: Principal) {
    hash_renewal_bytes(hasher, principal.as_slice());
}

fn hash_renewal_u64(hasher: &mut Sha256, value: u64) {
    hasher.update(value.to_be_bytes());
}

fn hash_renewal_bytes(hasher: &mut Sha256, bytes: &[u8]) {
    hash_renewal_u64(hasher, bytes.len() as u64);
    hasher.update(bytes);
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

fn root_issuer_renewal_template_view(
    template: &RootIssuerRenewalTemplate,
) -> RootIssuerRenewalTemplateView {
    RootIssuerRenewalTemplateView {
        issuer_pid: template.issuer_pid,
        enabled: template.enabled,
        aud: delegation_audience_view(&template.audience),
        grants: delegated_role_grant_views(&template.grants),
        cert_ttl_ns: template.cert_ttl_ns,
    }
}

fn root_delegation_renewal_batch_view(
    batch: &RootDelegationRenewalBatch,
    now_ns: u64,
) -> Option<RootDelegationRenewalBatchView> {
    let attempts = scheduled_renewal_batch_attempts(batch, now_ns).ok()?;
    let install_deadline_ns = attempts
        .iter()
        .map(|attempt| attempt.install_deadline_ns)
        .min()
        .unwrap_or(batch.retrieval_expires_at_ns);

    Some(RootDelegationRenewalBatchView {
        batch_id: batch.batch_id,
        attempt_count: attempts.len() as u64,
        prepared_at_ns: batch.prepared_at_ns,
        retrieval_expires_at_ns: batch.retrieval_expires_at_ns,
        install_deadline_ns,
        attempts: attempts
            .iter()
            .map(root_issuer_renewal_attempt_view)
            .collect(),
    })
}

const fn delegation_renewal_provisioner_view(
    provisioner: RootDelegationRenewalProvisioner,
) -> RootDelegationRenewalProvisionerView {
    RootDelegationRenewalProvisionerView {
        principal: provisioner.principal,
        enabled: provisioner.enabled,
    }
}

const fn root_issuer_renewal_state_view(
    state: &RootIssuerRenewalState,
) -> RootIssuerRenewalStateView {
    RootIssuerRenewalStateView {
        issuer_pid: state.issuer_pid,
        template_fingerprint: state.template_fingerprint,
        last_installed_cert_hash: state.last_installed_cert_hash,
        last_installed_expires_at_ns: state.last_installed_expires_at_ns,
        last_installed_refresh_after_ns: state.last_installed_refresh_after_ns,
        active_attempt_id: state.active_attempt_id,
        last_outcome: root_issuer_renewal_outcome_view(state.last_outcome),
        consecutive_failures: state.consecutive_failures,
        next_attempt_after_ns: state.next_attempt_after_ns,
        updated_at_ns: state.updated_at_ns,
    }
}

fn root_issuer_renewal_attempt_view(
    attempt: &RootIssuerRenewalAttempt,
) -> RootIssuerRenewalAttemptView {
    RootIssuerRenewalAttemptView {
        attempt_id: attempt.attempt_id,
        issuer_pid: attempt.issuer_pid,
        template_fingerprint: attempt.template_fingerprint,
        batch_id: attempt.batch_id,
        proof_ref: RootDelegationProofBatchProofRef {
            issuer_pid: attempt.proof_ref.issuer_pid,
            cert_hash: attempt.proof_ref.cert_hash,
        },
        status: root_issuer_renewal_attempt_status_view(attempt.status),
        prepared_at_ns: attempt.prepared_at_ns,
        retrieval_expires_at_ns: attempt.retrieval_expires_at_ns,
        install_deadline_ns: attempt.install_deadline_ns,
        prepared_cert_hash: attempt.prepared_cert_hash,
        prepared_expires_at_ns: attempt.prepared_expires_at_ns,
        prepared_refresh_after_ns: attempt.prepared_refresh_after_ns,
        failure: attempt.failure.map(root_issuer_renewal_outcome_view),
    }
}

const fn root_issuer_renewal_outcome_view(
    outcome: PolicyRenewalOutcome,
) -> RootIssuerRenewalOutcome {
    match outcome {
        PolicyRenewalOutcome::AlreadyInstalled => RootIssuerRenewalOutcome::AlreadyInstalled,
        PolicyRenewalOutcome::DriftDetected => RootIssuerRenewalOutcome::DriftDetected,
        PolicyRenewalOutcome::InstallDeadlineExpired => {
            RootIssuerRenewalOutcome::InstallDeadlineExpired
        }
        PolicyRenewalOutcome::Installed => RootIssuerRenewalOutcome::Installed,
        PolicyRenewalOutcome::IssuerCallFailed => RootIssuerRenewalOutcome::IssuerCallFailed,
        PolicyRenewalOutcome::NeverRun => RootIssuerRenewalOutcome::NeverRun,
        PolicyRenewalOutcome::PolicyRejected => RootIssuerRenewalOutcome::PolicyRejected,
        PolicyRenewalOutcome::ProofMismatch => RootIssuerRenewalOutcome::ProofMismatch,
        PolicyRenewalOutcome::QuotaExceeded => RootIssuerRenewalOutcome::QuotaExceeded,
        PolicyRenewalOutcome::RejectedByIssuer => RootIssuerRenewalOutcome::RejectedByIssuer,
        PolicyRenewalOutcome::RetrievalExpired => RootIssuerRenewalOutcome::RetrievalExpired,
        PolicyRenewalOutcome::TemplateChanged => RootIssuerRenewalOutcome::TemplateChanged,
        PolicyRenewalOutcome::TemplateDisabled => RootIssuerRenewalOutcome::TemplateDisabled,
    }
}

const fn root_issuer_renewal_attempt_status_view(
    status: PolicyRenewalAttemptStatus,
) -> RootIssuerRenewalAttemptStatus {
    match status {
        PolicyRenewalAttemptStatus::Prepared => RootIssuerRenewalAttemptStatus::Prepared,
        PolicyRenewalAttemptStatus::Installing => RootIssuerRenewalAttemptStatus::Installing,
        PolicyRenewalAttemptStatus::Installed => RootIssuerRenewalAttemptStatus::Installed,
        PolicyRenewalAttemptStatus::FailedRetryable => {
            RootIssuerRenewalAttemptStatus::FailedRetryable
        }
        PolicyRenewalAttemptStatus::FailedTerminal => {
            RootIssuerRenewalAttemptStatus::FailedTerminal
        }
        PolicyRenewalAttemptStatus::Disabled => RootIssuerRenewalAttemptStatus::Disabled,
        PolicyRenewalAttemptStatus::Expired => RootIssuerRenewalAttemptStatus::Expired,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        cdk::types::Principal,
        domain::policy::auth::{
            RootDelegatedRoleGrantPolicy, RootDelegationAudiencePolicy, RootDelegationRenewalBatch,
            RootIssuerPolicy, RootIssuerRenewalAttempt, RootIssuerRenewalProofRef,
        },
        dto::auth::{
            DelegatedRoleGrant, DelegationAudience, DelegationCert, DelegationProof,
            IcCanisterSignatureProofV1, IssuerProofAlgorithm, IssuerProofBinding,
            RootDelegationProofBatchEntry, RootDelegationProofBatchProof, RootProof,
        },
        dto::error::ErrorCode,
        ids::CanisterRole,
        ops::{
            runtime::metrics::delegated_auth::{
                DelegatedAuthMetricKey, DelegatedAuthMetricOperation,
            },
            storage::auth::AuthStateOps,
        },
    };

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn grant(scope: &str) -> DelegatedRoleGrant {
        DelegatedRoleGrant {
            target: CanisterRole::owned("project_instance".to_string()),
            scopes: vec![scope.to_string()],
        }
    }

    fn policy(issuer_pid: Principal) -> RootIssuerPolicy {
        RootIssuerPolicy {
            issuer_pid,
            enabled: true,
            allowed_audiences: vec![RootDelegationAudiencePolicy::Project("test".to_string())],
            allowed_grants: vec![RootDelegatedRoleGrantPolicy {
                target: CanisterRole::owned("project_instance".to_string()),
                scopes: vec!["canic.issue".to_string()],
            }],
            max_cert_ttl_ns: 120_000_000_000,
            refresh_after_ratio_bps: 8_000,
        }
    }

    fn upsert_request(issuer_pid: Principal) -> RootIssuerRenewalTemplateUpsertRequest {
        RootIssuerRenewalTemplateUpsertRequest {
            issuer_pid,
            enabled: true,
            aud: DelegationAudience::Project("test".to_string()),
            grants: vec![grant("canic.issue")],
            cert_ttl_ns: 60_000_000_000,
        }
    }

    fn renewal_attempt(
        attempt_id: [u8; 32],
        batch_id: [u8; 32],
        issuer_pid: Principal,
        cert_hash: [u8; 32],
    ) -> RootIssuerRenewalAttempt {
        RootIssuerRenewalAttempt {
            attempt_id,
            issuer_pid,
            template_fingerprint: [44; 32],
            batch_id,
            proof_ref: RootIssuerRenewalProofRef {
                issuer_pid,
                cert_hash,
            },
            status: PolicyRenewalAttemptStatus::Prepared,
            prepared_at_ns: 10,
            retrieval_expires_at_ns: 70,
            install_deadline_ns: 90,
            prepared_cert_hash: cert_hash,
            prepared_expires_at_ns: 200,
            prepared_refresh_after_ns: 160,
            failure: None,
        }
    }

    fn renewal_batch(batch_id: [u8; 32], attempt_ids: Vec<[u8; 32]>) -> RootDelegationRenewalBatch {
        RootDelegationRenewalBatch {
            batch_id,
            attempt_ids,
            prepared_at_ns: 10,
            retrieval_expires_at_ns: 70,
        }
    }

    fn proof_for(
        issuer_pid: Principal,
        cert_hash: [u8; 32],
        expires_at_ns: u64,
    ) -> RootDelegationProofBatchProof {
        RootDelegationProofBatchProof {
            issuer_pid,
            cert_hash,
            proof: DelegationProof {
                cert: DelegationCert {
                    root_pid: p(1),
                    issuer_pid,
                    issuer_proof_alg: IssuerProofAlgorithm::IcCanisterSignatureV1,
                    issuer_proof_binding_hash: [4; 32],
                    issuer_proof_binding: IssuerProofBinding::IcCanisterSignatureV1 {
                        seed_hash: [5; 32],
                    },
                    issued_at_ns: 10,
                    not_before_ns: 10,
                    expires_at_ns,
                    max_token_ttl_ns: 30,
                    aud: DelegationAudience::Project("test".to_string()),
                    grants: vec![grant("canic.issue")],
                },
                root_proof: RootProof::IcCanisterSignatureV1(IcCanisterSignatureProofV1 {
                    signature_cbor: vec![8; 64],
                    public_key_der: vec![9; 32],
                }),
            },
        }
    }

    fn schedule_install_attempt(
        issuer_pid: Principal,
        batch_id: [u8; 32],
        attempt_id: [u8; 32],
        cert_hash: [u8; 32],
    ) -> RootIssuerRenewalAttempt {
        let template = root_issuer_renewal_template_from_request(upsert_request(issuer_pid));
        AuthStateOps::upsert_root_issuer_renewal_template(template.clone());

        let mut attempt = renewal_attempt(attempt_id, batch_id, issuer_pid, cert_hash);
        attempt.template_fingerprint = renewal_template_fingerprint(&template);
        AuthStateOps::upsert_root_issuer_renewal_attempt(attempt.clone());
        AuthStateOps::upsert_root_delegation_renewal_batch(renewal_batch(
            batch_id,
            vec![attempt_id],
        ));
        AuthStateOps::upsert_root_issuer_renewal_state(RootIssuerRenewalState {
            issuer_pid,
            template_fingerprint: attempt.template_fingerprint,
            last_installed_cert_hash: None,
            last_installed_expires_at_ns: None,
            last_installed_refresh_after_ns: None,
            active_attempt_id: Some(attempt_id),
            last_outcome: PolicyRenewalOutcome::NeverRun,
            consecutive_failures: 0,
            next_attempt_after_ns: 0,
            updated_at_ns: 10,
        });

        attempt
    }

    fn renewal_attempt_metric_count(
        outcome: DelegatedAuthMetricOutcome,
        reason: DelegatedAuthMetricReason,
    ) -> u64 {
        let key = DelegatedAuthMetricKey {
            operation: DelegatedAuthMetricOperation::RenewalAttempt,
            outcome,
            reason,
        };
        DelegatedAuthMetrics::event_snapshot()
            .into_iter()
            .find_map(|(event_key, count)| (event_key == key).then_some(count))
            .unwrap_or(0)
    }

    fn fake_prepare_response(
        request: RootDelegationProofBatchPrepareRequest,
    ) -> RootDelegationProofBatchPrepareResponse {
        let batch_id = request
            .metadata
            .expect("renewal prepare must include replay metadata")
            .request_id;
        RootDelegationProofBatchPrepareResponse {
            batch_id,
            retrieval_expires_at_ns: 70,
            entries: request
                .entries
                .into_iter()
                .enumerate()
                .map(|(idx, entry)| RootDelegationProofBatchEntry {
                    issuer_pid: entry.issuer_pid,
                    cert_hash: [u8::try_from(idx + 1).expect("small test index"); 32],
                    expires_at_ns: 200,
                    refresh_after_ns: 160,
                })
                .collect(),
        }
    }

    #[test]
    fn upsert_root_issuer_renewal_template_accepts_registered_policy() {
        let issuer_pid = p(81);
        AuthStateOps::upsert_root_issuer_policy(policy(issuer_pid));

        let response = upsert_root_issuer_renewal_template(upsert_request(issuer_pid), 10)
            .expect("template should be accepted");

        assert_eq!(response.template.issuer_pid, issuer_pid);
        assert_eq!(response.template.grants, vec![grant("canic.issue")]);
        assert_eq!(
            root_issuer_renewal_status(RootIssuerRenewalStatusRequest { issuer_pid }).template,
            Some(response.template)
        );
    }

    #[test]
    fn upsert_root_issuer_renewal_template_rejects_policy_widening() {
        let issuer_pid = p(82);
        AuthStateOps::upsert_root_issuer_policy(policy(issuer_pid));
        let mut request = upsert_request(issuer_pid);
        request.grants = vec![grant("canic.admin")];

        assert!(upsert_root_issuer_renewal_template(request, 10).is_err());
    }

    #[test]
    fn disabled_root_issuer_renewal_template_can_be_staged_without_policy() {
        let issuer_pid = p(83);
        let mut request = upsert_request(issuer_pid);
        request.enabled = false;
        request.grants.clear();

        let response = upsert_root_issuer_renewal_template(request, 10)
            .expect("disabled template should not require an issuer policy");

        assert!(!response.template.enabled);
        assert_eq!(
            root_issuer_renewal_status(RootIssuerRenewalStatusRequest { issuer_pid }).template,
            Some(response.template)
        );
    }

    #[test]
    fn disabling_root_issuer_renewal_template_clears_active_attempt() {
        let issuer_pid = p(84);
        let batch_id = [84; 32];
        let attempt_id = [85; 32];
        AuthStateOps::upsert_root_issuer_policy(policy(issuer_pid));
        let active_attempt = schedule_install_attempt(issuer_pid, batch_id, attempt_id, [86; 32]);
        let mut request = upsert_request(issuer_pid);
        request.enabled = false;

        let response = upsert_root_issuer_renewal_template(request, 90)
            .expect("disabled template should be accepted");

        assert!(!response.template.enabled);
        let attempt = AuthStateOps::root_issuer_renewal_attempt(attempt_id)
            .expect("disabled attempt should remain observable");
        assert_eq!(attempt.status, PolicyRenewalAttemptStatus::Disabled);
        assert_eq!(
            attempt.failure,
            Some(PolicyRenewalOutcome::TemplateDisabled)
        );

        let state = AuthStateOps::root_issuer_renewal_state(issuer_pid)
            .expect("issuer renewal state should remain observable");
        assert_eq!(state.active_attempt_id, None);
        assert_eq!(state.last_outcome, PolicyRenewalOutcome::TemplateDisabled);
        assert_eq!(state.consecutive_failures, 0);
        assert_eq!(state.next_attempt_after_ns, 90);
        assert_eq!(state.updated_at_ns, 90);
        assert_ne!(
            state.template_fingerprint,
            active_attempt.template_fingerprint
        );
    }

    #[test]
    fn manual_delegation_install_success_updates_matching_renewal_state() {
        let issuer_pid = p(230);
        let cert_hash = [231; 32];
        AuthStateOps::upsert_root_issuer_policy(policy(issuer_pid));
        upsert_root_issuer_renewal_template(upsert_request(issuer_pid), 10)
            .expect("template should be accepted");
        let proof = proof_for(issuer_pid, cert_hash, 60_000_000_010);

        record_manual_delegation_renewal_install_outcome(
            &proof,
            RootDelegationProofInstallOutcome::Installed,
            20,
        );

        let state = AuthStateOps::root_issuer_renewal_state(issuer_pid)
            .expect("manual install should record renewal state");
        assert_eq!(state.last_installed_cert_hash, Some(cert_hash));
        assert_eq!(state.last_installed_expires_at_ns, Some(60_000_000_010));
        assert_eq!(state.last_installed_refresh_after_ns, Some(48_000_000_010));
        assert_eq!(state.active_attempt_id, None);
        assert_eq!(state.last_outcome, PolicyRenewalOutcome::Installed);
        assert_eq!(state.consecutive_failures, 0);
        assert_eq!(state.next_attempt_after_ns, 48_000_000_010);
        assert_eq!(state.updated_at_ns, 20);
    }

    #[test]
    fn root_issuer_renewal_status_reports_root_owned_state() {
        let issuer_pid = p(87);
        let state = RootIssuerRenewalState {
            issuer_pid,
            template_fingerprint: [1; 32],
            last_installed_cert_hash: Some([2; 32]),
            last_installed_expires_at_ns: Some(200),
            last_installed_refresh_after_ns: Some(160),
            active_attempt_id: Some([3; 32]),
            last_outcome: PolicyRenewalOutcome::RetrievalExpired,
            consecutive_failures: 2,
            next_attempt_after_ns: 90,
            updated_at_ns: 80,
        };
        AuthStateOps::upsert_root_issuer_renewal_attempt(renewal_attempt(
            [3; 32], [4; 32], issuer_pid, [5; 32],
        ));
        AuthStateOps::upsert_root_issuer_renewal_state(state);

        let status = root_issuer_renewal_status(RootIssuerRenewalStatusRequest { issuer_pid });

        assert_eq!(status.template, None);
        assert_eq!(
            status
                .state
                .as_ref()
                .map(|state| state.last_outcome.clone()),
            Some(RootIssuerRenewalOutcome::RetrievalExpired)
        );
        assert_eq!(
            status
                .active_attempt
                .as_ref()
                .map(|attempt| attempt.batch_id),
            Some([4; 32])
        );
    }

    #[test]
    fn renewal_proof_batch_get_uses_scheduled_refs_only() {
        let batch_id = [90; 32];
        let first_attempt_id = [91; 32];
        let second_attempt_id = [92; 32];
        let first_issuer = p(91);
        let second_issuer = p(92);
        AuthStateOps::upsert_root_issuer_renewal_attempt(renewal_attempt(
            first_attempt_id,
            batch_id,
            first_issuer,
            [93; 32],
        ));
        AuthStateOps::upsert_root_issuer_renewal_attempt(renewal_attempt(
            second_attempt_id,
            batch_id,
            second_issuer,
            [94; 32],
        ));
        AuthStateOps::upsert_root_delegation_renewal_batch(renewal_batch(
            batch_id,
            vec![first_attempt_id, second_attempt_id],
        ));

        let response = get_delegation_renewal_proof_batch_with_getter(
            RootDelegationRenewalProofBatchGetRequest { batch_id },
            20,
            |request| {
                assert_eq!(request.batch_id, batch_id);
                assert_eq!(
                    request.entries,
                    vec![
                        RootDelegationProofBatchProofRef {
                            issuer_pid: first_issuer,
                            cert_hash: [93; 32],
                        },
                        RootDelegationProofBatchProofRef {
                            issuer_pid: second_issuer,
                            cert_hash: [94; 32],
                        },
                    ]
                );
                Ok(RootDelegationProofBatchGetResponse {
                    batch_id,
                    proofs: Vec::new(),
                })
            },
        )
        .expect("scheduled renewal batch should retrieve through resolved refs");

        assert_eq!(response.batch_id, batch_id);
    }

    #[test]
    fn renewal_proof_batch_get_rejects_expired_or_nonprepared_attempts() {
        let batch_id = [95; 32];
        let attempt_id = [96; 32];
        let mut attempt = renewal_attempt(attempt_id, batch_id, p(95), [97; 32]);
        attempt.status = PolicyRenewalAttemptStatus::Installing;
        AuthStateOps::upsert_root_issuer_renewal_attempt(attempt);
        AuthStateOps::upsert_root_delegation_renewal_batch(renewal_batch(
            batch_id,
            vec![attempt_id],
        ));

        let err = get_delegation_renewal_proof_batch_with_getter(
            RootDelegationRenewalProofBatchGetRequest { batch_id },
            20,
            |_| panic!("nonprepared attempt must not call generic proof retrieval"),
        )
        .expect_err("nonprepared scheduled attempt should reject");
        assert!(err.to_string().contains("not prepared"));

        let expired_batch_id = [98; 32];
        let expired_attempt_id = [99; 32];
        AuthStateOps::upsert_root_issuer_renewal_attempt(renewal_attempt(
            expired_attempt_id,
            expired_batch_id,
            p(96),
            [100; 32],
        ));
        AuthStateOps::upsert_root_delegation_renewal_batch(renewal_batch(
            expired_batch_id,
            vec![expired_attempt_id],
        ));

        let err = get_delegation_renewal_proof_batch_with_getter(
            RootDelegationRenewalProofBatchGetRequest {
                batch_id: expired_batch_id,
            },
            70,
            |_| panic!("expired batch must not call generic proof retrieval"),
        )
        .expect_err("expired scheduled batch should reject");
        assert!(err.to_string().contains("expired"));
    }

    #[test]
    fn renewal_batch_install_gate_accepts_live_scheduled_work() {
        let issuer_pid = p(144);
        let batch_id = [144; 32];
        let attempt_id = [145; 32];
        schedule_install_attempt(issuer_pid, batch_id, attempt_id, [146; 32]);

        ensure_delegation_renewal_batch_scheduled(batch_id, 30)
            .expect("live scheduled renewal work should pass provisioner gate");

        assert_eq!(
            AuthStateOps::root_issuer_renewal_attempt(attempt_id)
                .expect("attempt should remain stored")
                .status,
            PolicyRenewalAttemptStatus::Prepared
        );
        assert_eq!(
            AuthStateOps::root_issuer_renewal_state(issuer_pid)
                .expect("state should remain stored")
                .active_attempt_id,
            Some(attempt_id)
        );
    }

    #[test]
    fn renewal_batch_install_gate_expires_late_scheduled_work() {
        let issuer_pid = p(147);
        let batch_id = [147; 32];
        let attempt_id = [148; 32];
        schedule_install_attempt(issuer_pid, batch_id, attempt_id, [149; 32]);

        let mut attempt = AuthStateOps::root_issuer_renewal_attempt(attempt_id)
            .expect("attempt should be scheduled");
        attempt.retrieval_expires_at_ns = 40;
        attempt.install_deadline_ns = 40;
        AuthStateOps::upsert_root_issuer_renewal_attempt(attempt);
        let mut batch =
            AuthStateOps::root_delegation_renewal_batch(batch_id).expect("batch should be stored");
        batch.retrieval_expires_at_ns = 40;
        AuthStateOps::upsert_root_delegation_renewal_batch(batch);

        let err = ensure_delegation_renewal_batch_scheduled(batch_id, 40)
            .expect_err("expired scheduled work should fail provisioner gate");

        assert!(err.to_string().contains("install deadline expired"));
        let attempt = AuthStateOps::root_issuer_renewal_attempt(attempt_id)
            .expect("attempt should remain visible");
        assert_eq!(attempt.status, PolicyRenewalAttemptStatus::Expired);
        assert_eq!(
            attempt.failure,
            Some(PolicyRenewalOutcome::InstallDeadlineExpired)
        );

        let state = AuthStateOps::root_issuer_renewal_state(issuer_pid)
            .expect("issuer renewal state should remain visible");
        assert_eq!(state.active_attempt_id, None);
        assert_eq!(
            state.last_outcome,
            PolicyRenewalOutcome::InstallDeadlineExpired
        );
        assert_eq!(state.consecutive_failures, 1);
        assert_eq!(state.next_attempt_after_ns, 40);
        assert_eq!(AuthStateOps::root_delegation_renewal_batch(batch_id), None);
    }

    #[test]
    fn delegation_renewal_work_lists_retrievable_batches_only() {
        let valid_batch_id = [210; 32];
        let valid_attempt_id = [211; 32];
        let skipped_batch_id = [212; 32];
        let skipped_attempt_id = [213; 32];
        let expired_batch_id = [214; 32];
        let expired_attempt_id = [215; 32];

        AuthStateOps::upsert_root_issuer_renewal_attempt(renewal_attempt(
            valid_attempt_id,
            valid_batch_id,
            p(210),
            [216; 32],
        ));
        AuthStateOps::upsert_root_delegation_renewal_batch(renewal_batch(
            valid_batch_id,
            vec![valid_attempt_id],
        ));

        let mut skipped_attempt =
            renewal_attempt(skipped_attempt_id, skipped_batch_id, p(211), [217; 32]);
        skipped_attempt.status = PolicyRenewalAttemptStatus::Installing;
        AuthStateOps::upsert_root_issuer_renewal_attempt(skipped_attempt);
        AuthStateOps::upsert_root_delegation_renewal_batch(renewal_batch(
            skipped_batch_id,
            vec![skipped_attempt_id],
        ));

        AuthStateOps::upsert_root_issuer_renewal_attempt(renewal_attempt(
            expired_attempt_id,
            expired_batch_id,
            p(212),
            [218; 32],
        ));
        let mut expired_batch = renewal_batch(expired_batch_id, vec![expired_attempt_id]);
        expired_batch.retrieval_expires_at_ns = 15;
        AuthStateOps::upsert_root_delegation_renewal_batch(expired_batch);

        let work = delegation_renewal_work(20);

        let valid_batch = work
            .batches
            .iter()
            .find(|batch| batch.batch_id == valid_batch_id)
            .expect("valid scheduled batch should be advertised");
        assert_eq!(valid_batch.attempt_count, 1);
        assert_eq!(valid_batch.attempts.len(), 1);
        assert_eq!(valid_batch.attempts[0].attempt_id, valid_attempt_id);
        assert_eq!(
            valid_batch.attempts[0].status,
            RootIssuerRenewalAttemptStatus::Prepared
        );
        assert!(
            work.batches
                .iter()
                .all(|batch| batch.batch_id != skipped_batch_id)
        );
        assert!(
            work.batches
                .iter()
                .all(|batch| batch.batch_id != expired_batch_id)
        );
    }

    #[test]
    fn prepare_due_delegation_renewals_schedules_initial_enabled_templates() {
        DelegatedAuthMetrics::reset();

        let first_issuer = p(101);
        let second_issuer = p(100);
        AuthStateOps::upsert_root_issuer_policy(policy(first_issuer));
        AuthStateOps::upsert_root_issuer_policy(policy(second_issuer));
        upsert_root_issuer_renewal_template(upsert_request(first_issuer), 10)
            .expect("first template should be accepted");
        upsert_root_issuer_renewal_template(upsert_request(second_issuer), 10)
            .expect("second template should be accepted");

        let result = prepare_due_delegation_renewals_with_prepare(120_000_000_000, 10, |request| {
            assert!(
                request
                    .entries
                    .iter()
                    .any(|entry| entry.issuer_pid == first_issuer)
            );
            assert!(
                request
                    .entries
                    .iter()
                    .any(|entry| entry.issuer_pid == second_issuer)
            );
            Ok(fake_prepare_response(request))
        })
        .expect("due renewal templates should prepare");

        let batch_id = result
            .prepared_batch_id
            .expect("initial enabled templates should create a batch");
        let batch = AuthStateOps::root_delegation_renewal_batch(batch_id)
            .expect("scheduler should persist renewal batch");

        assert!(result.prepared_attempts >= 2);
        assert_eq!(
            renewal_attempt_metric_count(
                DelegatedAuthMetricOutcome::Started,
                DelegatedAuthMetricReason::Ok,
            ),
            u64::try_from(result.prepared_attempts).expect("prepared attempt count should fit u64")
        );
        assert!(batch.attempt_ids.len() >= 2);
        for issuer_pid in [first_issuer, second_issuer] {
            let state = AuthStateOps::root_issuer_renewal_state(issuer_pid)
                .expect("scheduler should update issuer renewal state");
            let attempt = AuthStateOps::root_issuer_renewal_attempt(
                state
                    .active_attempt_id
                    .expect("scheduler should set active attempt id"),
            )
            .expect("scheduler should persist active attempt");
            assert_eq!(attempt.issuer_pid, issuer_pid);
            assert_eq!(attempt.status, PolicyRenewalAttemptStatus::Prepared);
        }
    }

    #[test]
    fn prepare_due_delegation_renewals_records_quota_prepare_failure() {
        let issuer_pid = p(141);
        AuthStateOps::upsert_root_issuer_policy(policy(issuer_pid));
        upsert_root_issuer_renewal_template(upsert_request(issuer_pid), 10)
            .expect("template should be accepted");
        let template = root_issuer_renewal_template_from_request(upsert_request(issuer_pid));
        AuthStateOps::upsert_root_issuer_renewal_state(RootIssuerRenewalState {
            issuer_pid,
            template_fingerprint: renewal_template_fingerprint(&template),
            last_installed_cert_hash: Some([142; 32]),
            last_installed_expires_at_ns: Some(1_000),
            last_installed_refresh_after_ns: Some(15),
            active_attempt_id: None,
            last_outcome: PolicyRenewalOutcome::Installed,
            consecutive_failures: 2,
            next_attempt_after_ns: 0,
            updated_at_ns: 12,
        });

        let err = prepare_due_delegation_renewals_with_prepare(120_000_000_000, 20, |_| {
            Err(InternalError::resource_exhausted(
                "pending renewal quota exhausted",
            ))
        })
        .expect_err("prepare quota failure should propagate");

        assert_eq!(
            err.public_error().map(|public| public.code),
            Some(ErrorCode::ResourceExhausted)
        );
        let state = AuthStateOps::root_issuer_renewal_state(issuer_pid)
            .expect("quota failure should update issuer renewal state");
        assert_eq!(state.active_attempt_id, None);
        assert_eq!(state.last_installed_cert_hash, Some([142; 32]));
        assert_eq!(state.last_installed_expires_at_ns, Some(1_000));
        assert_eq!(state.last_installed_refresh_after_ns, Some(15));
        assert_eq!(state.last_outcome, PolicyRenewalOutcome::QuotaExceeded);
        assert_eq!(state.consecutive_failures, 3);
        assert_eq!(state.next_attempt_after_ns, 60_000_000_020);
        assert_eq!(state.updated_at_ns, 20);
    }

    #[test]
    fn prepare_due_delegation_renewals_records_policy_prepare_failure() {
        let issuer_pid = p(143);
        AuthStateOps::upsert_root_issuer_policy(policy(issuer_pid));
        upsert_root_issuer_renewal_template(upsert_request(issuer_pid), 10)
            .expect("template should be accepted");

        let err = prepare_due_delegation_renewals_with_prepare(120_000_000_000, 30, |_| {
            Err(InternalError::forbidden(
                "root issuer policy rejected renewal",
            ))
        })
        .expect_err("policy failure should propagate");

        assert_eq!(
            err.public_error().map(|public| public.code),
            Some(ErrorCode::Forbidden)
        );
        let state = AuthStateOps::root_issuer_renewal_state(issuer_pid)
            .expect("policy failure should update issuer renewal state");
        assert_eq!(state.active_attempt_id, None);
        assert_eq!(state.last_outcome, PolicyRenewalOutcome::PolicyRejected);
        assert_eq!(state.consecutive_failures, 1);
        assert_eq!(state.next_attempt_after_ns, 60_000_000_030);
        assert_eq!(state.updated_at_ns, 30);
    }

    #[test]
    fn prepare_due_delegation_renewals_skips_fresh_or_active_attempts() {
        let fresh_issuer = p(102);
        let active_issuer = p(103);
        let fresh_template =
            root_issuer_renewal_template_from_request(upsert_request(fresh_issuer));
        let active_template =
            root_issuer_renewal_template_from_request(upsert_request(active_issuer));

        let fresh_state = RootIssuerRenewalState {
            issuer_pid: fresh_issuer,
            template_fingerprint: renewal_template_fingerprint(&fresh_template),
            last_installed_cert_hash: Some([1; 32]),
            last_installed_expires_at_ns: Some(1_000),
            last_installed_refresh_after_ns: Some(900),
            active_attempt_id: None,
            last_outcome: PolicyRenewalOutcome::Installed,
            consecutive_failures: 0,
            next_attempt_after_ns: 0,
            updated_at_ns: 20,
        };

        let active_attempt_id = [104; 32];
        let mut active_attempt =
            renewal_attempt(active_attempt_id, [105; 32], active_issuer, [106; 32]);
        active_attempt.install_deadline_ns = 200;
        AuthStateOps::upsert_root_issuer_renewal_attempt(active_attempt);
        let active_state = RootIssuerRenewalState {
            issuer_pid: active_issuer,
            template_fingerprint: renewal_template_fingerprint(&active_template),
            last_installed_cert_hash: Some([2; 32]),
            last_installed_expires_at_ns: Some(200),
            last_installed_refresh_after_ns: Some(100),
            active_attempt_id: Some(active_attempt_id),
            last_outcome: PolicyRenewalOutcome::Installed,
            consecutive_failures: 0,
            next_attempt_after_ns: 0,
            updated_at_ns: 20,
        };

        assert!(!renewal_template_due(
            30,
            renewal_template_fingerprint(&fresh_template),
            Some(&fresh_state)
        ));
        assert!(!renewal_template_due(
            150,
            renewal_template_fingerprint(&active_template),
            Some(&active_state)
        ));
    }

    #[test]
    fn prepare_due_delegation_renewals_expires_stale_active_attempts() {
        let issuer_pid = p(136);
        let stale_attempt_id = [137; 32];
        let stale_batch_id = [138; 32];
        let stale_cert_hash = [139; 32];
        AuthStateOps::upsert_root_issuer_policy(policy(issuer_pid));
        upsert_root_issuer_renewal_template(upsert_request(issuer_pid), 10)
            .expect("template should be accepted");
        let template = root_issuer_renewal_template_from_request(upsert_request(issuer_pid));

        let mut stale_attempt = renewal_attempt(
            stale_attempt_id,
            stale_batch_id,
            issuer_pid,
            stale_cert_hash,
        );
        stale_attempt.template_fingerprint = renewal_template_fingerprint(&template);
        stale_attempt.retrieval_expires_at_ns = 40;
        stale_attempt.install_deadline_ns = 40;
        AuthStateOps::upsert_root_issuer_renewal_attempt(stale_attempt);
        AuthStateOps::upsert_root_delegation_renewal_batch(renewal_batch(
            stale_batch_id,
            vec![stale_attempt_id],
        ));
        AuthStateOps::upsert_root_issuer_renewal_state(RootIssuerRenewalState {
            issuer_pid,
            template_fingerprint: renewal_template_fingerprint(&template),
            last_installed_cert_hash: Some([140; 32]),
            last_installed_expires_at_ns: Some(100),
            last_installed_refresh_after_ns: Some(30),
            active_attempt_id: Some(stale_attempt_id),
            last_outcome: PolicyRenewalOutcome::Installed,
            consecutive_failures: 0,
            next_attempt_after_ns: 0,
            updated_at_ns: 20,
        });

        let result = prepare_due_delegation_renewals_with_prepare(120_000_000_000, 80, |request| {
            assert!(
                request
                    .entries
                    .iter()
                    .any(|entry| entry.issuer_pid == issuer_pid)
            );
            Ok(fake_prepare_response(request))
        })
        .expect("expired active attempt should allow fresh renewal prepare");

        assert!(result.prepared_batch_id.is_some());
        let stale_attempt = AuthStateOps::root_issuer_renewal_attempt(stale_attempt_id)
            .expect("stale attempt should remain stored");
        assert_eq!(stale_attempt.status, PolicyRenewalAttemptStatus::Expired);
        assert_eq!(
            stale_attempt.failure,
            Some(PolicyRenewalOutcome::RetrievalExpired)
        );
        assert_eq!(
            AuthStateOps::root_delegation_renewal_batch(stale_batch_id),
            None
        );

        let state = AuthStateOps::root_issuer_renewal_state(issuer_pid)
            .expect("issuer state should remain stored");
        assert_ne!(state.active_attempt_id, Some(stale_attempt_id));
        assert_eq!(state.last_outcome, PolicyRenewalOutcome::RetrievalExpired);
        assert_eq!(state.consecutive_failures, 1);
    }

    #[test]
    fn scheduled_renewal_install_preflight_marks_attempt_installing() {
        let issuer_pid = p(120);
        let batch_id = [121; 32];
        let attempt_id = [122; 32];
        let cert_hash = [123; 32];
        schedule_install_attempt(issuer_pid, batch_id, attempt_id, cert_hash);

        let scheduled_attempt_id = preflight_delegation_renewal_proof_install(
            batch_id,
            &proof_for(issuer_pid, cert_hash, 200),
            30,
        )
        .expect("scheduled renewal install should preflight");

        assert_eq!(scheduled_attempt_id, Some(attempt_id));
        assert_eq!(
            AuthStateOps::root_issuer_renewal_attempt(attempt_id)
                .expect("attempt should remain stored")
                .status,
            PolicyRenewalAttemptStatus::Installing
        );
        assert_eq!(
            AuthStateOps::root_issuer_renewal_state(issuer_pid)
                .expect("state should remain stored")
                .active_attempt_id,
            Some(attempt_id)
        );
    }

    #[test]
    fn scheduled_renewal_install_success_updates_issuer_state() {
        let issuer_pid = p(124);
        let batch_id = [125; 32];
        let attempt_id = [126; 32];
        let cert_hash = [127; 32];
        let mut attempt = schedule_install_attempt(issuer_pid, batch_id, attempt_id, cert_hash);
        attempt.status = PolicyRenewalAttemptStatus::Installing;
        AuthStateOps::upsert_root_issuer_renewal_attempt(attempt);

        record_delegation_renewal_install_outcome(
            attempt_id,
            RootDelegationProofInstallOutcome::Installed,
            40,
        );

        assert_eq!(
            AuthStateOps::root_issuer_renewal_attempt(attempt_id)
                .expect("attempt should remain stored")
                .status,
            PolicyRenewalAttemptStatus::Installed
        );
        let state = AuthStateOps::root_issuer_renewal_state(issuer_pid)
            .expect("issuer renewal state should update");
        assert_eq!(state.active_attempt_id, None);
        assert_eq!(state.last_installed_cert_hash, Some(cert_hash));
        assert_eq!(state.last_installed_expires_at_ns, Some(200));
        assert_eq!(state.last_installed_refresh_after_ns, Some(160));
        assert_eq!(state.last_outcome, PolicyRenewalOutcome::Installed);
        assert_eq!(state.consecutive_failures, 0);
    }

    #[test]
    fn scheduled_renewal_install_call_failure_remains_retryable() {
        DelegatedAuthMetrics::reset();

        let issuer_pid = p(128);
        let batch_id = [129; 32];
        let attempt_id = [130; 32];
        let cert_hash = [131; 32];
        let mut attempt = schedule_install_attempt(issuer_pid, batch_id, attempt_id, cert_hash);
        attempt.status = PolicyRenewalAttemptStatus::Installing;
        AuthStateOps::upsert_root_issuer_renewal_attempt(attempt);

        record_delegation_renewal_install_outcome(
            attempt_id,
            RootDelegationProofInstallOutcome::CallFailed,
            40,
        );

        let attempt = AuthStateOps::root_issuer_renewal_attempt(attempt_id)
            .expect("attempt should remain stored");
        assert_eq!(attempt.status, PolicyRenewalAttemptStatus::FailedRetryable);
        assert_eq!(
            attempt.failure,
            Some(PolicyRenewalOutcome::IssuerCallFailed)
        );

        let state = AuthStateOps::root_issuer_renewal_state(issuer_pid)
            .expect("issuer renewal state should update");
        assert_eq!(state.active_attempt_id, Some(attempt_id));
        assert_eq!(state.last_outcome, PolicyRenewalOutcome::IssuerCallFailed);
        assert_eq!(state.consecutive_failures, 1);
        assert_eq!(state.next_attempt_after_ns, 90);
        assert_eq!(
            renewal_attempt_metric_count(
                DelegatedAuthMetricOutcome::Failed,
                DelegatedAuthMetricReason::RetryScheduled,
            ),
            1
        );
    }

    #[test]
    fn scheduled_renewal_install_rejects_changed_template() {
        let issuer_pid = p(132);
        let batch_id = [133; 32];
        let attempt_id = [134; 32];
        let cert_hash = [135; 32];
        schedule_install_attempt(issuer_pid, batch_id, attempt_id, cert_hash);

        let mut changed_template =
            root_issuer_renewal_template_from_request(upsert_request(issuer_pid));
        changed_template.cert_ttl_ns += 1;
        AuthStateOps::upsert_root_issuer_renewal_template(changed_template);

        let outcome = preflight_delegation_renewal_proof_install(
            batch_id,
            &proof_for(issuer_pid, cert_hash, 200),
            30,
        )
        .expect_err("changed template should reject scheduled install");

        assert_eq!(
            outcome,
            RootDelegationProofInstallOutcome::ExpiredOrSuperseded
        );
        let attempt = AuthStateOps::root_issuer_renewal_attempt(attempt_id)
            .expect("attempt should remain stored");
        assert_eq!(attempt.status, PolicyRenewalAttemptStatus::FailedTerminal);
        assert_eq!(attempt.failure, Some(PolicyRenewalOutcome::TemplateChanged));

        let state = AuthStateOps::root_issuer_renewal_state(issuer_pid)
            .expect("issuer renewal state should update");
        assert_eq!(state.active_attempt_id, None);
        assert_eq!(state.last_outcome, PolicyRenewalOutcome::TemplateChanged);
    }
}
