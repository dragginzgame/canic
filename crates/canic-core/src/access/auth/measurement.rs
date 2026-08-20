//! Module: access::auth::measurement
//!
//! Responsibility: provide a feature-gated Wasm probe for the closed denial partition.
//! Does not own: production authorization, state acquisition, or endpoint behavior.
//! Boundary: internal test fixtures invoke the same private value-to-policy adapter.

use super::{
    LocalApplicationAuthorizationDecision, LocalApplicationAuthorizationDenial,
    LocalApplicationAuthorizationRequest, authorize_with_values,
};
use crate::{
    cdk::types::Principal,
    ids::{CanisterRole, CanonicalNetworkId, FleetId, FleetKey},
    model::auth::application_authorization::{
        ApplicationScope, ApplicationScopeRef, CanonicalApplicationScopes,
        LocalApplicationAuthoritySnapshot, LocalApplicationSession,
    },
};

/// Measure one exact pure denial branch in a Wasm query fixture.
///
/// This is compiled only by internal test fixtures. Production consumers must
/// use [`super::authorize_local_application`], which acquires ambient authority.
#[must_use]
pub fn measure_local_application_authorization_denial(
    expected: LocalApplicationAuthorizationDenial,
) -> LocalApplicationAuthorizationDecision {
    let caller = Principal::from_slice(&[7; 29]);
    let other = Principal::from_slice(&[8; 29]);
    let fleet = FleetKey {
        canonical_network_id: CanonicalNetworkId::ic_mainnet(),
        fleet_id: FleetId::from_generated_bytes([9; 32]),
    };
    let role = CanisterRole::from("application");
    let authority = LocalApplicationAuthoritySnapshot::new(fleet, role.clone(), 4);
    let session_generation = if expected == LocalApplicationAuthorizationDenial::StaleAuthority {
        3
    } else {
        4
    };
    let session = LocalApplicationSession::new(
        caller,
        caller,
        Principal::from_slice(&[10; 29]),
        fleet,
        role,
        CanonicalApplicationScopes::for_session(vec![
            ApplicationScope::parse("application:read").expect("canonical fixture scope"),
        ])
        .expect("bounded fixture scopes"),
        session_generation,
        10,
        100,
        [11; 32],
        [12; 32],
    )
    .expect("valid fixture session");

    let actual_caller = if expected == LocalApplicationAuthorizationDenial::Anonymous {
        Principal::anonymous()
    } else if expected == LocalApplicationAuthorizationDenial::CallerMismatch {
        other
    } else {
        caller
    };
    let capability_enabled = expected != LocalApplicationAuthorizationDenial::Disabled;
    let authority = (expected != LocalApplicationAuthorizationDenial::AuthorityUnavailable)
        .then_some(&authority);
    let session =
        (expected != LocalApplicationAuthorizationDenial::MissingSession).then_some(&session);
    let now_ns = if expected == LocalApplicationAuthorizationDenial::Expired {
        100
    } else {
        20
    };
    let subject_admissible = expected != LocalApplicationAuthorizationDenial::InadmissibleSubject;
    let required_scope = if expected == LocalApplicationAuthorizationDenial::MissingScope {
        ApplicationScopeRef::from_static("application:write")
    } else {
        ApplicationScopeRef::from_static("application:read")
    };
    let decision = authorize_with_values(
        LocalApplicationAuthorizationRequest {
            observed_transport_caller: caller,
            required_scope,
        },
        actual_caller,
        now_ns,
        capability_enabled,
        authority,
        session,
        subject_admissible,
    );
    assert_eq!(
        decision,
        LocalApplicationAuthorizationDecision::Deny(expected)
    );
    decision
}
