use super::{Fixture, control, enrollment_request, enrollment_response, fixture, grant};
use canic::auth::issuer::{self, Issue, Lookup, Refusal, Renew, Status};
use canic_auth_witness::{
    IssuerObservation, KEY, LIMITS, OBSERVE, enrollment,
    protocol::{
        EstablishSession, SessionInput,
        model::{DelegatedRoleGrant, PortableProof},
    },
};
use ic_auth::{claims::decode_claims, verification::browser_principal};
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
        grants: ic_wire::List::new(vec![DelegatedRoleGrant {
            target: ic_wire::Text::new("user".into()).unwrap(),
            scopes: ic_wire::List::new(vec![ic_wire::Text::new("read".into()).unwrap()]).unwrap(),
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
    ic_wire::encode(
        input,
        ic_wire::Encoding::Frame {
            method: id,
            direction: ic_wire::Direction::Request,
        },
        issuer::REQUEST_LIMITS,
    )
    .unwrap()
}
pub(super) fn response(id: u64, bytes: &[u8]) -> Result<Status, Refusal> {
    ic_wire::decode(
        bytes,
        ic_wire::Encoding::Frame {
            method: id,
            direction: ic_wire::Direction::Response,
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
        issuer::ISSUE => "issue_portable_credential",
        issuer::RENEW => "renew_portable_credential",
        issuer::RECONCILE => "reconcile_portable_credential",
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
    VerifyingKey::from_bytes(&KEY)
        .unwrap()
        .verify_strict(
            &Sha256::digest(credential.claims.as_slice()),
            &Signature::from_slice(signature.as_slice()).unwrap(),
        )
        .unwrap();
    let claims = decode_claims(credential.claims.as_slice()).unwrap();
    assert_eq!(claims.admission_expires_at_ns, *admission_deadline_ns);
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
            "configure_auth",
            enrollment_request(enrollment::CONFIGURE, &approved),
        )
        .await
        .unwrap();
    assert_eq!(
        enrollment_response::<Result<(), enrollment::Error>>(enrollment::CONFIGURE, &bytes),
        Ok(())
    );
    assert!(control(pic, f.target, f.installer, 1).await);
    for wrong in [caller, f.installer, CanisterId::anonymous()] {
        assert_eq!(
            call(&f, wrong, issuer::ISSUE, &input).await,
            Err(Refusal::Denied)
        );
    }
    let mut denied = input.clone();
    denied.grants = ic_wire::List::new(vec![DelegatedRoleGrant {
        target: ic_wire::Text::new("admin".into()).unwrap(),
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
            "issue_portable_credential",
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
            "issue_portable_credential",
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
        scopes: ic_wire::List::new(input.grants.as_slice()[0].scopes.as_slice().to_vec()).unwrap(),
        ttl_secs: Some(30),
    };
    let bytes = pic
        .update_call(
            f.target,
            caller,
            "establish_portable_session",
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
                "issue_portable_credential",
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
            "issue_portable_credential",
            issuer::ISSUE,
            frame(issuer::ISSUE, &input),
        ),
        (
            "renew_portable_credential",
            issuer::RENEW,
            frame(issuer::RENEW, &renew(&input, 1)),
        ),
        (
            "reconcile_portable_credential",
            issuer::RECONCILE,
            frame(issuer::RECONCILE, &lookup(&input)),
        ),
    ] {
        let mut trailing = bytes.clone();
        trailing.push(0);
        let mut oversized = bytes.clone();
        oversized.resize(issuer::REQUEST_LIMITS.bytes + 1, 0);
        let wrong = ic_wire::encode(
            &(),
            ic_wire::Encoding::Frame {
                method: id,
                direction: ic_wire::Direction::Response,
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
async fn issuer_expired_entropy_completion_cannot_restart_the_intent() {
    let f = fixture(false).await;
    let pic = f.owner.client();
    let input = request(1);
    let pending = pic
        .submit_call(
            f.target,
            account(),
            "issue_portable_credential",
            frame(issuer::ISSUE, &input),
        )
        .await
        .unwrap();
    pic.tick().await;
    assert!(pic.ingress_status(pending.clone()).await.is_none());
    assert_eq!(observe(&f).await.operations, 1);
    pic.advance_time(Duration::from_secs(31)).await;
    assert_eq!(
        response(issuer::ISSUE, &pic.await_call(pending).await.unwrap()),
        Err(Refusal::Expired)
    );
    let original = observe(&f).await;
    assert_eq!(original.signatures, 0);
    assert_eq!(
        call(&f, account(), issuer::RECONCILE, &lookup(&input)).await,
        Ok(Status::Unresolved)
    );
    assert_eq!(
        call(&f, account(), issuer::ISSUE, &input).await,
        Err(Refusal::Expired)
    );
    assert_eq!(observe(&f).await, original);
    f.close().await;
}
