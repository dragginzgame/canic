//! Public stable receiver lifecycle through two upgrades of a live receiver.
use super::*;
use auth_single_root_tests::{admit, read};
use canic::auth::install::{Install, LimitsPolicy, Storage};

#[tokio::test]
async fn stable_session_and_logout_survive_fresh_root_pulls() {
    use auth_issuer_tests as issuer_test;
    use canic::auth::issuer;
    let mut f = fixture(false).await;
    let pic = f.owner.client();
    let target = pic
        .create_canister_with_settings(Some(f.installer), None)
        .await;
    f.target = target;
    pic.add_cycles(target, 3_000_000_000_000).await;
    let wasm = std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/auth/stable.wasm"),
    )
    .unwrap();
    let now = pic.get_time().await.as_nanos_since_unix_epoch();
    let mut grant = grant(target, now);
    grant.grant.scopes = auth_wire::List::new(
        ["read", "write"]
            .map(|s| auth_wire::Text::new(s.into()).unwrap())
            .to_vec(),
    )
    .unwrap();
    let args = Install {
        storage: Storage::Stable,
        root: Principal::from_slice(f.root.as_slice()),
        issuer: grant.issuer,
        audience: grant.audience.fleet_id,
        network: grant.audience.canonical_network_id,
        role: "user".into(),
        limits: LimitsPolicy {
            default_session_secs: 60,
            max_session_secs: 300,
            max_proof_ttl_ns: 60 * SECOND,
            max_sessions: None,
        },
        application: ic_wire::encode(&Storage::Stable, ic_wire::Encoding::Payload, LIMITS).unwrap(),
    }
    .encode()
    .unwrap();
    pic.install_canister(target, wasm.clone(), args, Some(f.installer))
        .await;
    let reply = pic
        .update_call(
            f.root,
            f.installer,
            "configure_auth",
            enrollment_request(enrollment::CONFIGURE, &grant),
        )
        .await
        .unwrap();
    enrollment_response::<Result<(), enrollment::Error>>(enrollment::CONFIGURE, &reply).unwrap();
    receiver_refresh(pic, target, f.installer).await.unwrap();
    pic.advance_time(Duration::from_secs(3)).await;
    pic.tick().await;
    let (browser, input) = input(now);
    let frame = EstablishSession::encode_request(&input).unwrap();
    let session = admit(pic, target, browser, &frame).await.unwrap();
    exercise_coordinator(pic, target, browser, grant.issuer, session.subject, now).await;
    let issue = issuer_test::request(1);
    let issued = issuer_test::call(&f, issuer_test::account(), issuer::ISSUE, &issue)
        .await
        .unwrap();
    let renewal = issuer_test::renew(&issue, 1);
    let method_frame = |method: &str, direction| ic_wire::Encoding::Frame {
        method: ic_wire::method_id(method),
        direction,
    };
    let fault = "reject_issuer_reply";
    pic.update_call(
        target,
        f.installer,
        fault,
        ic_wire::encode(
            &true,
            method_frame(fault, ic_wire::Direction::Request),
            canic::method::LIMITS,
        )
        .unwrap(),
    )
    .await
    .unwrap();
    assert!(
        pic.update_call(
            target,
            issuer_test::browser(&issue),
            "renew_portable_credential",
            issuer_test::frame(issuer::RENEW, &renewal)
        )
        .await
        .is_err()
    );
    let upgrade = async |args| {
        canic_qualification::upgrade_stopped(pic, target, wasm.clone(), args, Some(f.installer))
            .await
    };
    for (artifact, reason) in [
        ("damaged", "open Auth receiver checkpoint"),
        (
            "changed-login-limit",
            "open Auth receiver checkpoint: Configuration",
        ),
    ] {
        let successor = std::fs::read(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join(format!("../../target/auth/{artifact}.wasm")),
        )
        .unwrap();
        let refused =
            canic_qualification::upgrade_stopped(pic, target, successor, vec![], Some(f.installer))
                .await
                .unwrap_err();
        assert!(refused.reject_message.contains(reason), "{refused:?}");
        assert_eq!(read(pic, target, browser).await.unwrap(), session);
    }
    upgrade(vec![]).await.unwrap();
    assert_eq!(
        issuer_test::call(
            &f,
            issuer_test::account(),
            issuer::RECONCILE,
            &issuer_test::lookup(&issue)
        )
        .await,
        Ok(issued)
    );
    assert_eq!(
        issuer_test::call(
            &f,
            issuer_test::browser(&issue),
            issuer::RECONCILE,
            &issuer::Lookup::Renew(renewal.clone())
        )
        .await,
        Ok(issuer::Status::Unresolved)
    );
    let stale = "stale_callback";
    let reply = pic
        .update_call(
            target,
            issuer_test::browser(&issue),
            stale,
            ic_wire::encode(
                &renewal,
                method_frame(stale, ic_wire::Direction::Request),
                canic_auth_witness::CALLBACK_LIMITS,
            )
            .unwrap(),
        )
        .await
        .unwrap();
    assert!(
        ic_wire::decode::<bool>(
            &reply,
            method_frame(stale, ic_wire::Direction::Response),
            canic::method::LIMITS
        )
        .unwrap()
    );
    assert_eq!(issuer_test::observe(&f).await.signatures, 1);
    assert!(read(pic, target, browser).await.is_err());
    receiver_refresh(pic, target, f.installer).await.unwrap();
    assert_eq!(read(pic, target, browser).await.unwrap(), session);
    let reply = pic
        .update_call(
            target,
            browser,
            "logout_portable_session",
            Logout::encode_request(&()).unwrap(),
        )
        .await
        .unwrap();
    assert!(matches!(
        Logout::decode_response(&reply).unwrap(),
        LogoutResult::Revoked(_)
    ));
    upgrade(vec![]).await.unwrap();
    receiver_refresh(pic, target, f.installer).await.unwrap();
    assert!(read(pic, target, browser).await.is_err());
    assert!(matches!(
        admit(pic, target, browser, &frame).await,
        Err(AdmissionRefusal::Revoked | AdmissionRefusal::ReplayConflict)
    ));
    let revoked = pic
        .update_call(
            f.root,
            f.installer,
            "revoke_auth",
            enrollment_request(enrollment::REVOKE, &grant),
        )
        .await
        .unwrap();
    enrollment_response::<Result<(), enrollment::Error>>(enrollment::REVOKE, &revoked).unwrap();
    receiver_refresh(pic, target, f.installer).await.unwrap();
    assert_eq!(
        coordinator_write(pic, target, browser).await,
        Err(SessionReadRefusal::Revoked)
    );
    pic.advance_time(Duration::from_secs(601)).await;
    pic.tick().await;
    assert_eq!(
        coordinator_write(pic, target, browser).await,
        Err(SessionReadRefusal::Unavailable)
    );
    f.close().await;
}

async fn exercise_coordinator(
    pic: &PocketIc,
    target: CanisterId,
    browser: CanisterId,
    issuer: Principal,
    subject: Principal,
    now: u64,
) {
    let write = async |caller| coordinator_write(pic, target, caller).await;
    assert_eq!(write(browser).await, Err(SessionReadRefusal::ScopeRejected));
    let (writer, proof) = credential::signed(now, 10, 1, issuer, subject, &["read", "write"]);
    assert_eq!(
        write(writer).await,
        Err(SessionReadRefusal::MissingOrExpired)
    );
    // Refill the receiver’s proof verification budget after the first browser login.
    pic.advance_time(Duration::from_secs(3)).await;
    pic.tick().await;
    admit(
        pic,
        target,
        writer,
        &EstablishSession::encode_request(&proof).unwrap(),
    )
    .await
    .unwrap();
    assert_eq!(write(writer).await, Ok((issuer, subject)));
    let (gate, mut held) = auth_enrollment_tests::held_gate(pic).await;
    let message = pic
        .submit_call(
            target,
            writer,
            "coordinate_write",
            write_request(Some(gate)),
        )
        .await
        .unwrap();
    let token = auth_enrollment_tests::pending(&held, pic, 1)
        .await
        .pop()
        .unwrap();
    let logout = pic
        .update_call(
            target,
            writer,
            "logout_portable_session",
            Logout::encode_request(&()).unwrap(),
        )
        .await
        .unwrap();
    assert!(matches!(
        Logout::decode_response(&logout).unwrap(),
        LogoutResult::Revoked(_)
    ));
    held.release(pic, &token).await.unwrap();
    assert!(matches!(
        write_result(&pic.await_call(message).await.unwrap()),
        Err(SessionReadRefusal::MissingOrExpired | SessionReadRefusal::Revoked)
    ));
}

fn write_frame(direction: ic_wire::Direction) -> ic_wire::Encoding {
    ic_wire::Encoding::Frame {
        method: ic_wire::method_id("coordinate_write"),
        direction,
    }
}

fn write_request(gate: Option<CanisterId>) -> Vec<u8> {
    ic_wire::encode(
        &gate,
        write_frame(ic_wire::Direction::Request),
        canic::method::LIMITS,
    )
    .unwrap()
}

fn write_result(bytes: &[u8]) -> Result<(Principal, Principal), SessionReadRefusal> {
    ic_wire::decode(
        bytes,
        write_frame(ic_wire::Direction::Response),
        canic::method::LIMITS,
    )
    .unwrap()
}

async fn coordinator_write(
    pic: &PocketIc,
    target: CanisterId,
    caller: CanisterId,
) -> Result<(Principal, Principal), SessionReadRefusal> {
    write_result(
        &pic.update_call(target, caller, "coordinate_write", write_request(None))
            .await
            .unwrap(),
    )
}
