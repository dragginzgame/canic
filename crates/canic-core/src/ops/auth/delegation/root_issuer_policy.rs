//! Module: ops::auth::delegation::root_issuer_policy
//!
//! Responsibility: map and validate root issuer policy boundary DTOs.
//! Does not own: persisted record layout or batch proof preparation.

use crate::{
    InternalError,
    domain::policy::auth::{
        RootDelegatedRoleGrantPolicy, RootDelegationAudiencePolicy, RootIssuerPolicy,
    },
    dto::auth::{
        DelegatedRoleGrant, DelegationAudience, RootIssuerPolicyResponse,
        RootIssuerPolicyUpsertRequest, RootIssuerPolicyView,
    },
    ops::storage::auth::AuthStateOps,
};

pub(super) fn upsert_root_issuer_policy(
    request: RootIssuerPolicyUpsertRequest,
) -> Result<RootIssuerPolicyResponse, InternalError> {
    validate_root_issuer_policy_upsert_request(&request)?;

    let policy = root_issuer_policy_from_request(request);
    AuthStateOps::upsert_root_issuer_policy(policy.clone());

    Ok(RootIssuerPolicyResponse {
        issuer: root_issuer_policy_view(&policy),
    })
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

fn validate_root_issuer_policy_upsert_request(
    request: &RootIssuerPolicyUpsertRequest,
) -> Result<(), InternalError> {
    if request.max_cert_ttl_ns == 0 {
        return Err(InternalError::invalid_input(
            "root issuer max certificate TTL must be greater than zero",
        ));
    }
    if request.refresh_after_ratio_bps == 0 || request.refresh_after_ratio_bps >= 10_000 {
        return Err(InternalError::invalid_input(
            "root issuer refresh ratio must be between 1 and 9999 basis points",
        ));
    }
    if request.enabled && request.allowed_audiences.is_empty() {
        return Err(InternalError::invalid_input(
            "enabled root issuer policy must allow at least one audience",
        ));
    }
    if request.enabled && request.allowed_grants.is_empty() {
        return Err(InternalError::invalid_input(
            "enabled root issuer policy must allow at least one grant",
        ));
    }
    Ok(())
}

fn root_issuer_policy_from_request(request: RootIssuerPolicyUpsertRequest) -> RootIssuerPolicy {
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
        allowed_grants: policy
            .allowed_grants
            .iter()
            .map(delegated_role_grant_view)
            .collect(),
        max_cert_ttl_ns: policy.max_cert_ttl_ns,
        refresh_after_ratio_bps: policy.refresh_after_ratio_bps,
    }
}

fn delegation_audience_view(policy: &RootDelegationAudiencePolicy) -> DelegationAudience {
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
