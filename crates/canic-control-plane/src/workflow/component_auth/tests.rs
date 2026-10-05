//! Membership observations distinguish ordinary absence from broken authority.

use super::*;

#[test]
fn lookup_returns_an_ordinary_negative_for_inactive_membership() {
    assert_eq!(
        membership_observation(Err(ActiveComponentMemberError::NotActive)),
        Ok(None)
    );
}

#[test]
fn lookup_preserves_registry_and_runtime_errors() {
    for error in [
        InternalError::invariant(),
        InternalError::platform_failure(),
    ] {
        let expected = error.public_error();
        assert_eq!(
            membership_observation(Err(ActiveComponentMemberError::Internal(error))),
            Err(expected)
        );
    }
}
