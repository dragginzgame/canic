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
    ids::{CanisterRole, ManagedCanisterBinding, SubnetId},
    ops::config::ConfigOps,
};

pub(super) fn validate_role_attestation_request(
    caller: Principal,
    request: &RoleAttestationRequest,
    member: &ManagedCanisterBinding,
) -> Result<(), InternalError> {
    validate_active_component_subject(caller, request, member)?;

    let max_ttl_ns = role_attestation_max_ttl_ns()?;
    if request.ttl_ns == 0 || request.ttl_ns > max_ttl_ns {
        return Err(InternalError::public(Error::invalid(format!(
            "role attestation ttl_ns must satisfy 0 < ttl_ns <= {max_ttl_ns} (got {})",
            request.ttl_ns
        ))));
    }

    Ok(())
}

fn validate_active_component_subject(
    caller: Principal,
    request: &RoleAttestationRequest,
    member: &ManagedCanisterBinding,
) -> Result<(), InternalError> {
    let authority = RoleAttestationMemberAuthority::from(member);
    if request.subject != caller {
        return Err(InternalError::public(Error::forbidden(format!(
            "role attestation subject {} must match caller {}",
            request.subject, caller
        ))));
    }

    if authority.canister_id != caller {
        return Err(InternalError::public(Error::forbidden(format!(
            "role attestation caller {} differs from active Component Registry member {}",
            caller, authority.canister_id
        ))));
    }
    if authority.role != &request.role {
        return Err(InternalError::public(Error::forbidden(format!(
            "role attestation role mismatch for subject {}: requested {}, registered {}",
            request.subject, request.role, authority.role
        ))));
    }

    if let Some(requested_subnet) = request.subnet_id
        && requested_subnet != authority.placement_subnet.into_principal()
    {
        return Err(InternalError::public(Error::forbidden(format!(
            "role attestation subnet mismatch for subject {}: requested {}, local {}",
            request.subject, requested_subnet, authority.placement_subnet
        ))));
    }

    Ok(())
}

struct RoleAttestationMemberAuthority<'a> {
    canister_id: Principal,
    role: &'a CanisterRole,
    placement_subnet: &'a SubnetId,
}

impl<'a> From<&'a ManagedCanisterBinding> for RoleAttestationMemberAuthority<'a> {
    fn from(member: &'a ManagedCanisterBinding) -> Self {
        match member {
            ManagedCanisterBinding::Component(component) => Self {
                canister_id: component.canister_id,
                role: &component.role,
                placement_subnet: &component.placement_subnet,
            },
            ManagedCanisterBinding::ComponentChild(child) => Self {
                canister_id: child.canister_id,
                role: &child.role,
                placement_subnet: &child.component.placement_subnet,
            },
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        dto::error::ErrorCode,
        ids::{
            AppId, CanisterRole, CanonicalNetworkId, ComponentBinding, ComponentChildBinding,
            ComponentInstanceId, FleetBinding, FleetCoordinatorBinding, FleetId, FleetKey,
            FleetRegistryAuthority, ManagedCanisterBinding, SubnetId,
        },
    };

    fn p(byte: u8) -> Principal {
        Principal::from_slice(&[byte; 29])
    }

    fn component() -> ComponentBinding {
        ComponentBinding {
            authority: FleetRegistryAuthority {
                binding: FleetCoordinatorBinding {
                    fleet: FleetBinding {
                        fleet: FleetKey {
                            canonical_network_id: CanonicalNetworkId::ic_mainnet(),
                            fleet_id: FleetId::from_generated_bytes([1; 32]),
                        },
                        app: AppId::from("auth-test"),
                    },
                    coordinator_subnet: SubnetId::from_principal(p(2)),
                    coordinator: p(3),
                },
                epoch: 1,
            },
            component: ComponentInstanceId::from_generated_bytes([4; 32]),
            component_spec: "issuer".parse().expect("Component Spec"),
            spec_hash: [5; 32],
            role: CanisterRole::from("issuer"),
            placement_subnet: SubnetId::from_principal(p(6)),
            fleet_subnet_root: p(7),
            canister_id: p(8),
        }
    }

    fn request(component: &ComponentBinding) -> RoleAttestationRequest {
        RoleAttestationRequest {
            subject: component.canister_id,
            role: component.role.clone(),
            subnet_id: Some(component.placement_subnet.into_principal()),
            audience: p(9),
            ttl_ns: 60_000_000_000,
            epoch: component.authority.epoch,
            metadata: None,
        }
    }

    #[test]
    fn role_attestation_subject_accepts_exact_component_binding() {
        let component = component();
        let member = ManagedCanisterBinding::Component(component.clone());

        validate_active_component_subject(component.canister_id, &request(&component), &member)
            .expect("exact active Component binding");
    }

    #[test]
    fn role_attestation_subject_accepts_exact_component_child_binding() {
        let component = component();
        let child = ComponentChildBinding {
            component: component.clone(),
            parent_canister_id: component.canister_id,
            role: CanisterRole::from("project_instance"),
            canister_id: p(12),
        };
        let request = RoleAttestationRequest {
            subject: child.canister_id,
            role: child.role.clone(),
            subnet_id: Some(component.placement_subnet.into_principal()),
            audience: p(9),
            ttl_ns: 60_000_000_000,
            epoch: component.authority.epoch,
            metadata: None,
        };
        let member = ManagedCanisterBinding::ComponentChild(child.clone());

        validate_active_component_subject(child.canister_id, &request, &member)
            .expect("exact active Component Child binding");
    }

    #[test]
    fn role_attestation_subject_rejects_caller_or_role_drift() {
        let component = component();
        let member = ManagedCanisterBinding::Component(component.clone());
        let mut subject_drift = request(&component);
        subject_drift.subject = p(10);
        let mut role_drift = request(&component);
        role_drift.role = CanisterRole::from("project_hub");

        for (caller, request) in [
            (component.canister_id, subject_drift),
            (p(10), request(&component)),
            (component.canister_id, role_drift),
        ] {
            let error = validate_active_component_subject(caller, &request, &member)
                .expect_err("binding drift must fail closed");
            assert_eq!(
                error.public_error().expect("public rejection").code,
                ErrorCode::Forbidden
            );
        }
    }

    #[test]
    fn role_attestation_subject_rejects_placement_subnet_drift() {
        let component = component();
        let member = ManagedCanisterBinding::Component(component.clone());
        let mut request = request(&component);
        request.subnet_id = Some(p(11));

        let error = validate_active_component_subject(component.canister_id, &request, &member)
            .expect_err("placement Subnet drift must fail closed");

        assert_eq!(
            error.public_error().expect("public rejection").code,
            ErrorCode::Forbidden
        );
    }
}
