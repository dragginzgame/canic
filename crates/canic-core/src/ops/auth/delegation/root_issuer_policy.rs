//! Module: ops::auth::delegation::root_issuer_policy
//!
//! Responsibility: convert and persist root issuer policy values.
//! Does not own: policy admission, persisted record layout, or batch proof preparation.

use crate::{
    dto::auth::{
        DelegatedRoleGrant, DelegationAudience, RootIssuerPolicyResponse,
        RootIssuerPolicyUpsertRequest, RootIssuerPolicyView,
    },
    model::auth::{RootDelegatedRoleGrantPolicy, RootDelegationAudiencePolicy, RootIssuerPolicy},
    ops::storage::auth::AuthStateOps,
};

pub(super) fn commit_root_issuer_policy(policy: RootIssuerPolicy) -> RootIssuerPolicyResponse {
    AuthStateOps::upsert_root_issuer_policy(policy.clone());
    AuthStateOps::advance_delegated_auth_registry_epoch();

    RootIssuerPolicyResponse {
        issuer: root_issuer_policy_view(&policy),
    }
}

pub(super) fn audience_policy(audience: &DelegationAudience) -> RootDelegationAudiencePolicy {
    match audience {
        DelegationAudience::Canister(canister) => RootDelegationAudiencePolicy::Canister(*canister),
        DelegationAudience::CanicSubnet(subnet) => {
            RootDelegationAudiencePolicy::CanicSubnet(*subnet)
        }
        DelegationAudience::Project(project) => {
            RootDelegationAudiencePolicy::Project(project.clone())
        }
    }
}

pub(super) fn grant_policies(grants: &[DelegatedRoleGrant]) -> Vec<RootDelegatedRoleGrantPolicy> {
    grants.iter().map(grant_policy).collect()
}

pub(super) fn delegation_audience_view(
    policy: &RootDelegationAudiencePolicy,
) -> DelegationAudience {
    match policy {
        RootDelegationAudiencePolicy::Canister(canister) => DelegationAudience::Canister(*canister),
        RootDelegationAudiencePolicy::CanicSubnet(subnet) => {
            DelegationAudience::CanicSubnet(*subnet)
        }
        RootDelegationAudiencePolicy::Project(project) => {
            DelegationAudience::Project(project.clone())
        }
    }
}

pub(super) fn delegated_role_grant_views(
    grants: &[RootDelegatedRoleGrantPolicy],
) -> Vec<DelegatedRoleGrant> {
    grants.iter().map(delegated_role_grant_view).collect()
}

pub(super) fn root_issuer_policy_from_request(
    request: RootIssuerPolicyUpsertRequest,
) -> RootIssuerPolicy {
    RootIssuerPolicy {
        issuer_pid: request.issuer_pid,
        enabled: request.enabled,
        allowed_audiences: request
            .allowed_audiences
            .iter()
            .map(audience_policy)
            .collect(),
        allowed_grants: request.allowed_grants.iter().map(grant_policy).collect(),
        max_cert_ttl_ns: request.max_cert_ttl_ns,
        refresh_after_ratio_bps: request.refresh_after_ratio_bps,
    }
}

fn root_issuer_policy_view(policy: &RootIssuerPolicy) -> RootIssuerPolicyView {
    RootIssuerPolicyView {
        issuer_pid: policy.issuer_pid,
        enabled: policy.enabled,
        allowed_audiences: policy
            .allowed_audiences
            .iter()
            .map(delegation_audience_view)
            .collect(),
        allowed_grants: delegated_role_grant_views(&policy.allowed_grants),
        max_cert_ttl_ns: policy.max_cert_ttl_ns,
        refresh_after_ratio_bps: policy.refresh_after_ratio_bps,
    }
}

fn delegated_role_grant_view(policy: &RootDelegatedRoleGrantPolicy) -> DelegatedRoleGrant {
    DelegatedRoleGrant {
        target: policy.target.clone(),
        scopes: policy.scopes.clone(),
    }
}

fn grant_policy(grant: &DelegatedRoleGrant) -> RootDelegatedRoleGrantPolicy {
    RootDelegatedRoleGrantPolicy {
        target: grant.target.clone(),
        scopes: grant.scopes.clone(),
    }
}
