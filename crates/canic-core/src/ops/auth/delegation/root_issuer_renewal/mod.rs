//! Module: ops::auth::delegation::root_issuer_renewal
//!
//! Responsibility: map and validate root-managed issuer renewal boundary DTOs.
//! Does not own: renewal scheduling, proof retrieval, or issuer install calls.

mod identity;
mod install;
mod retrieval;
mod schedule;
#[cfg(test)]
mod tests;
mod view;

use super::{
    errors::map_root_provisioning_policy_error,
    root_issuer_policy::{audience_policy, grant_policies},
};
use crate::{
    InternalError,
    cdk::types::Principal,
    domain::policy::auth::{
        RootIssuerRenewalAttemptStatus as PolicyRenewalAttemptStatus,
        RootIssuerRenewalOutcome as PolicyRenewalOutcome, RootIssuerRenewalTemplate,
        validate_root_issuer_renewal_template_policy,
    },
    dto::auth::{
        RootDelegationProofBatchProof, RootDelegationProofInstallOutcome,
        RootDelegationRenewalProvisionerListResponse, RootDelegationRenewalProvisionerResponse,
        RootDelegationRenewalProvisionerUpsertRequest, RootDelegationRenewalWorkListResponse,
        RootIssuerRenewalStatusRequest, RootIssuerRenewalStatusResponse,
        RootIssuerRenewalTemplateResponse, RootIssuerRenewalTemplateUpsertRequest,
    },
    log::Topic,
    ops::{
        runtime::metrics::delegated_auth::{
            DelegatedAuthMetricOutcome, DelegatedAuthMetricReason, DelegatedAuthMetrics,
        },
        storage::auth::{AuthStateOps, RootDelegationRenewalProvisioner},
    },
};

use identity::renewal_template_fingerprint;
#[cfg(test)]
use retrieval::get_delegation_renewal_proof_batch_with_getter;
pub(super) use retrieval::{
    ensure_delegation_renewal_batch_scheduled, get_delegation_renewal_proof_batch,
};
pub(super) use schedule::prepare_due_delegation_renewals;
#[cfg(test)]
use schedule::{prepare_due_delegation_renewals_with_prepare, renewal_template_due};
use view::{
    delegation_renewal_provisioner_view, root_delegation_renewal_batch_view,
    root_issuer_renewal_attempt_view, root_issuer_renewal_state_view,
    root_issuer_renewal_template_view,
};

const ROOT_DELEGATION_RENEWAL_RETRY_BACKOFF_NS: u64 = 60_000_000_000;

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
