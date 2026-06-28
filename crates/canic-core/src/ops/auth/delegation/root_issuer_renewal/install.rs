//! Module: ops::auth::delegation::root_issuer_renewal::install
//!
//! Responsibility: validate and record root-managed renewal proof install outcomes.
//! Does not own: renewal scheduling, proof retrieval, or DTO view conversion.

use super::identity::renewal_template_fingerprint;
use crate::{
    cdk::types::Principal,
    domain::policy::auth::{
        RootDelegationProofPreparePolicyDecision, RootDelegationProofPreparePolicyInput,
        RootDelegationRenewalBatch, RootIssuerRenewalAttempt,
        RootIssuerRenewalAttemptStatus as PolicyRenewalAttemptStatus,
        RootIssuerRenewalOutcome as PolicyRenewalOutcome, RootIssuerRenewalState,
        RootIssuerRenewalTemplate, validate_root_delegation_proof_prepare_policy,
    },
    dto::auth::{RootDelegationProofBatchProof, RootDelegationProofInstallOutcome},
    log::Topic,
    ops::{
        auth::delegation::root_issuer_policy::{audience_policy, grant_policies},
        runtime::metrics::delegated_auth::{
            DelegatedAuthMetricOutcome, DelegatedAuthMetricReason, DelegatedAuthMetrics,
        },
        storage::auth::AuthStateOps,
    },
};

const ROOT_DELEGATION_RENEWAL_RETRY_BACKOFF_NS: u64 = 60_000_000_000;

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

pub(super) fn record_scheduled_renewal_attempt_failure(
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

pub(super) fn retry_after_ns(now_ns: u64, install_deadline_ns: u64) -> u64 {
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

pub(super) const fn delegated_auth_reason_from_renewal_attempt_outcome(
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
