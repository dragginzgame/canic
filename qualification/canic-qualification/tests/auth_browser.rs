//! Actual Chromium/IndexedDB, Canic issuer mounts and the shared profile engine.
use canic::auth::{enrollment, protocol::wire as auth_wire};
use canic_auth_witness::{IssuerObservation, KEY, LIMITS, OBSERVE};
use canic_profile_protocol::{GET, PUT, Profile, ProfileError, ProfileResult, Put};
use ic_auth::verification::browser_principal;
use ic_auth_protocol_types::Principal;
use ic_host_process::{Input, Ownership, Policy, Stderr, run};
use ic_platform_crypto::ed25519::SigningKey;
use ic_storage::VectorMemory;
use ic_testkit::{
    pocket_ic::{CanisterId, PocketIcBuilder, Time},
    server::{Limits, PreparedServer, Server},
};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::json;
use std::{
    path::PathBuf,
    process::Command,
    sync::atomic::AtomicBool,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

fn key(seed: u8) -> Principal {
    browser_principal(
        &SigningKey::from_bytes(&[seed; 32])
            .verifying_key()
            .to_bytes(),
    )
}

fn id(principal: Principal) -> CanisterId {
    CanisterId::from_slice(principal.as_slice())
}

fn frame<T: Serialize>(method: u64, direction: ic_wire::Direction, value: &T) -> Vec<u8> {
    ic_wire::encode(
        value,
        ic_wire::Encoding::Frame { method, direction },
        canic_profile_protocol::LIMITS,
    )
    .unwrap()
}

fn observed<T: DeserializeOwned>(bytes: &[u8]) -> T {
    ic_wire::decode(
        bytes,
        ic_wire::Encoding::Frame {
            method: OBSERVE,
            direction: ic_wire::Direction::Response,
        },
        LIMITS,
    )
    .unwrap()
}

fn enrollment_frame<T: Serialize>(
    method: u64,
    direction: ic_wire::Direction,
    value: &T,
) -> Vec<u8> {
    ic_wire::encode(
        value,
        ic_wire::Encoding::Frame { method, direction },
        enrollment::LIMITS,
    )
    .unwrap()
}

fn profile(seed: u8, user_id: u64, display_name: &str) -> Profile {
    Profile {
        user_id,
        owner: key(seed).as_slice().to_vec(),
        revision: 1,
        display_name: display_name.into(),
    }
}

#[tokio::test]
#[ignore = "requires explicitly prepared Chromium/npm and simulator; make auth-profile-test"]
async fn installed_browser_profiles_preserve_identity_recovery_and_logout() {
    assert!(
        std::env::var_os("CANIC_AUTH_BROWSER_EXECUTABLE").is_some(),
        "select prepared Chromium"
    );
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .to_path_buf();
    let wasm =
        std::fs::read(repo.join("target/auth/profile.wasm")).expect("make auth-profile-test");
    let root_wasm =
        std::fs::read(repo.join("target/auth/root.wasm")).expect("make auth-profile-test");
    let mut server = Server::start(
        PreparedServer::check(&canic_qualification::server_directory()).unwrap(),
        Limits {
            startup: Duration::from_secs(30),
            lifetime: Duration::from_secs(180),
            stdout_bytes: 1024 * 1024,
            stderr_bytes: 1024 * 1024,
        },
    )
    .unwrap();
    let mut owned = server
        .instance(
            PocketIcBuilder::new()
                .with_nns_subnet()
                .with_application_subnet(),
            Duration::from_secs(30),
        )
        .await
        .unwrap();
    let pic = owned.client();
    let wall_now: u64 = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos()
        .try_into()
        .unwrap();
    // Auth starts with an empty verification bucket. Give the installed owner
    // twenty seconds of real policy time before the live browser clock takes over.
    let now = wall_now - 20_000_000_000;
    pic.set_time(Time::from_nanos_since_unix_epoch(now)).await;
    let installer = id(key(13));
    let mut roots = Vec::new();
    for _ in 0..2 {
        let root = pic
            .create_canister_with_settings(Some(installer), None)
            .await;
        pic.add_cycles(root, 10_000_000_000_000).await;
        pic.install_canister(root, root_wasm.clone(), vec![], Some(installer))
            .await;
        roots.push(root);
    }
    // Separate issuer and two independent receivers, using two existing single-grant Roots.
    let mut actors = Vec::new();
    for authority in [roots[0], roots[0], roots[1]] {
        let actor = pic
            .create_canister_with_settings(Some(installer), None)
            .await;
        pic.add_cycles(actor, 10_000_000_000_000).await;
        pic.install_canister(
            actor,
            wasm.clone(),
            authority.as_slice().to_vec(),
            Some(installer),
        )
        .await;
        actors.push(actor);
    }
    let issuer = actors[0];
    let receivers = &actors[1..];
    let mut authorities = Vec::new();
    for (root, receiver) in roots.iter().zip(receivers) {
        let grant = enrollment::Grant {
            receiver: Principal::from_slice(receiver.as_slice()),
            audience: canic::auth::protocol::model::AudienceId {
                canonical_network_id: [0x11; 32],
                fleet_id: [0x22; 32],
            },
            issuer: Principal::from_slice(issuer.as_slice()),
            key: KEY,
            grant: canic::auth::protocol::model::DelegatedRoleGrant {
                target: auth_wire::Text::new("user".into()).unwrap(),
                scopes: auth_wire::List::new(vec![
                    auth_wire::Text::new("read".into()).unwrap(),
                    auth_wire::Text::new("write".into()).unwrap(),
                ])
                .unwrap(),
            },
            not_before_ns: now,
            expires_at_ns: now + 600_000_000_000,
        };
        let bytes = enrollment_frame(enrollment::CONFIGURE, ic_wire::Direction::Request, &grant);
        authorities.push(json!({
            "canister": root.to_text(), "configure": bytes,
            "revoke": enrollment_frame(enrollment::REVOKE, ic_wire::Direction::Request, &grant),
        }));
        let result = pic
            .update_call(*root, installer, "configure_auth", bytes)
            .await
            .unwrap();
        let reply: Result<(), enrollment::Error> = ic_wire::decode(
            &result,
            ic_wire::Encoding::Frame {
                method: enrollment::CONFIGURE,
                direction: ic_wire::Direction::Response,
            },
            enrollment::LIMITS,
        )
        .unwrap();
        assert_eq!(reply, Ok(()));
        assert_eq!(
            pic.update_call(
                *receiver,
                installer,
                enrollment::REFRESH_METHOD,
                enrollment::encode_refresh_request().unwrap()
            )
            .await
            .unwrap(),
            enrollment_frame(
                enrollment::REFRESH,
                ic_wire::Direction::Response,
                &Ok::<(), enrollment::Error>(())
            )
        );
    }
    pic.set_time(Time::from_nanos_since_unix_epoch(wall_now))
        .await;
    let trust = pic.root_key().await.unwrap();
    let gateway = owned.client_mut().make_live(None).await;
    let request = ic_wire::Direction::Request;
    let response = ic_wire::Direction::Response;
    let result = |method, value: ProfileResult| frame(method, response, &value);
    let config = json!({ "issuer": issuer.to_text(), "receivers": receivers.iter().map(|id| id.to_text()).collect::<Vec<_>>(),
        "gateway": gateway.as_str(), "root": trust, "authorities": authorities, "frames": {
            "putA": frame(PUT, request, &Put { user_id: 1, expected_revision: 0, display_name: "Alice".into() }),
            "putC": frame(PUT, request, &Put { user_id: 2, expected_revision: 0, display_name: "Carol".into() }),
            "remapA": frame(PUT, request, &Put { user_id: 3, expected_revision: 0, display_name: "Alice".into() }),
            "changeA": frame(PUT, request, &Put { user_id: 1, expected_revision: 1, display_name: "rolled back".into() }),
            "changeC": frame(PUT, request, &Put { user_id: 2, expected_revision: 1, display_name: "must not escape".into() }),
            "getA": frame(GET, request, &1_u64), "getC": frame(GET, request, &2_u64),
            "profileA": result(PUT, Ok(profile(8, 1, "Alice"))),
            "profileAGet": result(GET, Ok(profile(8, 1, "Alice"))),
            "profileC": result(PUT, Ok(profile(10, 2, "Carol"))),
            "unauthGet": result(GET, Err(ProfileError::Unauthenticated)),
            "unauthPut": result(PUT, Err(ProfileError::Unauthenticated)),
            "missingGet": result(GET, Err(ProfileError::NotFound)),
            "forbiddenGet": result(GET, Err(ProfileError::Forbidden)),
            "alreadyRegistered": result(PUT, Err(ProfileError::AlreadyRegistered)),
            "revoked": canic::auth::protocol::EstablishSession::encode_response(&Err(canic::auth::protocol::model::AdmissionRefusal::Revoked)).unwrap(),
            "refreshMethod": enrollment::REFRESH_METHOD,
            "refresh": enrollment::encode_refresh_request().unwrap(),
            "refreshOk": enrollment_frame(enrollment::REFRESH, response, &Ok::<(), enrollment::Error>(())),
            "revokeOk": enrollment_frame(enrollment::REVOKE, response, &Ok::<(), enrollment::Error>(())),
            "configureOk": enrollment_frame(enrollment::CONFIGURE, response, &Ok::<(), enrollment::Error>(())),
        } });
    let output = repo.join("target/auth-browser-installed");
    std::fs::create_dir_all(&output).unwrap();
    let config_path = output.join("config.json");
    std::fs::write(&config_path, serde_json::to_vec_pretty(&config).unwrap()).unwrap();
    let mut command = Command::new("node");
    command
        .current_dir(&repo)
        .arg("qualification/canic-qualification/browser/run.mjs")
        .arg(&config_path);
    let mut evidence = Vec::new();
    let report = run(
        &mut command,
        Policy {
            ownership: Ownership::Direct,
            input: Input::Null,
            stdout_bytes: 64 * 1024,
            stderr: Stderr::Capture(64 * 1024),
            elapsed: Duration::from_secs(90),
        },
        &AtomicBool::new(false),
        |_stream, bytes| {
            evidence.extend_from_slice(bytes);
            Ok(())
        },
    );
    std::fs::write(output.join("browser.log"), &evidence).unwrap();
    owned.client_mut().stop_live().await;
    let pic = owned.client();
    let issuance = pic
        .query_call(issuer, CanisterId::anonymous(), "observe_issuer", vec![])
        .await
        .unwrap();
    let writes = pic
        .query_call(
            receivers[0],
            CanisterId::anonymous(),
            "observe_profile_writes",
            vec![],
        )
        .await
        .unwrap();
    let before = pic.get_stable_memory(receivers[0]).await;
    let other_writes = pic
        .query_call(
            receivers[1],
            CanisterId::anonymous(),
            "observe_profile_writes",
            vec![],
        )
        .await
        .unwrap();
    let upgrade =
        canic_qualification::upgrade_stopped(pic, receivers[0], wasm, vec![], Some(installer))
            .await;
    let after = pic.get_stable_memory(receivers[0]).await;
    // The replacement was active and could read this owner's profile before upgrade.
    let closed = pic
        .update_call(
            receivers[0],
            id(key(15)),
            "profile_get",
            frame(GET, request, &1_u64),
        )
        .await;
    let refresh = pic
        .update_call(
            receivers[0],
            installer,
            enrollment::REFRESH_METHOD,
            enrollment::encode_refresh_request().unwrap(),
        )
        .await;
    assert_eq!(
        owned.close(Duration::from_secs(10)).await,
        ic_testkit::server::Deletion::Attempted
    );
    let stopped = server.stop().unwrap();
    assert!(stopped.process.cleanup.is_empty(), "{stopped:?}");
    println!("{}", String::from_utf8_lossy(&evidence));
    assert!(
        report.failure.is_none()
            && report.cleanup.is_empty()
            && report.status.is_some_and(|s| s.success()),
        "{report:?}"
    );
    let issuance: IssuerObservation = observed(&issuance);
    assert_eq!(
        (issuance.signatures, issuance.operations, issuance.logins),
        (6, 6, 5)
    );
    assert_eq!(
        observed::<u32>(&writes),
        2,
        "exactly two successful profile writes; trap rolled back its count"
    );
    assert_eq!(
        observed::<u32>(&other_writes),
        0,
        "revoked receiver cannot write"
    );
    upgrade.unwrap();
    assert_eq!(before, after, "upgrade preserved application backing");
    assert_eq!(
        closed.unwrap(),
        result(GET, Err(ProfileError::Unauthenticated))
    );
    assert_eq!(
        enrollment::decode_refresh_response(&refresh.unwrap()).unwrap(),
        Err(enrollment::Error::Closed),
        "fresh enrollment cannot revive lost Auth state"
    );
    let backing = VectorMemory::new(std::cell::RefCell::new(after));
    let state = canic_auth_witness::open_profiles(backing, false);
    assert_eq!(state.get(key(8).as_slice(), 1), Ok(profile(8, 1, "Alice")));
    assert_eq!(
        state.get(key(10).as_slice(), 2),
        Ok(profile(10, 2, "Carol"))
    );
}
