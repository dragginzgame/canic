//! Module: ops::auth::delegation::root_issuer_renewal
//!
//! Responsibility: convert and persist root-managed issuer renewal state.
//! Does not own: admission policy, scheduling, signing, proof retrieval, or issuer install calls.

mod identity;
#[cfg(test)]
mod tests;
mod view;

use super::root_issuer_policy::{audience_policy, grant_policies};
use crate::{
    dto::auth::{
        RootIssuerRenewalStatusRequest, RootIssuerRenewalStatusResponse,
        RootIssuerRenewalTemplateResponse, RootIssuerRenewalTemplateUpsertRequest,
    },
    log::Topic,
    model::auth::{
        RootIssuerRenewalAttemptStatus as PolicyRenewalAttemptStatus,
        RootIssuerRenewalOutcome as PolicyRenewalOutcome, RootIssuerRenewalTemplate,
    },
    ops::{
        runtime::metrics::delegated_auth::{
            DelegatedAuthMetricOutcome, DelegatedAuthMetricReason, DelegatedAuthMetrics,
        },
        storage::auth::AuthStateOps,
    },
};

pub(in crate::ops::auth::delegation) use identity::renewal_template_fingerprint;
use view::{
    root_issuer_renewal_attempt_view, root_issuer_renewal_state_view,
    root_issuer_renewal_template_view,
};

pub(super) fn commit_root_issuer_renewal_template(
    template: RootIssuerRenewalTemplate,
    now_ns: u64,
) -> RootIssuerRenewalTemplateResponse {
    AuthStateOps::upsert_root_issuer_renewal_template(template.clone());
    AuthStateOps::advance_delegated_auth_registry_epoch();
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

    RootIssuerRenewalTemplateResponse {
        template: root_issuer_renewal_template_view(&template),
    }
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

pub(super) fn root_issuer_renewal_template_from_request(
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
