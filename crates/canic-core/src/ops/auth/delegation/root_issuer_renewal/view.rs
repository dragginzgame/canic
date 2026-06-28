//! Module: ops::auth::delegation::root_issuer_renewal::view
//!
//! Responsibility: project root issuer renewal policy records into boundary views.
//! Does not own: storage mutation, scheduling decisions, or install outcome handling.

use super::retrieval::scheduled_renewal_batch_attempts;
use crate::{
    domain::policy::auth::{
        RootDelegationRenewalBatch, RootIssuerRenewalAttempt,
        RootIssuerRenewalAttemptStatus as PolicyRenewalAttemptStatus,
        RootIssuerRenewalOutcome as PolicyRenewalOutcome, RootIssuerRenewalState,
        RootIssuerRenewalTemplate,
    },
    dto::auth::{
        RootDelegationProofBatchProofRef, RootDelegationRenewalBatchView,
        RootDelegationRenewalProvisionerView, RootIssuerRenewalAttemptStatus,
        RootIssuerRenewalAttemptView, RootIssuerRenewalOutcome, RootIssuerRenewalStateView,
        RootIssuerRenewalTemplateView,
    },
    ops::{
        auth::delegation::root_issuer_policy::{
            delegated_role_grant_views, delegation_audience_view,
        },
        storage::auth::RootDelegationRenewalProvisioner,
    },
};

pub(super) fn root_issuer_renewal_template_view(
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

pub(super) fn root_delegation_renewal_batch_view(
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

pub(super) const fn delegation_renewal_provisioner_view(
    provisioner: RootDelegationRenewalProvisioner,
) -> RootDelegationRenewalProvisionerView {
    RootDelegationRenewalProvisionerView {
        principal: provisioner.principal,
        enabled: provisioner.enabled,
    }
}

pub(super) const fn root_issuer_renewal_state_view(
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

pub(super) fn root_issuer_renewal_attempt_view(
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
