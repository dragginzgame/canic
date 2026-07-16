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
    model::auth::RootIssuerRenewalTemplate,
    ops::storage::auth::{
        AuthStateOps, ChainKeyRootDelegationBatch, ChainKeyRootDelegationBatchIssuer,
    },
};

pub(in crate::ops::auth::delegation) use identity::renewal_template_fingerprint;
use view::{
    root_issuer_renewal_batch_view, root_issuer_renewal_state_view,
    root_issuer_renewal_template_view,
};

pub(super) fn commit_root_issuer_renewal_template(
    template: RootIssuerRenewalTemplate,
    now_ns: u64,
) -> RootIssuerRenewalTemplateResponse {
    AuthStateOps::upsert_root_issuer_renewal_template(template.clone());
    AuthStateOps::advance_delegated_auth_registry_epoch();
    if !template.enabled {
        record_disabled_renewal_template(&template, now_ns);
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
    let latest_batch = latest_issuer_renewal_batch(request.issuer_pid)
        .map(|(batch, issuer)| root_issuer_renewal_batch_view(&batch, &issuer));

    RootIssuerRenewalStatusResponse {
        template: AuthStateOps::root_issuer_renewal_template(request.issuer_pid)
            .map(|template| root_issuer_renewal_template_view(&template)),
        state: state.map(|state| root_issuer_renewal_state_view(&state)),
        latest_batch,
    }
}

fn latest_issuer_renewal_batch(
    issuer_pid: crate::cdk::types::Principal,
) -> Option<(
    ChainKeyRootDelegationBatch,
    ChainKeyRootDelegationBatchIssuer,
)> {
    AuthStateOps::chain_key_root_delegation_batches()
        .into_iter()
        .filter_map(|batch| {
            let issuer = batch
                .issuers
                .iter()
                .find(|issuer| issuer.issuer_pid == issuer_pid)?
                .clone();
            Some((batch, issuer))
        })
        .max_by(|(left, _), (right, _)| {
            left.header
                .proof_epoch
                .cmp(&right.header.proof_epoch)
                .then_with(|| left.prepared_at_ns.cmp(&right.prepared_at_ns))
                .then_with(|| left.batch_id.cmp(&right.batch_id))
        })
}

fn record_disabled_renewal_template(template: &RootIssuerRenewalTemplate, now_ns: u64) {
    let Some(mut state) = AuthStateOps::root_issuer_renewal_state(template.issuer_pid) else {
        return;
    };

    state.template_fingerprint = renewal_template_fingerprint(template);
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
