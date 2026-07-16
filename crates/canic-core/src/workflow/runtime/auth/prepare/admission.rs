//! Module: workflow::runtime::auth::prepare::admission
//!
//! Responsibility: validate delegated-token and role-attestation prepare requests.
//! Does not own: replay state, proof creation, or response encoding.
//! Boundary: pure request checks plus deterministic configuration reads before replay reservation.

use crate::{
    InternalError,
    cdk::types::Principal,
    domain::policy::pure::auth::{
        AuthPolicyError, DelegatedRoleGrantPolicy, validate_public_delegated_token_prepare,
    },
    dto::{
        auth::{DelegatedRoleGrant, DelegatedTokenPrepareRequest, RoleAttestationRequest},
        error::Error,
    },
    ops::{config::ConfigOps, runtime::env::EnvOps, storage::registry::subnet::SubnetRegistryOps},
};

pub(super) fn validate_role_attestation_request(
    caller: Principal,
    request: &RoleAttestationRequest,
) -> Result<(), InternalError> {
    if request.subject != caller {
        return Err(InternalError::public(Error::forbidden(format!(
            "role attestation subject {} must match caller {}",
            request.subject, caller
        ))));
    }

    let (registered_role, _) =
        SubnetRegistryOps::role_parent(request.subject).ok_or_else(|| {
            InternalError::public(Error::forbidden(format!(
                "role attestation subject {} is not registered",
                request.subject
            )))
        })?;
    if registered_role != request.role {
        return Err(InternalError::public(Error::forbidden(format!(
            "role attestation role mismatch for subject {}: requested {}, registered {}",
            request.subject, request.role, registered_role
        ))));
    }

    if let Some(requested_subnet) = request.subnet_id {
        let local_subnet = EnvOps::subnet_pid()?;
        if requested_subnet != local_subnet {
            return Err(InternalError::public(Error::forbidden(format!(
                "role attestation subnet mismatch for subject {}: requested {}, local {}",
                request.subject, requested_subnet, local_subnet
            ))));
        }
    }

    let max_ttl_ns = role_attestation_max_ttl_ns()?;
    if request.ttl_ns == 0 || request.ttl_ns > max_ttl_ns {
        return Err(InternalError::public(Error::invalid(format!(
            "role attestation ttl_ns must satisfy 0 < ttl_ns <= {max_ttl_ns} (got {})",
            request.ttl_ns
        ))));
    }

    Ok(())
}

fn role_attestation_max_ttl_ns() -> Result<u64, InternalError> {
    let cfg = ConfigOps::role_attestation_config()?;
    cfg.max_ttl_secs.checked_mul(1_000_000_000).ok_or_else(|| {
        InternalError::public(Error::invalid(
            "auth.role_attestation.max_ttl_secs overflows nanoseconds",
        ))
    })
}

pub(super) fn validate_token_prepare_public_request(
    caller: Principal,
    request: &DelegatedTokenPrepareRequest,
) -> Result<(), InternalError> {
    let grants = request
        .grants
        .iter()
        .map(delegated_role_grant_policy)
        .collect::<Vec<_>>();
    validate_public_delegated_token_prepare(caller, request.subject, &grants)
        .map_err(map_token_prepare_policy_error)
}

fn delegated_role_grant_policy(grant: &DelegatedRoleGrant) -> DelegatedRoleGrantPolicy {
    DelegatedRoleGrantPolicy {
        target: grant.target.clone(),
        scopes: grant.scopes.clone(),
    }
}

fn map_token_prepare_policy_error(err: AuthPolicyError) -> InternalError {
    InternalError::public(Error::forbidden(err.to_string()))
}
