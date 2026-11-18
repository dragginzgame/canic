use super::*;
use crate::auth::enrollment::tests::{grant, memory_config, response};

#[test]
fn repeated_receiver_initialization_preserves_logout_evidence_and_capacity() {
    use crate::auth::{
        protocol::model::{LogoutResult, LogoutUnavailable},
        session,
    };
    let grant = grant();
    let policy = || session::Policy {
        issuer: grant.issuer,
        role: "user".parse().unwrap(),
        session_ttl: SessionTtlPolicy::new(5, 10).unwrap(),
        max_proof_ttl_ns: 1000,
    };
    session::initialize(memory_config(), policy(), grant.receiver, 1);
    let mut root = enrollment::Authority::new(grant.clone(), 1).unwrap();
    STATE.with_borrow_mut(|slot| {
        let s = slot.as_mut().unwrap();
        let reply = response(&mut s.enrollment, &mut s.receiver, &mut root, 2, 1);
        s.enrollment.finish(&mut s.receiver, reply, 2).unwrap();
    });
    let first = Principal::self_authenticating(b"first");
    let receipt = session::logout(first, 1_000_000_002);
    assert!(
        matches!(receipt, LogoutResult::Unconfirmed(_)),
        "{receipt:?}"
    );
    for receiver in [Principal::anonymous(), grant.receiver] {
        assert!(
            std::panic::catch_unwind(|| session::initialize(
                memory_config(),
                policy(),
                receiver,
                3
            ))
            .is_err()
        );
        assert_eq!(session::logout(first, 1_000_000_004), receipt);
        assert_eq!(
            session::logout(Principal::self_authenticating(b"other"), 1_000_000_004),
            LogoutResult::Unavailable(LogoutUnavailable::Capacity)
        );
    }
}
