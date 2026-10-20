//! Module: domain::policy::pure::application_checkpoint::tests
//!
//! Responsibility: qualify transport caller and complete installation audience refusals.
//! Boundary: these pure tests supply current authority; they prove no installed endpoint.

use super::require_root;
use crate::model::{
    application_checkpoint::tests::binding, caller_authority::CallerAdmissionError as Error,
};
use ic_platform::principal::ExternalPrincipal as Principal;

#[test]
fn exact_installed_root_is_admitted() {
    let request = binding(1);
    assert_eq!(
        require_root(&request, &request.authority, request.authority.issuer.root),
        Ok(())
    );
}

#[test]
fn controller_or_subject_cannot_substitute_for_the_issuing_root() {
    let request = binding(1);
    for caller in [
        Principal::anonymous(),
        request.authority.receiver.canister(),
        Principal::from_slice(&[99]),
    ] {
        assert_eq!(
            require_root(&request, &request.authority, caller),
            Err(Error::PermissionDenied)
        );
    }
}

#[test]
fn changed_installation_registry_and_policy_refuse_before_application_work() {
    let request = binding(1);
    let current = request.authority.clone();
    let mut changed = Vec::new();
    let mut authority = current.clone();
    authority.receiver.install_id = [99; 32];
    changed.push(authority);
    let mut authority = current.clone();
    authority.receiver.component_install_id = [99; 32];
    changed.push(authority);
    let mut authority = current.clone();
    authority.issuer.install_id = [99; 32];
    changed.push(authority);
    let mut authority = current.clone();
    authority.issuer.registry.epoch += 1;
    changed.push(authority);
    let mut authority = current.clone();
    authority.issuer.root = Principal::from_slice(&[99]);
    changed.push(authority);
    let mut authority = current.clone();
    authority.policy_digest = [99; 32];
    changed.push(authority);
    let mut authority = current.clone();
    if let canic_contracts::ids::ManagedCanisterBinding::Component(component) =
        &mut authority.receiver.binding
    {
        component.canister_id = Principal::from_slice(&[99]);
    }
    changed.push(authority);
    for authority in changed {
        let mut requested = request.clone();
        requested.authority = authority;
        assert_eq!(
            require_root(&requested, &current, current.issuer.root),
            Err(Error::AuthorityConflict)
        );
    }
}
