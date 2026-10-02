//! Module: ops::auth::delegation::issuer_configuration
//!
//! Responsibility: derive and persist issuer policy and renewal configuration together.
//! Does not own: policy admission, persisted record layout, or batch proof preparation.

use crate::{
    InternalError,
    diagnostics::codes,
    dto::auth::{
        DelegatedRoleGrant, DelegationAudience, RootIssuerConfigureRequest,
        RootIssuerConfigureResponse, RootIssuerPolicyView,
    },
    ids::FleetKey,
    model::auth::{RootDelegatedRoleGrantPolicy, RootIssuerPolicy, RootIssuerRenewalTemplate},
    ops::{auth::delegated::audience::validate_role_grants, storage::auth::RootDelegationStateOps},
};

///
/// RootIssuerConfiguration
///
/// Policy and renewal material derived by ops for workflow admission.
///

pub struct RootIssuerConfiguration {
    pub policy: RootIssuerPolicy,
    pub template: RootIssuerRenewalTemplate,
}

pub(super) fn commit_root_issuer_configuration(
    configuration: RootIssuerConfiguration,
    now_ns: u64,
) -> RootIssuerConfigureResponse {
    let RootIssuerConfiguration { policy, template } = configuration;
    let unchanged = RootDelegationStateOps::root_issuer_policy(policy.issuer_pid).as_ref()
        == Some(&policy)
        && RootDelegationStateOps::root_issuer_renewal_template(policy.issuer_pid).as_ref()
            == Some(&template);
    if !unchanged {
        RootDelegationStateOps::upsert_root_issuer_policy(policy.clone());
        super::root_issuer_renewal::commit_root_issuer_renewal_template(template.clone(), now_ns);
        RootDelegationStateOps::advance_delegated_auth_registry_epoch();
    }

    RootIssuerConfigureResponse {
        issuer: root_issuer_policy_view(&policy),
        template: super::root_issuer_renewal::root_issuer_renewal_template_view(&template),
    }
}

pub(super) const fn audience_policy(audience: &DelegationAudience) -> FleetKey {
    match audience {
        DelegationAudience::Fleet(fleet) => *fleet,
    }
}

pub(super) fn grant_policies(grants: &[DelegatedRoleGrant]) -> Vec<RootDelegatedRoleGrantPolicy> {
    grants.iter().map(grant_policy).collect()
}

pub(super) const fn delegation_audience_view(fleet: &FleetKey) -> DelegationAudience {
    DelegationAudience::Fleet(*fleet)
}

pub(super) fn delegated_role_grant_views(
    grants: &[RootDelegatedRoleGrantPolicy],
) -> Vec<DelegatedRoleGrant> {
    grants.iter().map(delegated_role_grant_view).collect()
}

pub(super) fn root_issuer_configuration_from_request(
    mut request: RootIssuerConfigureRequest,
) -> Result<RootIssuerConfiguration, InternalError> {
    request
        .grants
        .sort_by(|left, right| left.target.as_str().cmp(right.target.as_str()));
    for grant in &mut request.grants {
        grant.scopes.sort();
    }
    if !request.grants.is_empty() {
        validate_role_grants(&request.grants)
            .map_err(|_| InternalError::public(codes::CONFIGURATION_INVALID))?;
    }
    let audience = audience_policy(&request.aud);
    let grants = grant_policies(&request.grants);
    let policy = RootIssuerPolicy {
        issuer_pid: request.issuer_pid,
        enabled: request.enabled,
        allowed_audiences: vec![audience],
        allowed_grants: grants.clone(),
        max_cert_ttl_ns: request.cert_ttl_ns,
        refresh_after_ratio_bps: request.refresh_after_ratio_bps,
    };
    let template = RootIssuerRenewalTemplate {
        issuer_pid: request.issuer_pid,
        enabled: request.enabled,
        audience,
        grants,
        cert_ttl_ns: request.cert_ttl_ns,
    };
    Ok(RootIssuerConfiguration { policy, template })
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
