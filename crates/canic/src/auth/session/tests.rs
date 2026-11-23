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
        let reply = response(&mut s.enrollment, s.receiver.state(), &mut root, 2, 1);
        s.enrollment.finish(s.receiver.state(), reply, 2).unwrap();
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

#[test]
fn predecessor_receiver_envelope_keeps_the_supplier_login_limit() {
    use crate::auth::install::{Install, Storage};
    let grant = grant();
    let old = (
        Storage::Stable,
        grant.receiver,
        grant.issuer,
        [1u8; 32],
        [2u8; 32],
        "user",
        (60u64, 300u64, 500u64, Some(7u32)),
        vec![9u8, 4],
    );
    let frame = ic_wire::Encoding::Frame {
        method: 0x4341_5554_4807,
        direction: ic_wire::Direction::Request,
    };
    let bytes = ic_wire::encode(&old, frame, enrollment::LIMITS).unwrap();
    let install = Install::decode(&bytes).unwrap();
    assert_eq!(install.limits.max_sessions, Some(7));
    assert_eq!(install.limits.max_admitted_logins, None);
    assert_eq!(install.application, [9, 4]);
    assert_eq!(install.encode().unwrap(), bytes);
    #[cfg(feature = "auth-receiver-stable")]
    {
        let enrollment =
            enrollment::Receiver::new(memory_config().enrollment, grant.audience(), 1).unwrap();
        let bytes = ic_wire::encode(
            &(old, Some(enrollment)),
            ic_wire::Encoding::Payload,
            enrollment::LIMITS,
        )
        .unwrap();
        let (saved, receiver) = stable::decode_host(&bytes);
        assert_eq!(saved.encode().unwrap(), install.encode().unwrap());
        assert!(receiver.is_some());
    }
}
