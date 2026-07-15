//! Module: ops::auth::delegation::chain_key_batch::selection
//!
//! Responsibility: select due issuer renewal templates for one chain-key batch.
//! Does not own: proof materialization, signing, or issuer install retry state.
//! Boundary: private helper for root-local chain-key batch preparation.

use super::{
    MAX_CHAIN_KEY_ROOT_DELEGATION_BATCH_ISSUERS, MAX_PENDING_CHAIN_KEY_ROOT_DELEGATION_BATCHES,
};
use crate::{
    InternalError,
    cdk::types::Principal,
    model::auth::{RootIssuerRenewalState, RootIssuerRenewalTemplate},
    ops::storage::auth::{AuthStateOps, ChainKeyRootDelegationBatchStatus},
};

#[derive(Clone)]
pub(super) struct DueChainKeyTemplate {
    pub(super) template: RootIssuerRenewalTemplate,
}

pub(super) fn due_chain_key_templates(
    now_ns: u64,
    required_issuer_pid: Option<Principal>,
) -> Vec<DueChainKeyTemplate> {
    AuthStateOps::root_issuer_renewal_templates()
        .into_iter()
        .filter(|template| template.enabled)
        .filter_map(|template| {
            if required_issuer_pid.is_some_and(|issuer_pid| issuer_pid == template.issuer_pid) {
                return Some(DueChainKeyTemplate { template });
            }
            let template_fingerprint =
                super::super::root_issuer_renewal::renewal_template_fingerprint(&template);
            let state = AuthStateOps::root_issuer_renewal_state(template.issuer_pid);
            chain_key_template_due(now_ns, template_fingerprint, state.as_ref())
                .then_some(DueChainKeyTemplate { template })
        })
        .collect()
}

pub(super) fn cap_due_chain_key_templates(due_templates: &mut Vec<DueChainKeyTemplate>) {
    if due_templates.len() > MAX_CHAIN_KEY_ROOT_DELEGATION_BATCH_ISSUERS {
        due_templates.truncate(MAX_CHAIN_KEY_ROOT_DELEGATION_BATCH_ISSUERS);
    }
}

pub(super) fn chain_key_template_due(
    now_ns: u64,
    template_fingerprint: [u8; 32],
    state: Option<&RootIssuerRenewalState>,
) -> bool {
    let Some(state) = state else {
        return true;
    };
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

pub(super) fn enabled_template_count() -> usize {
    AuthStateOps::root_issuer_renewal_templates()
        .into_iter()
        .filter(|template| template.enabled)
        .count()
}

pub(super) fn pending_chain_key_root_delegation_batch_count(now_ns: u64) -> usize {
    AuthStateOps::chain_key_root_delegation_batches()
        .into_iter()
        .filter(|batch| now_ns < batch.header.expires_at_ns)
        .filter(|batch| batch.status != ChainKeyRootDelegationBatchStatus::Installed)
        .count()
}

pub(super) fn chain_key_root_delegation_batch_quota_exceeded(
    pending_batches: usize,
) -> InternalError {
    InternalError::resource_exhausted(format!(
        "chain-key root delegation batch quota exceeded: pending_batches={pending_batches} max_pending_batches={MAX_PENDING_CHAIN_KEY_ROOT_DELEGATION_BATCHES}"
    ))
}
