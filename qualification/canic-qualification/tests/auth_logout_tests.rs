use super::auth_issuer_tests::{
    account, browser, call, claims, frame, lookup, observe, principal, renew, request, response,
};
use super::{Fixture, control, fixture};
use canic::auth::issuer::{self, Issue, Logout, Refusal, Revoked, Status};
use ic_testkit::pocket_ic::{CanisterId, Time};

fn selection(input: &Issue) -> Logout {
    Logout {
        account: principal(account()),
        login_epoch: input.login_epoch,
        login_id: input.login_id,
    }
}

async fn logout(f: &Fixture, caller: CanisterId, input: &Logout) -> Result<Revoked, Refusal> {
    let bytes = f
        .owner
        .client()
        .update_call(
            f.target,
            caller,
            "revoke_portable_login",
            frame(issuer::LOGOUT, input),
        )
        .await
        .unwrap();
    ic_wire::decode(
        &bytes,
        ic_wire::Encoding::Frame {
            method: issuer::LOGOUT,
            direction: ic_wire::Direction::Response,
        },
        issuer::RESPONSE_LIMITS,
    )
    .unwrap()
}

#[tokio::test(flavor = "current_thread")]
async fn logout_binds_browser_preserves_original_results_and_other_logins() {
    let f = fixture(false).await;
    let pic = f.owner.client();
    let input = request(1);
    let original = call(&f, account(), issuer::ISSUE, &input).await.unwrap();
    let original_claims = claims(&original);
    let caller = browser(&input);
    let selected = selection(&input);
    for wrong in [account(), f.installer, CanisterId::anonymous()] {
        assert_eq!(logout(&f, wrong, &selected).await, Err(Refusal::Denied));
    }
    for wrong in [
        Logout {
            account: principal(f.installer),
            ..selected.clone()
        },
        Logout {
            login_epoch: 2,
            ..selected.clone()
        },
        Logout {
            login_id: [99; 32],
            ..selected.clone()
        },
    ] {
        assert_eq!(logout(&f, caller, &wrong).await, Err(Refusal::Denied));
    }
    let mut second = request(2);
    second.login_id = [2; 32];
    second.browser_public_key = ic_platform_crypto::ed25519::SigningKey::from_bytes(&[8; 32])
        .verifying_key()
        .to_bytes();
    call(&f, account(), issuer::ISSUE, &second).await.unwrap();
    let expected = Revoked {
        login_deadline_ns: original_claims.login_deadline_ns,
    };
    assert_eq!(logout(&f, caller, &selected).await, Ok(expected));
    assert_eq!(logout(&f, caller, &selected).await, Ok(expected));
    assert_eq!(
        call(&f, account(), issuer::RECONCILE, &lookup(&input)).await,
        Ok(original.clone())
    );
    assert_eq!(
        call(&f, caller, issuer::RENEW, &renew(&input, 1)).await,
        Err(Refusal::Denied)
    );
    assert_eq!(observe(&f).await.signatures, 2);
    call(&f, browser(&second), issuer::RENEW, &renew(&second, 1))
        .await
        .unwrap();
    assert_eq!(observe(&f).await.signatures, 3);
    pic.set_time(Time::from_nanos_since_unix_epoch(
        original_claims.login_deadline_ns + 1,
    ))
    .await;
    assert_eq!(logout(&f, caller, &selected).await, Ok(expected));
    assert_eq!(
        call(&f, account(), issuer::RECONCILE, &lookup(&input)).await,
        Ok(original)
    );
    assert!(control(pic, f.target, f.installer, 9).await);
    assert_eq!(observe(&f).await.operations, 0);
    // Auth prunes operations first; a second bounded pass retires newly idle logins.
    assert!(control(pic, f.target, f.installer, 9).await);
    // The second device was issued later and remains a separate live login.
    assert_eq!(observe(&f).await.logins, 1);
    assert_eq!(logout(&f, caller, &selected).await, Err(Refusal::Denied));
    f.close().await;
}

#[tokio::test(flavor = "current_thread")]
async fn logout_fences_inflight_initial_and_renewal_completion() {
    for renewal in [false, true] {
        let f = fixture(false).await;
        let pic = f.owner.client();
        let input = request(1);
        let caller = browser(&input);
        let original = if renewal {
            Some(call(&f, account(), issuer::ISSUE, &input).await.unwrap())
        } else {
            None
        };
        let (method, id, actor, bytes) = if renewal {
            (
                "renew_portable_credential",
                issuer::RENEW,
                caller,
                frame(issuer::RENEW, &renew(&input, 1)),
            )
        } else {
            (
                "issue_portable_credential",
                issuer::ISSUE,
                account(),
                frame(issuer::ISSUE, &input),
            )
        };
        let pending = pic
            .submit_call(f.target, actor, method, bytes)
            .await
            .unwrap();
        pic.tick().await;
        assert!(pic.ingress_status(pending.clone()).await.is_none());
        let before = observe(&f).await;
        assert_eq!(before.signatures, u32::from(renewal));
        let revoked = logout(&f, caller, &selection(&input)).await.unwrap();
        assert_eq!(
            response(id, &pic.await_call(pending).await.unwrap()),
            Err(Refusal::Denied)
        );
        assert_eq!(observe(&f).await.signatures, before.signatures);
        let lookup_request = if renewal {
            issuer::Lookup::Renew(renew(&input, 1))
        } else {
            lookup(&input)
        };
        assert_eq!(
            call(&f, actor, issuer::RECONCILE, &lookup_request).await,
            Ok(Status::Unresolved)
        );
        assert_eq!(logout(&f, caller, &selection(&input)).await, Ok(revoked));
        if let Some(original) = original {
            assert_eq!(
                call(&f, account(), issuer::RECONCILE, &lookup(&input)).await,
                Ok(original)
            );
        }
        f.close().await;
    }
}

#[tokio::test(flavor = "current_thread")]
async fn logout_reply_failure_rolls_back_and_lost_state_never_acknowledges() {
    let f = fixture(false).await;
    let pic = f.owner.client();
    let input = request(1);
    call(&f, account(), issuer::ISSUE, &input).await.unwrap();
    let caller = browser(&input);
    let selected = selection(&input);
    for invalid in [
        vec![],
        frame(issuer::RENEW, &selected),
        vec![0; issuer::REQUEST_LIMITS.bytes + 1],
    ] {
        assert!(
            pic.update_call(f.target, caller, "revoke_portable_login", invalid)
                .await
                .is_err()
        );
    }
    assert!(control(pic, f.target, f.installer, 6).await);
    assert!(
        pic.update_call(
            f.target,
            caller,
            "revoke_portable_login",
            frame(issuer::LOGOUT, &selected)
        )
        .await
        .is_err()
    );
    assert!(control(pic, f.target, f.installer, 7).await);
    // A renewal proves the failed response did not leave a revocation behind.
    let retained = call(&f, caller, issuer::RENEW, &renew(&input, 1))
        .await
        .unwrap();
    logout(&f, caller, &selected).await.unwrap();
    assert_eq!(
        call(
            &f,
            caller,
            issuer::RECONCILE,
            &issuer::Lookup::Renew(renew(&input, 1))
        )
        .await,
        Ok(retained)
    );
    assert_eq!(observe(&f).await.signatures, 2);
    canic_qualification::upgrade_stopped(pic, f.target, f.wasm.clone(), vec![], Some(f.installer))
        .await
        .unwrap();
    assert_eq!(
        logout(&f, caller, &selected).await,
        Err(Refusal::Unavailable)
    );
    f.close().await;
}
