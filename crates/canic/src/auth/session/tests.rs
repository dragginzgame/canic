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
    session::initialize_with(
        memory_config(),
        policy(),
        grant.receiver,
        1,
        crate::auth::enrollment::tests::fresh_engine,
    );
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
            std::panic::catch_unwind(|| session::initialize_with(
                memory_config(),
                policy(),
                receiver,
                3,
                crate::auth::enrollment::tests::fresh_engine
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
fn pre_cut_receiver_frames_are_refused() {
    let grant = grant();
    for method in [0x4341_5554_4807, 0x4341_5554_4808] {
        let frame = ic_wire::Encoding::Frame {
            method,
            direction: ic_wire::Direction::Request,
        };
        let bytes = ic_wire::encode(
            &(
                1_u8,
                grant.receiver,
                grant.issuer,
                [1_u8; 32],
                [2_u8; 32],
                "user",
                (60_u64, 300_u64, 500_u64, Some(7_u32)),
                vec![9_u8, 4],
            ),
            frame,
            enrollment::LIMITS,
        )
        .unwrap();
        #[cfg(feature = "topology")]
        assert!(crate::topology::Install::approved(&bytes).is_err());
        assert!(crate::auth::install::Install::decode(&bytes).is_err());
    }
}
