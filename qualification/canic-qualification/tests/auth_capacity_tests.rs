//! Retained generation fences at a populated shard's upgrade boundary.
use super::*;
use auth_single_root_tests::admit;
use canic::auth::install::{Install, LimitsPolicy, Storage};

#[tokio::test]
async fn known_login_capacity_survives_checkpoint() {
    const ROWS: u32 = 10_000;
    let f = fixture_with_lifetime(false, Duration::from_secs(1200)).await;
    let pic = f.owner.client();
    let target = pic
        .create_canister_with_settings(Some(f.installer), None)
        .await;
    pic.add_cycles(target, 10_000_000_000_000).await;
    let wasm = std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/auth/stable.wasm"),
    )
    .unwrap();
    let now = pic.get_time().await.as_nanos_since_unix_epoch();
    let mut grant = grant(target, now);
    grant.expires_at_ns = now + 86400 * SECOND;
    let install = Install {
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
            max_sessions: Some(32),
            max_admitted_logins: Some(ROWS),
        },
        application: ic_wire::encode(&Storage::Stable, ic_wire::Encoding::Payload, LIMITS).unwrap(),
    };
    pic.install_canister(
        target,
        wasm.clone(),
        install.encode().unwrap(),
        Some(f.installer),
    )
    .await;
    let reply = pic
        .update_call(
            f.root,
            f.installer,
            "grant_auth_access",
            enrollment_request(enrollment::CONFIGURE, &grant),
        )
        .await
        .unwrap();
    enrollment_response::<Result<(), enrollment::Error>>(enrollment::CONFIGURE, &reply).unwrap();
    let login = |index: u32, now| {
        let id = index.to_le_bytes();
        let mut key = [0; 32];
        key[..4].copy_from_slice(&id);
        let (browser, request) = credential::signed_for_key(
            now,
            key,
            0,
            grant.issuer,
            Principal::from_slice(&id),
            &["read"],
            None,
        );
        (browser, EstablishSession::encode_request(&request).unwrap())
    };
    for first in (0..=ROWS).step_by(24) {
        pic.advance_time(Duration::from_secs(65)).await;
        receiver_refresh(pic, target, f.installer).await.unwrap();
        let now = pic.get_time().await.as_nanos_since_unix_epoch();
        for i in first..(first + 24).min(ROWS + 1) {
            let (browser, frame) = login(i, now);
            let result = admit(pic, target, browser, &frame).await;
            if i == ROWS {
                assert_eq!(result, Err(AdmissionRefusal::Capacity));
            } else {
                assert_eq!(
                    result
                        .unwrap_or_else(|err| panic!("row {i}: {err:?}"))
                        .subject,
                    Principal::from_slice(&i.to_le_bytes())
                );
            }
        }
    }
    let before = pic.cycle_balance(target).await;
    canic_qualification::upgrade_stopped(pic, target, wasm, vec![], Some(f.installer))
        .await
        .unwrap();
    let cost = before - pic.cycle_balance(target).await;
    let method = ic_wire::method_id("checkpoint_bytes");
    let reply = pic
        .query_call(
            target,
            f.installer,
            "checkpoint_bytes",
            enrollment_request(method, &()),
        )
        .await
        .unwrap();
    let bytes = enrollment_response::<u64>(method, &reply);
    assert!(bytes > 0);
    eprintln!(
        "{ROWS} retained login rows: {bytes} checkpoint bytes, {cost} PocketIC stop/upgrade/start cycles"
    );
    pic.advance_time(Duration::from_secs(65)).await;
    receiver_refresh(pic, target, f.installer).await.unwrap();
    let now = pic.get_time().await.as_nanos_since_unix_epoch();
    let (browser, frame) = login(ROWS, now);
    assert_eq!(
        admit(pic, target, browser, &frame).await,
        Err(AdmissionRefusal::Capacity)
    );
    f.close().await;
}
