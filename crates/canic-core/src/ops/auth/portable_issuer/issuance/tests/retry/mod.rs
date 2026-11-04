//! Retry reads reuse only the original nonce under still-live local authority.
use super::*;

#[test]
fn retained_nonce_read_is_effect_free_and_checks_clock_deadline_and_key() {
    let _lock = seams::lock();
    let _state = StateGuard::empty();
    enroll();
    let pending = pending_at(1, NOW);
    let empty = PortableIssuerState::read();
    assert_eq!(retained_challenge(&pending, NOW), Ok(None));
    assert_eq!(PortableIssuerState::read(), empty);
    challenge(&pending, [5; 32], NOW).unwrap();
    let original = PortableIssuerState::read();
    assert_eq!(
        retained_challenge(&pending, NOW + SECOND),
        Ok(Some([5; 32]))
    );
    assert_eq!(retained_challenge(&pending, NOW - 1), Err(E::Clock));
    assert_eq!(
        retained_challenge(&pending, NOW + 6 * SECOND),
        Err(E::Expired)
    );
    assert_eq!(PortableIssuerState::read(), original);
    keys::restore().unwrap();
    let restored = PortableIssuerState::read();
    assert_eq!(retained_challenge(&pending, NOW + SECOND), Err(E::Conflict));
    assert_eq!(PortableIssuerState::read(), restored);
}
