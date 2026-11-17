use super::{control, enrollment_request, enrollment_response, fixture, grant, observe, snapshot};
use canic_auth_witness::{LIMITS, OBSERVE, RefreshObservation, enrollment};
use ic_testkit::pocket_ic::{CanisterId, Time, nonblocking::PocketIc};
use std::time::Duration;

async fn refresh_state(pic: &PocketIc, target: CanisterId) -> RefreshObservation {
    let bytes = pic
        .query_call(target, CanisterId::anonymous(), "observe_refresh", vec![])
        .await
        .unwrap();
    ic_wire::decode(
        &bytes,
        ic_wire::Encoding::Frame {
            method: OBSERVE,
            direction: ic_wire::Direction::Response,
        },
        LIMITS,
    )
    .unwrap()
}

async fn configure(f: &super::Fixture) -> enrollment::Grant {
    let pic = f.owner.client();
    let grant = grant(f.target, pic.get_time().await.as_nanos_since_unix_epoch());
    let bytes = pic
        .update_call(
            f.root,
            f.installer,
            "configure_auth",
            enrollment_request(enrollment::CONFIGURE, &grant),
        )
        .await
        .unwrap();
    assert_eq!(
        enrollment_response::<Result<(), enrollment::Error>>(enrollment::CONFIGURE, &bytes),
        Ok(())
    );
    grant
}

#[tokio::test(flavor = "current_thread")]
async fn held_authenticated_replies_keep_receiver_bindings_and_revocation_isolated() {
    let f = fixture(false).await;
    configure(&f).await;
    let pic = f.owner.client();
    let second_root = pic
        .create_canister_with_settings(Some(f.installer), None)
        .await;
    pic.install_canister(second_root, f.root_wasm.clone(), vec![], Some(f.installer))
        .await;
    let second = pic
        .create_canister_with_settings(Some(f.installer), None)
        .await;
    pic.install_canister(
        second,
        f.wasm.clone(),
        second_root.as_slice().to_vec(),
        Some(f.installer),
    )
    .await;
    let second_grant = grant(second, pic.get_time().await.as_nanos_since_unix_epoch());
    for (method, id) in [
        ("configure_auth", enrollment::CONFIGURE),
        ("revoke_auth", enrollment::REVOKE),
    ] {
        let bytes = pic
            .update_call(
                second_root,
                f.installer,
                method,
                enrollment_request(id, &second_grant),
            )
            .await
            .unwrap();
        assert_eq!(
            enrollment_response::<Result<(), enrollment::Error>>(id, &bytes),
            Ok(())
        );
    }

    let (gate, mut held) = held_gate(pic).await;
    for receiver in [f.target, second] {
        assert_eq!(
            pic.update_call(
                receiver,
                CanisterId::anonymous(),
                "set_refresh_barrier",
                gate.as_slice().to_vec()
            )
            .await
            .unwrap(),
            [0]
        );
        assert_eq!(
            pic.update_call(
                receiver,
                f.installer,
                "set_refresh_barrier",
                gate.as_slice().to_vec()
            )
            .await
            .unwrap(),
            [1]
        );
    }
    let first_call = pic
        .submit_call(f.target, f.installer, "control", vec![1])
        .await
        .unwrap();
    for _ in 0..30 {
        if held.pending(pic).await.unwrap().len() == 1 {
            break;
        }
        pic.tick().await;
    }
    assert_eq!(held.pending(pic).await.unwrap().len(), 1);
    let first_pending = refresh_state(pic, f.target).await;
    assert_eq!(
        (
            first_pending.reserved,
            first_pending.dispatched,
            first_pending.fetched,
            first_pending.installed
        ),
        (1, 1, 1, 0)
    );
    let second_call = pic
        .submit_call(second, f.installer, "control", vec![1])
        .await
        .unwrap();
    for _ in 0..30 {
        if held.pending(pic).await.unwrap().len() == 2 {
            break;
        }
        pic.tick().await;
    }
    let tokens = held.pending(pic).await.unwrap();
    assert_eq!(tokens.len(), 2);
    let second_pending = refresh_state(pic, second).await;
    assert_eq!(
        (
            second_pending.reserved,
            second_pending.dispatched,
            second_pending.fetched,
            second_pending.installed
        ),
        (1, 1, 1, 0)
    );
    assert_eq!(observe(pic, f.target).await.enrollment, None);
    assert_eq!(observe(pic, second).await.enrollment, None);
    assert!(pic.ingress_status(first_call.clone()).await.is_none());
    assert!(pic.ingress_status(second_call.clone()).await.is_none());
    // Both responses came from their real Root, with real captured challenges.
    // Testkit only delays the publication continuations. No time jump, forged
    // authenticated marker or replaced authority supplies a result here.
    held.release(pic, tokens.iter().find(|t| t.slot() == 1).unwrap())
        .await
        .unwrap();
    assert_eq!(pic.await_call(second_call).await.unwrap(), [1]);
    let revoked = snapshot(pic, second).await;
    assert!(revoked.revoked);
    assert_eq!(revoked.grant, second_grant);
    assert_eq!(
        revoked.request.receipt_deadline_ns,
        second_pending.deadline_ns
    );
    assert_eq!(observe(pic, f.target).await.enrollment, None);
    assert_eq!(refresh_state(pic, f.target).await, first_pending);
    held.release(pic, tokens.iter().find(|t| t.slot() == 0).unwrap())
        .await
        .unwrap();
    assert_eq!(pic.await_call(first_call).await.unwrap(), [1]);
    let active = snapshot(pic, f.target).await;
    assert!(!active.revoked);
    assert_eq!(active.grant.receiver.as_slice(), f.target.as_slice());
    assert_eq!(
        active.request.receipt_deadline_ns,
        first_pending.deadline_ns
    );
    assert_ne!(active.request.challenge, revoked.request.challenge);
    assert_eq!(snapshot(pic, second).await, revoked);
    assert_eq!(refresh_state(pic, f.target).await.installed, 1);
    assert_eq!(refresh_state(pic, second).await.installed, 1);
    assert_eq!(observe(pic, f.target).await.writes, 0);
    assert_eq!(observe(pic, second).await.writes, 0);
    assert!(held.pending(pic).await.unwrap().is_empty());
    f.close().await;
}

#[tokio::test(flavor = "current_thread")]
async fn overlapping_refreshes_reserve_and_dispatch_only_once() {
    let f = fixture(false).await;
    configure(&f).await;
    let pic = f.owner.client();
    let first = pic
        .submit_call(f.target, f.installer, "control", vec![1])
        .await
        .unwrap();
    pic.tick().await;
    let reserved = refresh_state(pic, f.target).await;
    assert_eq!(
        (reserved.reserved, reserved.dispatched, reserved.installed),
        (1, 0, 0)
    );
    assert!(pic.ingress_status(first.clone()).await.is_none());
    let duplicate = pic
        .submit_call(f.target, f.installer, "control", vec![1])
        .await
        .unwrap();
    assert_eq!(pic.await_call(duplicate).await.unwrap(), [0]);
    assert_eq!(pic.await_call(first).await.unwrap(), [1]);
    let completed = refresh_state(pic, f.target).await;
    assert_eq!(
        (
            completed.reserved,
            completed.dispatched,
            completed.installed
        ),
        (1, 1, 1)
    );
    assert_eq!(completed.deadline_ns, reserved.deadline_ns);
    let retained = snapshot(pic, f.target).await;
    assert_eq!(retained.request.receipt_deadline_ns, reserved.deadline_ns);
    assert_eq!(
        observe(pic, f.target).await.enrollment,
        Some((retained.revision, retained.generation))
    );
    f.close().await;
}

#[tokio::test(flavor = "current_thread")]
async fn expiry_during_entropy_or_snapshot_cannot_publish_authority() {
    // Suspend real IC calls at each await boundary. No injected snapshots,
    // mocked transport or production failure switches supply the response.
    for after_dispatch in [false, true] {
        let f = fixture(true).await;
        configure(&f).await;
        let pic = f.owner.client();
        let previous = if after_dispatch {
            assert!(control(pic, f.target, f.installer, 1).await);
            Some(snapshot(pic, f.target).await)
        } else {
            None
        };
        let initial = refresh_state(pic, f.target).await;
        let original_state = observe(pic, f.target).await;
        let pending = pic
            .submit_call(f.target, f.installer, "control", vec![1])
            .await
            .unwrap();
        pic.tick().await;
        let reserved = refresh_state(pic, f.target).await;
        assert_eq!(reserved.reserved, initial.reserved + 1);
        if after_dispatch {
            let mut dispatched = false;
            for _ in 0..10 {
                let state = refresh_state(pic, f.target).await;
                if state.dispatched == initial.dispatched + 1 {
                    assert_eq!(state.installed, initial.installed);
                    dispatched = true;
                    break;
                }
                pic.tick().await;
            }
            assert!(
                dispatched,
                "direct Root call did not reach its commit boundary"
            );
        }
        assert!(pic.ingress_status(pending.clone()).await.is_none());
        pic.set_time(Time::from_nanos_since_unix_epoch(reserved.deadline_ns))
            .await;
        assert_eq!(pic.await_call(pending).await.unwrap(), [0]);
        let expired = refresh_state(pic, f.target).await;
        assert_eq!(
            (expired.reserved, expired.dispatched, expired.installed),
            (
                initial.reserved + 1,
                initial.dispatched + u32::from(after_dispatch),
                initial.installed
            )
        );
        assert_eq!(expired.deadline_ns, reserved.deadline_ns);
        let state = observe(pic, f.target).await;
        assert_eq!(state, original_state);
        if let Some(previous) = previous {
            assert_eq!(snapshot(pic, f.target).await, previous);
        }

        // Only an explicit later admission gets another entropy/call attempt.
        assert!(control(pic, f.target, f.installer, 1).await);
        let recovered = refresh_state(pic, f.target).await;
        assert_eq!(
            (
                recovered.reserved,
                recovered.dispatched,
                recovered.installed
            ),
            (
                initial.reserved + 2,
                initial.dispatched + u32::from(after_dispatch) + 1,
                initial.installed + 1
            )
        );
        assert!(recovered.deadline_ns > expired.deadline_ns);
        assert_eq!(
            snapshot(pic, f.target).await.request.receipt_deadline_ns,
            recovered.deadline_ns
        );
        f.close().await;
    }
}

#[tokio::test(flavor = "current_thread")]
async fn authority_loss_cannot_renew_snapshot_or_session_deadlines() {
    use canic_auth_witness::protocol::{
        EstablishSession, ReadSession,
        model::{AdmissionRefusal, SessionReadRefusal},
    };

    for upgrade in [false, true] {
        let f = fixture(false).await;
        configure(&f).await;
        let pic = f.owner.client();
        assert!(control(pic, f.target, f.installer, 1).await);
        let retained = snapshot(pic, f.target).await;
        // Replenish actual verification credit before admitting a live session.
        pic.advance_time(Duration::from_secs(20)).await;
        let (browser, input) = super::input(pic.get_time().await.as_nanos_since_unix_epoch());
        let frame = EstablishSession::encode_request(&input).unwrap();
        let bytes = pic
            .update_call(
                f.target,
                browser,
                "establish_portable_session",
                frame.clone(),
            )
            .await
            .unwrap();
        let session = EstablishSession::decode_response(&bytes).unwrap().unwrap();
        assert_eq!(session.expires_at_ns, retained.expires_at_ns);
        let original = observe(pic, f.target).await;
        let initial = refresh_state(pic, f.target).await;

        if upgrade {
            canic_qualification::upgrade_stopped(
                pic,
                f.root,
                f.root_wasm.clone(),
                vec![],
                Some(f.installer),
            )
            .await
            .unwrap();
        } else {
            pic.stop_canister(f.root, Some(f.installer)).await.unwrap();
        }
        // Exercise both an actual call rejection and Root's closed application
        // response. Neither can replace the previously installed authority.
        assert!(!control(pic, f.target, f.installer, 1).await);
        let failed = refresh_state(pic, f.target).await;
        assert_eq!(
            (failed.reserved, failed.dispatched, failed.installed),
            (
                initial.reserved + 1,
                initial.dispatched + 1,
                initial.installed
            )
        );
        assert!(!control(pic, f.target, f.installer, 1).await);
        assert_eq!(refresh_state(pic, f.target).await, failed);
        assert_eq!(snapshot(pic, f.target).await, retained);
        assert_eq!(observe(pic, f.target).await, original);

        // After the failed attempt expires, a new explicit request still cannot
        // extend the last good snapshot or the session captured from it.
        pic.set_time(Time::from_nanos_since_unix_epoch(failed.deadline_ns))
            .await;
        assert!(!control(pic, f.target, f.installer, 1).await);
        let second = refresh_state(pic, f.target).await;
        assert_eq!(second.reserved, failed.reserved + 1);
        assert_eq!(second.dispatched, failed.dispatched + 1);
        assert_eq!(second.installed, initial.installed);
        assert_eq!(snapshot(pic, f.target).await, retained);
        let read = ReadSession::encode_request(&()).unwrap();
        pic.set_time(Time::from_nanos_since_unix_epoch(
            retained.expires_at_ns - 1,
        ))
        .await;
        // Queries use the last executed round's time, not the simulator's next
        // round setting. Commit a round before checking each query boundary.
        pic.tick().await;
        let bytes = pic
            .query_call(f.target, browser, "portable_session_read", read.clone())
            .await
            .unwrap();
        assert_eq!(ReadSession::decode_response(&bytes).unwrap(), Ok(session));
        for now in [retained.expires_at_ns, retained.expires_at_ns + 1] {
            pic.set_time(Time::from_nanos_since_unix_epoch(now)).await;
            pic.tick().await;
            let bytes = pic
                .query_call(f.target, browser, "portable_session_read", read.clone())
                .await
                .unwrap();
            assert_eq!(
                ReadSession::decode_response(&bytes).unwrap(),
                Err(SessionReadRefusal::Unavailable)
            );
        }
        let bytes = pic
            .update_call(f.target, browser, "establish_portable_session", frame)
            .await
            .unwrap();
        assert_eq!(
            EstablishSession::decode_response(&bytes).unwrap(),
            Err(AdmissionRefusal::Unavailable)
        );
        let state = observe(pic, f.target).await;
        assert_eq!(state.writes, original.writes);
        assert_eq!(state.usage, original.usage);
        assert_eq!(state.enrollment, original.enrollment);
        assert_eq!(refresh_state(pic, f.target).await, second);
        f.close().await;
    }
}

async fn held_gate(pic: &PocketIc) -> (CanisterId, ic_testkit::held::Fixture) {
    use ic_testkit::held::{Catalog, Fixture as Held, Limits, Reply};
    let gate = pic.create_canister().await;
    let catalog = Catalog::new(
        &[Reply {
            method: "hold",
            bytes: &[],
        }; 2],
        Limits {
            slots: 2,
            name_bytes: 8,
            reply_bytes: 0,
        },
    )
    .unwrap();
    let held = Held::new(
        pic,
        gate.to_text().parse().unwrap(),
        &catalog,
        100_000_000_000,
        8192,
    )
    .await
    .unwrap();
    pic.add_cycles(gate, 1_000_000_000_000).await;
    pic.install_canister(gate, held.wasm().to_vec(), vec![], None)
        .await;
    (gate, held)
}

async fn pending(
    held: &ic_testkit::held::Fixture,
    pic: &PocketIc,
    count: usize,
) -> Vec<ic_testkit::held::Request> {
    for _ in 0..30 {
        let tokens = held.pending(pic).await.unwrap();
        if tokens.len() == count {
            return tokens;
        }
        pic.tick().await;
    }
    panic!("held reply count did not reach {count}");
}

#[tokio::test(flavor = "current_thread")]
async fn expired_held_response_cannot_clear_newer_pending_or_committed_revocation() {
    for old_first in [true, false] {
        let f = fixture(false).await;
        let grant = configure(&f).await;
        let pic = f.owner.client();
        let (gate, mut held) = held_gate(pic).await;
        assert_eq!(
            pic.update_call(
                f.target,
                f.installer,
                "set_refresh_barrier",
                gate.as_slice().to_vec()
            )
            .await
            .unwrap(),
            [1]
        );
        let old_call = pic
            .submit_call(f.target, f.installer, "control", vec![1])
            .await
            .unwrap();
        let old_token = pending(&held, pic, 1).await.remove(0);
        let old = refresh_state(pic, f.target).await;
        assert_eq!(
            (old.reserved, old.dispatched, old.fetched, old.installed),
            (1, 1, 1, 0)
        );
        assert!(!control(pic, f.target, f.installer, 1).await);
        assert_eq!(refresh_state(pic, f.target).await, old);
        let before = pic.get_time().await.as_nanos_since_unix_epoch();
        let after = held
            .advance_time(pic, Duration::from_nanos(old.deadline_ns - before + 1))
            .await
            .unwrap();
        assert!(after.as_nanos_since_unix_epoch() > old.deadline_ns);
        assert_eq!(held.pending(pic).await.unwrap().len(), 1);
        let bytes = pic
            .update_call(
                f.root,
                f.installer,
                "revoke_auth",
                enrollment_request(enrollment::REVOKE, &grant),
            )
            .await
            .unwrap();
        assert_eq!(
            enrollment_response::<Result<(), enrollment::Error>>(enrollment::REVOKE, &bytes),
            Ok(())
        );
        let new_call = pic
            .submit_call(f.target, f.installer, "control", vec![1])
            .await
            .unwrap();
        let new_token = pending(&held, pic, 2)
            .await
            .into_iter()
            .find(|token| token.slot() == 1)
            .unwrap();
        let newer = refresh_state(pic, f.target).await;
        assert_eq!(
            (
                newer.reserved,
                newer.dispatched,
                newer.fetched,
                newer.installed
            ),
            (2, 2, 2, 0)
        );
        assert!(newer.deadline_ns > old.deadline_ns);
        assert_eq!(observe(pic, f.target).await.enrollment, None);
        if old_first {
            held.release(pic, &old_token).await.unwrap();
            assert_eq!(pic.await_call(old_call.clone()).await.unwrap(), [0]);
            assert_eq!(refresh_state(pic, f.target).await, newer);
            assert_eq!(observe(pic, f.target).await.enrollment, None);
            assert!(!control(pic, f.target, f.installer, 1).await);
            assert_eq!(refresh_state(pic, f.target).await, newer);
        }
        held.release(pic, &new_token).await.unwrap();
        assert_eq!(pic.await_call(new_call).await.unwrap(), [1]);
        let committed = snapshot(pic, f.target).await;
        assert!(committed.revoked);
        assert_eq!(committed.grant, grant);
        assert_eq!(committed.request.receipt_deadline_ns, newer.deadline_ns);
        let metadata = observe(pic, f.target).await;
        let counters = refresh_state(pic, f.target).await;
        assert_eq!(counters.installed, 1);
        if !old_first {
            held.release(pic, &old_token).await.unwrap();
            assert_eq!(pic.await_call(old_call).await.unwrap(), [0]);
        }
        assert_eq!(snapshot(pic, f.target).await, committed);
        assert_eq!(observe(pic, f.target).await, metadata);
        assert_eq!(refresh_state(pic, f.target).await, counters);
        assert_eq!(metadata.writes, 0);
        assert!(held.pending(pic).await.unwrap().is_empty());
        f.close().await;
    }
}
