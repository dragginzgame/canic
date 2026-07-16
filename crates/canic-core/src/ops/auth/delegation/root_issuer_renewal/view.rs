//! Module: ops::auth::delegation::root_issuer_renewal::view
//!
//! Responsibility: project root issuer renewal policy records into boundary views.
//! Does not own: storage mutation, scheduling decisions, or install outcome handling.

use crate::{
    dto::auth::{
        RootIssuerRenewalBatchStatus, RootIssuerRenewalBatchView, RootIssuerRenewalStateView,
        RootIssuerRenewalTemplateView,
    },
    model::auth::{RootIssuerRenewalState, RootIssuerRenewalTemplate},
    ops::{
        auth::delegation::root_issuer_policy::{
            delegated_role_grant_views, delegation_audience_view,
        },
        storage::auth::{
            ChainKeyRootDelegationBatch, ChainKeyRootDelegationBatchIssuer,
            ChainKeyRootDelegationBatchStatus,
        },
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

pub(super) const fn root_issuer_renewal_state_view(
    state: &RootIssuerRenewalState,
) -> RootIssuerRenewalStateView {
    RootIssuerRenewalStateView {
        issuer_pid: state.issuer_pid,
        template_fingerprint: state.template_fingerprint,
        last_installed_cert_hash: state.last_installed_cert_hash,
        last_installed_expires_at_ns: state.last_installed_expires_at_ns,
        last_installed_refresh_after_ns: state.last_installed_refresh_after_ns,
        next_attempt_after_ns: state.next_attempt_after_ns,
        updated_at_ns: state.updated_at_ns,
    }
}

pub(super) fn root_issuer_renewal_batch_view(
    batch: &ChainKeyRootDelegationBatch,
    issuer: &ChainKeyRootDelegationBatchIssuer,
) -> RootIssuerRenewalBatchView {
    RootIssuerRenewalBatchView {
        batch_id: batch.batch_id,
        status: root_issuer_renewal_batch_status_view(batch.status, issuer.installed_at_ns),
        cert_hash: issuer.cert_hash,
        proof_epoch: batch.header.proof_epoch,
        prepared_at_ns: batch.prepared_at_ns,
        expires_at_ns: batch.header.expires_at_ns,
        installed_at_ns: issuer.installed_at_ns,
        retry_after_ns: batch.retry_after_ns,
        failure: issuer.last_failure.clone().or_else(|| {
            if batch.status == ChainKeyRootDelegationBatchStatus::FailedRetryable {
                batch.failure.clone()
            } else {
                None
            }
        }),
    }
}

const fn root_issuer_renewal_batch_status_view(
    status: ChainKeyRootDelegationBatchStatus,
    installed_at_ns: Option<u64>,
) -> RootIssuerRenewalBatchStatus {
    if installed_at_ns.is_some() {
        return RootIssuerRenewalBatchStatus::Installed;
    }
    match status {
        ChainKeyRootDelegationBatchStatus::Prepared => RootIssuerRenewalBatchStatus::Prepared,
        ChainKeyRootDelegationBatchStatus::Signing => RootIssuerRenewalBatchStatus::Signing,
        ChainKeyRootDelegationBatchStatus::Signed => RootIssuerRenewalBatchStatus::Signed,
        ChainKeyRootDelegationBatchStatus::Installing => RootIssuerRenewalBatchStatus::Installing,
        ChainKeyRootDelegationBatchStatus::Installed => RootIssuerRenewalBatchStatus::Installed,
        ChainKeyRootDelegationBatchStatus::FailedRetryable => {
            RootIssuerRenewalBatchStatus::FailedRetryable
        }
    }
}
