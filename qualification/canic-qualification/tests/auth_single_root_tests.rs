//! Operator-controlled hub and a shard whose sole controller is the installed Root.
use super::*;

pub(super) async fn refresh(
    pic: &PocketIc,
    caller: CanisterId,
    root: CanisterId,
    receiver: CanisterId,
    managed: bool,
) -> Result<(), enrollment::Error> {
    let (target, method, id, request) = if managed {
        (
            root,
            "refresh_auth_receiver",
            enrollment::TRIGGER_REFRESH,
            enrollment_request(
                enrollment::TRIGGER_REFRESH,
                &Principal::from_slice(receiver.as_slice()),
            ),
        )
    } else {
        (
            receiver,
            enrollment::REFRESH_METHOD,
            enrollment::REFRESH,
            enrollment_request(enrollment::REFRESH, &()),
        )
    };
    let response = pic
        .update_call(target, caller, method, request)
        .await
        .unwrap();
    enrollment_response(id, &response)
}

async fn admit(
    pic: &PocketIc,
    receiver: CanisterId,
    browser: CanisterId,
    frame: &[u8],
) -> Result<canic_auth_witness::protocol::SessionView, AdmissionRefusal> {
    let bytes = pic
        .update_call(
            receiver,
            browser,
            "establish_portable_session",
            frame.to_vec(),
        )
        .await
        .unwrap();
    EstablishSession::decode_response(&bytes).unwrap()
}

async fn read(
    pic: &PocketIc,
    receiver: CanisterId,
    browser: CanisterId,
) -> Result<canic_auth_witness::protocol::SessionView, SessionReadRefusal> {
    let bytes = pic
        .query_call(
            receiver,
            browser,
            "portable_session_read",
            ReadSession::encode_request(&()).unwrap(),
        )
        .await
        .unwrap();
    ReadSession::decode_response(&bytes).unwrap()
}

#[tokio::test]
async fn single_root_revokes_shard_independently() {
    journey(true).await;
}

#[tokio::test]
async fn single_root_revokes_hub_independently() {
    journey(false).await;
}

async fn journey(revoke_shard: bool) {
    let f = fixture(false).await;
    let pic = f.owner.client();
    let hub = f.target;
    let shard = pic
        .create_canister_with_settings(Some(f.installer), None)
        .await;
    pic.install_canister(
        shard,
        f.wasm.clone(),
        f.root.as_slice().to_vec(),
        Some(f.installer),
    )
    .await;
    pic.set_controllers(shard, Some(f.installer), vec![f.root])
        .await
        .unwrap();
    let now = pic.get_time().await.as_nanos_since_unix_epoch();
    let (browser, credential) = input(now);
    let frame = EstablishSession::encode_request(&credential).unwrap();
    assert_eq!(
        admit(pic, shard, browser, &frame).await,
        Err(AdmissionRefusal::Unavailable)
    );
    let shard_grant = grant(shard, now);
    for (method, id, request) in [
        (
            "configure_auth",
            enrollment::CONFIGURE,
            enrollment_request(enrollment::CONFIGURE, &shard_grant),
        ),
        (
            "revoke_auth",
            enrollment::REVOKE,
            enrollment_request(enrollment::REVOKE, &shard_grant),
        ),
        (
            "refresh_auth_receiver",
            enrollment::TRIGGER_REFRESH,
            enrollment_request(enrollment::TRIGGER_REFRESH, &shard_grant.receiver),
        ),
    ] {
        assert!(
            pic.update_call(f.root, browser, method, request)
                .await
                .is_err(),
            "{method} {id}"
        );
    }
    pic.advance_time(Duration::from_secs(20)).await;
    for receiver in [hub, shard] {
        let approval = grant(receiver, now);
        let response = pic
            .update_call(
                f.root,
                f.installer,
                "configure_auth",
                enrollment_request(enrollment::CONFIGURE, &approval),
            )
            .await
            .unwrap();
        assert_eq!(
            enrollment_response::<Result<(), enrollment::Error>>(enrollment::CONFIGURE, &response),
            Ok(())
        );
        assert_eq!(
            refresh(pic, browser, f.root, receiver, false).await,
            Err(enrollment::Error::Binding)
        );
        // The stale installer cannot refresh the shard directly; Root cannot
        // refresh the independently controlled hub on the operator's behalf.
        assert_eq!(
            refresh(pic, f.installer, f.root, receiver, receiver == hub).await,
            Err(enrollment::Error::Binding)
        );
        refresh(pic, f.installer, f.root, receiver, receiver == shard)
            .await
            .unwrap();
        assert_eq!(
            read(pic, receiver, browser).await,
            Err(SessionReadRefusal::MissingOrExpired)
        );
        admit(pic, receiver, browser, &frame).await.unwrap();
        if receiver == hub {
            assert!(read(pic, shard, browser).await.is_err());
        }
    }
    let (retired, other) = if revoke_shard {
        (shard, hub)
    } else {
        (hub, shard)
    };
    let original = read(pic, other, browser).await.unwrap();
    let revoked = grant(retired, now);
    let response = pic
        .update_call(
            f.root,
            f.installer,
            "revoke_auth",
            enrollment_request(enrollment::REVOKE, &revoked),
        )
        .await
        .unwrap();
    assert_eq!(
        enrollment_response::<Result<(), enrollment::Error>>(enrollment::REVOKE, &response),
        Ok(())
    );
    refresh(pic, f.installer, f.root, retired, revoke_shard)
        .await
        .unwrap();
    assert_eq!(
        read(pic, retired, browser).await,
        Err(SessionReadRefusal::AuthorityUnavailable)
    );
    let (new_browser, new_credential) = input_for_device(now, 12, 1);
    let fresh_frame = EstablishSession::encode_request(&new_credential).unwrap();
    assert_eq!(
        admit(pic, retired, new_browser, &fresh_frame).await,
        Err(AdmissionRefusal::AuthorityUnavailable)
    );
    assert_eq!(read(pic, other, browser).await.unwrap(), original);
    admit(pic, other, new_browser, &fresh_frame).await.unwrap();
    f.close().await;
}

#[tokio::test]
async fn full_root_retains_revoked_slots_and_exact_approvals() {
    let f = fixture(false).await;
    let pic = f.owner.client();
    let now = pic.get_time().await.as_nanos_since_unix_epoch();
    let submit = |method, id, value: enrollment::Grant| async move {
        let bytes = pic
            .update_call(f.root, f.installer, method, enrollment_request(id, &value))
            .await
            .unwrap();
        enrollment_response::<Result<(), enrollment::Error>>(id, &bytes)
    };
    for n in 0..enrollment::MAX_RECEIVERS {
        let target = CanisterId::self_authenticating(n.to_be_bytes());
        submit("configure_auth", enrollment::CONFIGURE, grant(target, now))
            .await
            .unwrap();
    }
    let first = grant(CanisterId::self_authenticating(0_usize.to_be_bytes()), now);
    submit("revoke_auth", enrollment::REVOKE, first.clone())
        .await
        .unwrap();
    submit("configure_auth", enrollment::CONFIGURE, first.clone())
        .await
        .unwrap();
    let mut conflict = first;
    conflict.expires_at_ns += 1;
    assert_eq!(
        submit("configure_auth", enrollment::CONFIGURE, conflict).await,
        Err(enrollment::Error::Conflict)
    );
    assert_eq!(
        submit(
            "configure_auth",
            enrollment::CONFIGURE,
            grant(f.target, now)
        )
        .await,
        Err(enrollment::Error::Capacity)
    );
    f.close().await;
}
