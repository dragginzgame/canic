use super::*;

fn grant() -> Grant {
    Grant {
        receiver: Principal::from_slice(&[1]),
        audience: AudienceId {
            canonical_network_id: [1; 32],
            fleet_id: [2; 32],
        },
        issuer: Principal::from_slice(&[2]),
        key: [
            0xea, 0x4a, 0x6c, 0x63, 0xe2, 0x9c, 0x52, 0x0a, 0xbe, 0xf5, 0x50, 0x7b, 0x13, 0x2e,
            0xc5, 0xf9, 0x95, 0x47, 0x76, 0xae, 0xbe, 0xbe, 0x7b, 0x92, 0x42, 0x1e, 0xea, 0x69,
            0x14, 0x46, 0xd2, 0x2c,
        ],
        grant: DelegatedRoleGrant {
            target: ic_wire::Text::new("user".into()).unwrap(),
            scopes: ic_wire::List::new(vec![ic_wire::Text::new("read".into()).unwrap()]).unwrap(),
        },
        not_before_ns: 1,
        expires_at_ns: 300_000_000_000,
    }
}
#[test]
fn authority_retains_exact_windows_and_terminal_revocation() {
    let grant = grant();
    let mut authority = Authority::new(grant.clone(), 1).unwrap();
    let request = Request {
        challenge: [1; 32],
        receipt_deadline_ns: RECEIPT_NS,
    };
    assert_eq!(
        authority.snapshot(Principal::from_slice(&[9]), request, 2),
        Err(Error::Binding)
    );
    let first = authority.snapshot(grant.receiver, request, 3).unwrap();
    assert_eq!(
        first.revision, 1,
        "refusals do not consume snapshot revision"
    );
    assert_eq!(
        authority.snapshot(grant.receiver, request, 2),
        Err(Error::Invalid)
    );
    assert_eq!(
        authority.snapshot(grant.receiver, request, 4).unwrap(),
        first
    );
    let changed = Request {
        receipt_deadline_ns: RECEIPT_NS + 1,
        ..request
    };
    assert_eq!(
        authority.snapshot(grant.receiver, changed, 5),
        Err(Error::Conflict)
    );
    authority.revoke(6).unwrap();
    authority.matches(&grant).unwrap(); // Configuration retry does not clear retirement.
    assert_eq!(
        authority.snapshot(grant.receiver, request, 7),
        Err(Error::Conflict)
    );
    let retired = authority
        .snapshot(
            grant.receiver,
            Request {
                challenge: [2; 32],
                ..request
            },
            8,
        )
        .unwrap();
    assert!(retired.revoked);
    assert!(retired.generation > first.generation);
    assert_ne!(
        retired.metadata().unwrap().authority_digest,
        first.metadata().unwrap().authority_digest
    );
    assert_eq!(
        authority.snapshot(grant.receiver, retired.request, RECEIPT_NS),
        Err(Error::Expired)
    );
}

#[test]
fn expired_pending_work_cannot_renew_or_replace_newer_admission() {
    let (mut receiver, mut engine, mut authority) = receiver_engine();
    let first = response(&mut receiver, &mut engine, &mut authority, 2, 1);
    let late = duplicate(&first);
    assert_eq!(receiver.begin(3), Err(Error::Busy));
    assert_eq!(
        receiver.finish(&mut engine, first, RECEIPT_NS + 2),
        Err(Error::Expired)
    );
    assert!(engine.enrollment().is_none());
    let next = response(
        &mut receiver,
        &mut engine,
        &mut authority,
        RECEIPT_NS + 3,
        2,
    );
    assert_eq!(
        receiver.finish(&mut engine, late, RECEIPT_NS + 4),
        Err(Error::Conflict)
    );
    assert!(engine.enrollment().is_none());
    receiver.finish(&mut engine, next, RECEIPT_NS + 4).unwrap();
    assert_eq!(engine.enrollment().unwrap().challenge, [2; 32]);
}

fn receiver_engine() -> (
    Receiver,
    supplier::receiver_memory::MemoryReceiver,
    Authority,
) {
    use supplier::{
        receiver_memory::{
            EnrollmentBinding, LoginLimits, MemoryConfig, MemoryReceiver, SessionCapacity,
            UnknownLogoutLimits,
        },
        verification::VerificationCosts,
    };
    let grant = grant();
    let binding = EnrollmentBinding {
        receiver: grant.receiver,
        authority: Principal::from_slice(&[3]),
    };
    let engine = MemoryReceiver::new_fresh(
        MemoryConfig {
            audience: grant.audience(),
            enrollment: binding,
            login: LoginLimits {
                max_device_slots: 1,
                max_login_ttl_ns: 300_000_000_000,
                max_admitted_logins: 1,
            },
            capacity: SessionCapacity {
                max_sessions: 1,
                max_replays: 2,
                max_replays_per_account: 2,
                max_encoded_bytes: 4096,
            },
            verification_capacity: 10,
            refill_interval_ns: 1_000_000_000,
            verification_costs: VerificationCosts {
                ed25519: 1,
                certified: 2,
            },
            unknown_logout: UnknownLogoutLimits {
                max_denials: 1,
                capacity: 1,
                refill_interval_ns: 1_000_000_000,
            },
        },
        1,
    )
    .unwrap();
    (
        Receiver::new(binding, grant.audience(), 1).unwrap(),
        engine,
        Authority::new(grant, 1).unwrap(),
    )
}
fn response(
    receiver: &mut Receiver,
    engine: &mut supplier::receiver_memory::MemoryReceiver,
    authority: &mut Authority,
    now: u64,
    nonce: u8,
) -> Authenticated {
    let start = receiver.begin(now).unwrap();
    let pull = receiver
        .prepare(
            engine,
            Challenge {
                start,
                bytes: [nonce; 32],
            },
            now,
        )
        .unwrap();
    let snapshot = authority
        .snapshot(grant().receiver, pull.request, now)
        .unwrap();
    Authenticated { pull, snapshot }
}
fn duplicate(response: &Authenticated) -> Authenticated {
    Authenticated {
        pull: Pull {
            authority: response.pull.authority,
            start: response.pull.start.clone(),
            request: response.pull.request,
        },
        snapshot: response.snapshot.clone(),
    }
}
#[test]
fn complete_reply_validation_precedes_atomic_metadata_and_entry_installation() {
    // These deliberately forged authenticated markers are private unit-test
    // construction. Public callers can only obtain one from a direct IC call.
    for fault in 0..9 {
        let (mut receiver, mut engine, mut authority) = receiver_engine();
        let mut reply = response(&mut receiver, &mut engine, &mut authority, 2, 1);
        match fault {
            0 => reply.pull.authority = Principal::from_slice(&[4]),
            1 => reply.snapshot.grant.receiver = Principal::from_slice(&[4]),
            2 => reply.snapshot.grant.audience.fleet_id = [9; 32],
            3 => reply.snapshot.request.challenge = [9; 32],
            4 => reply.snapshot.request.receipt_deadline_ns += 1,
            5 => reply.snapshot.issued_at_ns += 1,
            6 => reply.snapshot.expires_at_ns = reply.snapshot.issued_at_ns,
            7 => {
                reply.snapshot.grant.grant.scopes = ic_wire::List::new(vec![
                    ic_wire::Text::new("z".into()).unwrap(),
                    ic_wire::Text::new("a".into()).unwrap(),
                ])
                .unwrap()
            }
            8 => reply.snapshot.grant.key = [0; 32],
            _ => unreachable!(),
        }
        assert!(
            receiver.finish(&mut engine, reply, 2).is_err(),
            "fault {fault}"
        );
        assert!(engine.enrollment().is_none());
        assert_eq!(receiver.current(2), Err(Error::Closed));
    }
    let (mut receiver, mut engine, mut authority) = receiver_engine();
    let first = response(&mut receiver, &mut engine, &mut authority, 2, 1);
    let replay = duplicate(&first);
    let late = duplicate(&first);
    receiver.finish(&mut engine, first, 2).unwrap();
    let metadata = engine.enrollment().unwrap();
    receiver.finish(&mut engine, replay, 3).unwrap();
    assert_eq!(engine.enrollment(), Some(metadata));
    let newer = response(&mut receiver, &mut engine, &mut authority, 4, 2);
    assert_eq!(receiver.finish(&mut engine, late, 5), Err(Error::Conflict));
    receiver.finish(&mut engine, newer, 5).unwrap();
    assert_eq!(
        engine.enrollment().unwrap().authority_digest,
        metadata.authority_digest
    );

    authority.revoke(6).unwrap();
    let revoked = response(&mut receiver, &mut engine, &mut authority, 6, 3);
    receiver.finish(&mut engine, revoked, 6).unwrap();
    let retired = engine.enrollment().unwrap();
    let mut revived = response(&mut receiver, &mut engine, &mut authority, 7, 4);
    revived.snapshot.revoked = false;
    revived.snapshot.generation += 1;
    assert_eq!(
        receiver.finish(&mut engine, revived, 7),
        Err(Error::Conflict)
    );
    assert_eq!(engine.enrollment(), Some(retired));
    assert!(receiver.current(7).unwrap().revoked);
}
