//! Module: access::auth::measurement
//!
//! Responsibility: measure the implemented caller gate while enrollment is unavailable.
//! Does not own: session authorization, state acquisition, or production enrollment.
//! Boundary: the fixture runs the same closed caller gate as the public facade.

use super::{
    ApplicationScopeRef, LocalApplicationAuthorizationDecision,
    LocalApplicationAuthorizationDenial, LocalApplicationAuthorizationRequest,
    authorize_with_values,
};
use crate::cdk::types::Principal;

/// Measure an implemented closed boundary outcome; session decisions belong to IC Auth.
#[must_use]
pub fn measure_local_application_authorization_denial(
    expected: LocalApplicationAuthorizationDenial,
) -> LocalApplicationAuthorizationDecision {
    let caller = Principal::from_slice(&[7; 29]);
    let actual = match expected {
        LocalApplicationAuthorizationDenial::Anonymous => Principal::anonymous(),
        LocalApplicationAuthorizationDenial::CallerMismatch => Principal::from_slice(&[8; 29]),
        LocalApplicationAuthorizationDenial::AuthorityUnavailable => caller,
        _ => panic!("session outcomes require the real IC Auth engine"),
    };
    let decision = authorize_with_values(
        LocalApplicationAuthorizationRequest {
            observed_transport_caller: caller,
            required_scope: ApplicationScopeRef::from_static("application:read"),
        },
        actual,
    );
    assert_eq!(
        decision,
        LocalApplicationAuthorizationDecision::Deny(expected)
    );
    decision
}
