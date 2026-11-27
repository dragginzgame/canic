use super::{
    Fixture, control, enrollment_request, enrollment_response, fixture, grant, receiver_refresh,
};
use canic::auth::{
    issuer::{self, Issue, Lookup, Refusal, Renew, Status},
    protocol::wire as auth_wire,
};
use canic_auth_witness::{
    IssuerObservation, KEY, LIMITS, OBSERVE, enrollment,
    protocol::{
        EstablishSession, SessionInput,
        model::{DelegatedRoleGrant, PortableProof},
    },
};
use ic_auth::{
    claims::decode_claims,
    verification::{Ed25519Key, browser_principal},
};
use ic_auth_protocol_types::Principal;
use ic_platform_crypto::ed25519::{Signature, SigningKey, VerifyingKey};
use ic_testkit::pocket_ic::{CanisterId, Time};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::time::Duration;

pub(super) fn account() -> CanisterId {
    CanisterId::self_authenticating(b"canic-auth-account")
}

pub(super) fn principal(id: CanisterId) -> Principal {
    Principal::from_slice(id.as_slice())
}

pub(super) fn request(id: u8) -> Issue {
    Issue {
        epoch: 1,
        operation_id: [id; 32],
        login_epoch: 1,
        login_id: [1; 32],
        browser_public_key: SigningKey::from_bytes(&[9; 32]).verifying_key().to_bytes(),
        grants: auth_wire::List::new(vec![DelegatedRoleGrant {
            target: auth_wire::Text::new("user".into()).unwrap(),
            scopes: auth_wire::List::new(vec![auth_wire::Text::new("read".into()).unwrap()])
                .unwrap(),
        }])
        .unwrap(),
    }
}

pub(super) fn browser(input: &Issue) -> CanisterId {
    CanisterId::from_slice(browser_principal(&input.browser_public_key).as_slice())
}

pub(super) fn lookup(input: &Issue) -> Lookup {
    Lookup::Issue {
        epoch: input.epoch,
        operation_id: input.operation_id,
    }
}

pub(super) fn renew(input: &Issue, sequence: u64) -> Renew {
    Renew {
        account: principal(account()),
        login_epoch: input.login_epoch,
        login_id: input.login_id,
        sequence,
        grants: input.grants.clone(),
    }
}

pub(super) fn frame(id: u64, input: &impl Serialize) -> Vec<u8> {
    auth_wire::encode(
        input,
        auth_wire::Encoding::Frame {
            method: id,
            direction: auth_wire::Direction::Request,
        },
        issuer::REQUEST_LIMITS,
    )
    .unwrap()
}

pub(super) fn response(id: u64, bytes: &[u8]) -> Result<Status, Refusal> {
    auth_wire::decode(
        bytes,
        auth_wire::Encoding::Frame {
            method: id,
            direction: auth_wire::Direction::Response,
        },
        issuer::RESPONSE_LIMITS,
    )
    .unwrap()
}

pub(super) async fn call(
    f: &Fixture,
    caller: CanisterId,
    id: u64,
    input: &impl Serialize,
) -> Result<Status, Refusal> {
    let name = match id {
        issuer::ISSUE => "canic_issue_portable_credential",
        issuer::RENEW => "canic_renew_portable_credential",
        issuer::RECONCILE => "canic_reconcile_portable_credential",
        _ => unreachable!(),
    };
    let bytes = f
        .owner
        .client()
        .update_call(f.target, caller, name, frame(id, input))
        .await
        .unwrap();
    response(id, &bytes)
}

pub(super) async fn observe(f: &Fixture) -> IssuerObservation {
    let bytes = f
        .owner
        .client()
        .query_call(f.target, CanisterId::anonymous(), "observe_issuer", vec![])
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

pub(super) fn claims(value: &Status) -> ic_auth_protocol_types::PortableClaims {
    verified_claims(value, &KEY)
}

fn verified_claims(
    value: &Status,
    verification_key: &[u8; 32],
) -> ic_auth_protocol_types::PortableClaims {
    let Status::Issued {
        credential,
        admission_deadline_ns,
    } = value
    else {
        panic!("issued credential required");
    };
    let PortableProof::Ed25519(signature) = &credential.proof else {
        panic!("Ed25519 proof required");
    };
    VerifyingKey::from_bytes(verification_key)
        .unwrap()
        .verify_strict(
            &Sha256::digest(credential.claims.as_slice()),
            &Signature::from_slice(signature.as_slice()).unwrap(),
        )
        .unwrap();
    let claims = decode_claims(credential.claims.as_slice()).unwrap();
    assert_eq!(claims.admission_expires_at_ns, *admission_deadline_ns);
    assert_eq!(
        claims.key_binding_id,
        Ed25519Key::new(*verification_key).unwrap().binding()
    );
    claims
}

#[tokio::test(flavor = "current_thread")]
async fn issuer_real_calls_bind_accounts_renewals_and_exact_results() {
    let f = fixture(false).await;
    let pic = f.owner.client();
    let input = request(1);
    let caller = browser(&input);
    // Enroll the selected issuer before issuing credentials under that grant.
    let now = pic.get_time().await.as_nanos_since_unix_epoch();
    let mut approved = grant(f.target, now);
    approved.issuer = principal(f.target);
    let bytes = pic
        .update_call(
            f.root,
            f.installer,
            "grant_auth_access",
            enrollment_request(enrollment::CONFIGURE, &approved),
        )
        .await
        .unwrap();
    assert_eq!(
        enrollment_response::<Result<(), enrollment::Error>>(enrollment::CONFIGURE, &bytes),
        Ok(())
    );
    assert!(receiver_refresh(pic, f.target, f.installer).await.is_ok());
    for wrong in [caller, f.installer, CanisterId::anonymous()] {
        assert_eq!(
            call(&f, wrong, issuer::ISSUE, &input).await,
            Err(Refusal::Denied)
        );
    }
    let mut denied = input.clone();
    denied.grants = auth_wire::List::new(vec![DelegatedRoleGrant {
        target: auth_wire::Text::new("admin".into()).unwrap(),
        scopes: input.grants.as_slice()[0].scopes.clone(),
    }])
    .unwrap();
    assert_eq!(
        call(&f, account(), issuer::ISSUE, &denied).await,
        Err(Refusal::Denied)
    );
    assert_eq!(observe(&f).await.operations, 0);

    let first = pic
        .submit_call(
            f.target,
            account(),
            "canic_issue_portable_credential",
            frame(issuer::ISSUE, &input),
        )
        .await
        .unwrap();
    pic.tick().await;
    assert!(pic.ingress_status(first.clone()).await.is_none());
    assert_eq!(observe(&f).await.operations, 1);
    assert_eq!(observe(&f).await.signatures, 0);
    assert_eq!(
        call(&f, account(), issuer::RECONCILE, &lookup(&input)).await,
        Ok(Status::Unresolved)
    );
    // A concurrent duplicate has no second entropy/signing permission.
    let duplicate = pic
        .submit_call(
            f.target,
            account(),
            "canic_issue_portable_credential",
            frame(issuer::ISSUE, &input),
        )
        .await
        .unwrap();
    let original = response(issuer::ISSUE, &pic.await_call(first).await.unwrap()).unwrap();
    let repeated = response(issuer::ISSUE, &pic.await_call(duplicate).await.unwrap()).unwrap();
    assert!(repeated == Status::Unresolved || repeated == original);
    assert_eq!(
        call(&f, account(), issuer::ISSUE, &input).await,
        Ok(original.clone())
    );
    // Forgetting the delivered response uses only retained lookup, never another issue.
    assert_eq!(
        call(&f, account(), issuer::RECONCILE, &lookup(&input)).await,
        Ok(original.clone())
    );
    let first_claims = claims(&original);
    assert_eq!(first_claims.account_subject, principal(account()));
    assert_eq!(first_claims.issuer, principal(f.target));
    assert_eq!(first_claims.browser_public_key, input.browser_public_key);
    assert_eq!(observe(&f).await.signatures, 1);
    let mut changed = input.clone();
    changed.browser_public_key = SigningKey::from_bytes(&[8; 32]).verifying_key().to_bytes();
    assert_eq!(
        call(&f, account(), issuer::ISSUE, &changed).await,
        Err(Refusal::Conflict)
    );
    assert_eq!(
        call(&f, caller, issuer::RECONCILE, &lookup(&input)).await,
        Err(Refusal::Denied)
    );

    pic.advance_time(Duration::from_secs(3)).await;
    let Status::Issued { credential, .. } = original.clone() else {
        unreachable!()
    };
    let admission = SessionInput {
        credential,
        scopes: auth_wire::List::new(input.grants.as_slice()[0].scopes.as_slice().to_vec())
            .unwrap(),
        ttl_secs: Some(30),
    };
    let bytes = pic
        .update_call(
            f.target,
            caller,
            "canic_establish_portable_session",
            EstablishSession::encode_request(&admission).unwrap(),
        )
        .await
        .unwrap();
    let session = EstablishSession::decode_response(&bytes).unwrap().unwrap();
    assert_eq!(
        (session.subject, session.issuer, session.caller),
        (principal(account()), principal(f.target), principal(caller))
    );

    let next = renew(&input, 1);
    assert_eq!(
        call(&f, account(), issuer::RENEW, &next).await,
        Err(Refusal::Denied)
    );
    let mut forged = next.clone();
    forged.account = principal(f.installer);
    assert_eq!(
        call(&f, caller, issuer::RENEW, &forged).await,
        Err(Refusal::Denied)
    );
    forged = next.clone();
    forged.grants = denied.grants;
    assert_eq!(
        call(&f, caller, issuer::RENEW, &forged).await,
        Err(Refusal::Denied)
    );
    let renewed = call(&f, caller, issuer::RENEW, &next).await.unwrap();
    let next_claims = claims(&renewed);
    assert_eq!(
        next_claims.login_deadline_ns,
        first_claims.login_deadline_ns
    );
    assert_eq!(
        next_claims.login_started_at_ns,
        first_claims.login_started_at_ns
    );
    assert_eq!(next_claims.grants, first_claims.grants);
    assert!(next_claims.issued_at_ns > first_claims.issued_at_ns);
    assert_eq!(
        call(&f, caller, issuer::RENEW, &next).await,
        Ok(renewed.clone())
    );
    assert_eq!(
        call(&f, caller, issuer::RECONCILE, &Lookup::Renew(next.clone())).await,
        Ok(renewed.clone())
    );
    assert_eq!(
        call(
            &f,
            account(),
            issuer::RECONCILE,
            &Lookup::Renew(next.clone())
        )
        .await,
        Err(Refusal::Denied)
    );
    assert_eq!(observe(&f).await.signatures, 2);
    assert!(control(pic, f.target, f.installer, 8).await);
    assert_eq!(
        call(&f, caller, issuer::RENEW, &renew(&input, 2)).await,
        Err(Refusal::Denied)
    );
    assert_eq!(
        call(&f, caller, issuer::RECONCILE, &Lookup::Renew(next)).await,
        Ok(renewed)
    );
    assert_eq!(observe(&f).await.signatures, 2);
    f.close().await;
}

#[tokio::test(flavor = "current_thread")]
async fn issuer_callback_failures_and_changed_permission_stay_unresolved() {
    for trap in [true, false] {
        let f = fixture(false).await;
        let pic = f.owner.client();
        let input = request(1);
        if trap {
            assert!(control(pic, f.target, f.installer, 6).await);
        }
        let pending = pic
            .submit_call(
                f.target,
                account(),
                "canic_issue_portable_credential",
                frame(issuer::ISSUE, &input),
            )
            .await
            .unwrap();
        pic.tick().await;
        assert!(pic.ingress_status(pending.clone()).await.is_none());
        if !trap {
            assert!(control(pic, f.target, f.installer, 8).await);
        }
        let result = pic.await_call(pending).await;
        if trap {
            assert!(result.is_err());
            assert!(control(pic, f.target, f.installer, 7).await);
            assert_eq!(
                call(&f, account(), issuer::ISSUE, &input).await,
                Ok(Status::Unresolved)
            );
        } else {
            assert_eq!(
                response(issuer::ISSUE, &result.unwrap()),
                Err(Refusal::Denied)
            );
        }
        assert_eq!(
            call(&f, account(), issuer::RECONCILE, &lookup(&input)).await,
            Ok(Status::Unresolved)
        );
        assert_eq!(observe(&f).await.signatures, 0);
        assert_eq!(observe(&f).await.operations, 1);
        canic_qualification::upgrade_stopped(
            pic,
            f.target,
            f.wasm.clone(),
            vec![],
            Some(f.installer),
        )
        .await
        .unwrap();
        assert_eq!(
            call(&f, account(), issuer::ISSUE, &input).await,
            Err(Refusal::Unavailable)
        );
        assert_eq!(
            call(&f, account(), issuer::RECONCILE, &lookup(&input)).await,
            Err(Refusal::Unavailable)
        );
        assert!(!observe(&f).await.present);
        f.close().await;
    }
}

#[tokio::test(flavor = "current_thread")]
async fn issuer_bounds_expiry_and_pruning_never_create_replacement_results() {
    let f = fixture(false).await;
    let pic = f.owner.client();
    let input = request(1);
    let before = observe(&f).await;
    for (method, id, bytes) in [
        (
            "canic_issue_portable_credential",
            issuer::ISSUE,
            frame(issuer::ISSUE, &input),
        ),
        (
            "canic_renew_portable_credential",
            issuer::RENEW,
            frame(issuer::RENEW, &renew(&input, 1)),
        ),
        (
            "canic_reconcile_portable_credential",
            issuer::RECONCILE,
            frame(issuer::RECONCILE, &lookup(&input)),
        ),
    ] {
        let mut trailing = bytes.clone();
        trailing.push(0);
        let mut oversized = bytes.clone();
        oversized.resize(issuer::REQUEST_LIMITS.bytes + 1, 0);
        let wrong = auth_wire::encode(
            &(),
            auth_wire::Encoding::Frame {
                method: id,
                direction: auth_wire::Direction::Response,
            },
            issuer::REQUEST_LIMITS,
        )
        .unwrap();
        for invalid in [vec![], trailing, oversized, wrong] {
            assert!(
                pic.update_call(f.target, account(), method, invalid)
                    .await
                    .is_err()
            );
            assert_eq!(observe(&f).await, before);
        }
        assert!(
            pic.query_call(f.target, account(), method, bytes)
                .await
                .is_err()
        );
    }
    let original = call(&f, account(), issuer::ISSUE, &input).await.unwrap();
    let original_claims = claims(&original);
    for id in 2..=4 {
        assert!(matches!(
            call(&f, account(), issuer::ISSUE, &request(id))
                .await
                .unwrap(),
            Status::Issued { .. }
        ));
    }
    assert_eq!(
        call(&f, account(), issuer::ISSUE, &request(5)).await,
        Err(Refusal::Capacity)
    );
    assert_eq!(observe(&f).await.signatures, 4);
    pic.set_time(Time::from_nanos_since_unix_epoch(
        original_claims.admission_expires_at_ns,
    ))
    .await;
    assert_eq!(
        call(&f, account(), issuer::RECONCILE, &lookup(&input)).await,
        Ok(original.clone())
    );
    pic.advance_time(Duration::from_secs(61)).await;
    assert_eq!(
        call(&f, account(), issuer::RECONCILE, &lookup(&input)).await,
        Ok(original)
    );
    assert!(control(pic, f.target, f.installer, 9).await);
    assert_eq!(
        call(&f, account(), issuer::RECONCILE, &lookup(&input)).await,
        Ok(Status::Unresolved)
    );
    assert_eq!(
        call(&f, account(), issuer::ISSUE, &input).await,
        Err(Refusal::Conflict)
    );
    assert_eq!(observe(&f).await.operations, 0);
    assert_eq!(observe(&f).await.signatures, 4);
    f.close().await;
}

#[tokio::test(flavor = "current_thread")]
async fn issuer_entropy_completion_respects_the_original_exclusive_deadline() {
    for distance_ns in [-1_i64, 0, 1] {
        let f = fixture(false).await;
        let pic = f.owner.client();
        let input = request(1);
        let pending = pic
            .submit_call(
                f.target,
                account(),
                "canic_issue_portable_credential",
                frame(issuer::ISSUE, &input),
            )
            .await
            .unwrap();
        pic.tick().await;
        let issued_at = pic.get_time().await.as_nanos_since_unix_epoch();
        assert!(pic.ingress_status(pending.clone()).await.is_none());
        assert_eq!(observe(&f).await.operations, 1);

        let deadline = issued_at + 30_000_000_000;
        let callback_at = deadline.checked_add_signed(distance_ns).unwrap();
        // PocketIC advances one nanosecond for this callback round.
        pic.set_time(Time::from_nanos_since_unix_epoch(callback_at - 1))
            .await;
        let received = response(issuer::ISSUE, &pic.await_call(pending).await.unwrap());
        assert_eq!(
            pic.get_time().await.as_nanos_since_unix_epoch(),
            callback_at
        );
        let retained = if distance_ns < 0 {
            let issued = received.unwrap();
            assert_eq!(claims(&issued).issued_at_ns, issued_at);
            assert_eq!(observe(&f).await.signatures, 1);
            Ok(issued)
        } else {
            assert_eq!(received, Err(Refusal::Expired));
            assert_eq!(observe(&f).await.signatures, 0);
            Ok(Status::Unresolved)
        };
        let original = observe(&f).await;
        assert_eq!(
            call(&f, account(), issuer::RECONCILE, &lookup(&input)).await,
            retained
        );
        let duplicate = call(&f, account(), issuer::ISSUE, &input).await;
        if distance_ns < 0 {
            assert_eq!(duplicate, retained);
        } else {
            assert_eq!(duplicate, Err(Refusal::Expired));
        }
        assert_eq!(observe(&f).await, original);
        f.close().await;
    }
}

#[tokio::test(flavor = "current_thread")]
async fn issuer_rotation_fences_pending_renewal_and_preserves_original_issue() {
    let f = fixture(false).await;
    let pic = f.owner.client();
    let input = request(1);
    let original = call(&f, account(), issuer::ISSUE, &input).await.unwrap();
    claims(&original);
    let caller = browser(&input);
    assert!(!control(pic, f.target, caller, 10).await);

    let renewal = renew(&input, 1);
    let pending = pic
        .submit_call(
            f.target,
            caller,
            "canic_renew_portable_credential",
            frame(issuer::RENEW, &renewal),
        )
        .await
        .unwrap();
    pic.tick().await;
    assert!(pic.ingress_status(pending.clone()).await.is_none());
    let reserved = observe(&f).await;
    assert_eq!((reserved.signatures, reserved.operations), (1, 2));
    assert!(control(pic, f.target, f.installer, 10).await);
    assert_eq!(
        response(issuer::RENEW, &pic.await_call(pending).await.unwrap()),
        Err(Refusal::Denied)
    );
    assert_eq!(
        call(&f, caller, issuer::RENEW, &renewal).await,
        Err(Refusal::Denied)
    );
    assert_eq!(
        call(&f, caller, issuer::RECONCILE, &Lookup::Renew(renewal)).await,
        Ok(Status::Unresolved)
    );
    assert_eq!(
        call(&f, account(), issuer::RECONCILE, &lookup(&input)).await,
        Ok(original)
    );
    assert_eq!(observe(&f).await, reserved);

    let mut fresh = request(2);
    fresh.login_id = [2; 32];
    fresh.browser_public_key = SigningKey::from_bytes(&[11; 32]).verifying_key().to_bytes();
    let issued = call(&f, account(), issuer::ISSUE, &fresh).await.unwrap();
    let rotated_key = SigningKey::from_bytes(&[17; 32]).verifying_key().to_bytes();
    assert_eq!(
        verified_claims(&issued, &rotated_key).browser_public_key,
        fresh.browser_public_key
    );
    let observed = observe(&f).await;
    assert_eq!(
        (observed.signatures, observed.operations, observed.logins),
        (2, 3, 2)
    );
    f.close().await;
}

#[tokio::test(flavor = "current_thread")]
async fn pruning_a_renewal_keeps_the_live_logins_consumed_sequence() {
    let f = fixture(false).await;
    let pic = f.owner.client();
    let input = request(1);
    let caller = browser(&input);
    let original = call(&f, account(), issuer::ISSUE, &input).await.unwrap();
    let first = renew(&input, 1);
    let completed = call(&f, caller, issuer::RENEW, &first).await.unwrap();
    let lookup = Lookup::Renew(first.clone());
    pic.advance_time(Duration::from_secs(121)).await;
    assert_eq!(
        call(&f, caller, issuer::RECONCILE, &lookup).await,
        Ok(completed)
    );
    assert!(control(pic, f.target, f.installer, 9).await);
    assert_eq!(
        call(&f, caller, issuer::RECONCILE, &lookup).await,
        Ok(Status::Unresolved)
    );
    assert_eq!(
        call(&f, caller, issuer::RENEW, &first).await,
        Err(Refusal::Conflict)
    );
    let after = observe(&f).await;
    assert_eq!(
        (after.operations, after.signatures, after.logins),
        (0, 2, 1)
    );
    let next = call(&f, caller, issuer::RENEW, &renew(&input, 2))
        .await
        .unwrap();
    assert_eq!(
        claims(&next).login_deadline_ns,
        claims(&original).login_deadline_ns
    );
    assert_eq!(observe(&f).await.signatures, 3);
    f.close().await;
}
