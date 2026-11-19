//! Actual Canic mounts with Auth's real volatile receiver and Platform signatures.
mod auth_enrollment_tests;
mod auth_host_tests;
mod auth_issuer_tests;
mod auth_logout_tests;
mod auth_single_root_tests;
mod credential;

use canic::auth::protocol::wire as auth_wire;
use canic_auth_witness::{
    KEY, LIMITS, OBSERVE, Observation, enrollment,
    protocol::{
        CredentialEnvelope, EstablishSession, Logout, ReadSession, SessionInput,
        model::{AdmissionRefusal, LogoutResult, PortableProof, SessionReadRefusal},
    },
};
use ic_auth_protocol_types::Principal;
use ic_platform_crypto::ed25519::{Signer, SigningKey};
use ic_testkit::{
    pocket_ic::{CanisterId, PocketIcBuilder, nonblocking::PocketIc},
    server::{Instance, Limits, PreparedServer, Server},
};
use std::{path::PathBuf, time::Duration};

const SECOND: u64 = 1_000_000_000;

fn input(now: u64) -> (CanisterId, SessionInput) {
    input_for_device(now, 9, 0)
}

fn input_for_device(now: u64, device: u8, slot: u8) -> (CanisterId, SessionInput) {
    credential::signed(
        now,
        device,
        slot,
        Principal::from_slice(&[1]),
        Principal::from_slice(&[2]),
        &["read"],
    )
}

async fn receiver_refresh(
    pic: &PocketIc,
    receiver: CanisterId,
    caller: CanisterId,
) -> Result<(), enrollment::Error> {
    let bytes = pic
        .update_call(
            receiver,
            caller,
            enrollment::REFRESH_METHOD,
            enrollment::encode_refresh_request().unwrap(),
        )
        .await
        .unwrap();
    enrollment::decode_refresh_response(&bytes).unwrap()
}

async fn control(pic: &PocketIc, target: CanisterId, caller: CanisterId, command: u8) -> bool {
    pic.update_call(target, caller, "control", vec![command])
        .await
        .unwrap()
        == [1]
}

async fn observe(pic: &PocketIc, target: CanisterId) -> Observation {
    let bytes = pic
        .query_call(target, CanisterId::anonymous(), "observe", vec![])
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

fn enrollment_request(method: u64, value: &impl serde::Serialize) -> Vec<u8> {
    ic_wire::encode(
        value,
        ic_wire::Encoding::Frame {
            method,
            direction: ic_wire::Direction::Request,
        },
        enrollment::LIMITS,
    )
    .unwrap()
}

fn enrollment_response<T: serde::de::DeserializeOwned>(method: u64, bytes: &[u8]) -> T {
    ic_wire::decode(
        bytes,
        ic_wire::Encoding::Frame {
            method,
            direction: ic_wire::Direction::Response,
        },
        enrollment::LIMITS,
    )
    .unwrap()
}

async fn snapshot(pic: &PocketIc, receiver: CanisterId) -> enrollment::Snapshot {
    let bytes = pic
        .query_call(
            receiver,
            CanisterId::anonymous(),
            "observe_enrollment",
            vec![],
        )
        .await
        .unwrap();
    enrollment_response::<Result<enrollment::Snapshot, enrollment::Error>>(
        enrollment::SNAPSHOT,
        &bytes,
    )
    .unwrap()
}

struct Fixture {
    server: Server,
    owner: Instance,
    root: CanisterId,
    target: CanisterId,
    installer: CanisterId,
    root_wasm: Vec<u8>,
    wasm: Vec<u8>,
}

impl Fixture {
    async fn close(mut self) {
        assert_eq!(
            self.owner.close(Duration::from_secs(10)).await,
            ic_testkit::server::Deletion::Attempted
        );
        let stopped = self.server.stop().unwrap();
        assert!(stopped.process.cleanup.is_empty(), "{stopped:?}");
    }
}

async fn fixture(cross_subnet: bool) -> Fixture {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .to_path_buf();
    let wasm = std::fs::read(repo.join("target/auth/receiver.wasm"))
        .expect("make auth-test builds witness");
    let server = PreparedServer::check(&canic_qualification::server_directory())
        .expect("make qualification-setup");
    let builder = PocketIcBuilder::new().with_application_subnet();
    let builder = if cross_subnet {
        builder.with_application_subnet()
    } else {
        builder
    };
    let server = Server::start(
        server,
        Limits {
            startup: Duration::from_secs(30),
            lifetime: Duration::from_secs(180),
            stdout_bytes: 1024 * 1024,
            stderr_bytes: 1024 * 1024,
        },
    )
    .unwrap();
    let owner = server
        .instance(builder, Duration::from_secs(30))
        .await
        .unwrap();
    let pic = owner.client();
    let subnets = pic.topology().await.get_app_subnets();
    let installer = CanisterId::self_authenticating(b"auth-installation-authority");
    let root = pic
        .create_canister_on_subnet(Some(installer), None, subnets[0])
        .await;
    let root_wasm = std::fs::read(repo.join("target/auth/root.wasm")).unwrap();
    pic.install_canister(root, root_wasm.clone(), vec![], Some(installer))
        .await;
    let target = pic
        .create_canister_on_subnet(Some(installer), None, subnets[usize::from(cross_subnet)])
        .await;
    pic.install_canister(
        target,
        wasm.clone(),
        root.as_slice().to_vec(),
        Some(installer),
    )
    .await;
    Fixture {
        server,
        owner,
        root,
        target,
        installer,
        root_wasm,
        wasm,
    }
}

fn grant(target: CanisterId, now: u64) -> enrollment::Grant {
    enrollment::Grant {
        receiver: Principal::from_slice(target.as_slice()),
        audience: canic_auth_witness::protocol::model::AudienceId {
            canonical_network_id: [0x11; 32],
            fleet_id: [0x22; 32],
        },
        issuer: Principal::from_slice(&[1]),
        key: KEY,
        grant: canic_auth_witness::protocol::model::DelegatedRoleGrant {
            target: auth_wire::Text::new("user".into()).unwrap(),
            scopes: auth_wire::List::new(vec![auth_wire::Text::new("read".into()).unwrap()])
                .unwrap(),
        },
        not_before_ns: now,
        expires_at_ns: now + 600 * SECOND,
    }
}

#[tokio::test(flavor = "current_thread")]
async fn receiver_mounts_bind_callers_preserve_refusals_and_rollback() {
    let Fixture {
        mut server,
        owner,
        root,
        target,
        installer,
        root_wasm,
        wasm,
    } = fixture(false).await;
    let pic = owner.client();
    let now = pic.get_time().await.as_nanos_since_unix_epoch();
    let (browser, input) = input(now);
    let frame = EstablishSession::encode_request(&input).unwrap();
    let outsider = CanisterId::self_authenticating(b"not-the-browser");
    let grant = grant(target, now);
    let configure = enrollment_request(enrollment::CONFIGURE, &grant);
    assert!(
        pic.update_call(root, browser, "configure_auth", configure.clone())
            .await
            .is_err()
    );
    let response = pic
        .update_call(root, installer, "configure_auth", configure.clone())
        .await
        .unwrap();
    assert_eq!(
        enrollment_response::<Result<(), enrollment::Error>>(enrollment::CONFIGURE, &response),
        Ok(())
    );
    let forged = enrollment::Request {
        challenge: [9; 32],
        receipt_deadline_ns: now + enrollment::RECEIPT_NS,
    };
    let response = pic
        .update_call(
            root,
            browser,
            "auth_enrollment",
            enrollment_request(enrollment::SNAPSHOT, &forged),
        )
        .await
        .unwrap();
    assert_eq!(
        enrollment_response::<Result<enrollment::Snapshot, enrollment::Error>>(
            enrollment::SNAPSHOT,
            &response
        ),
        Err(enrollment::Error::Binding)
    );
    let closed = pic
        .update_call(target, browser, "establish_portable_session", frame.clone())
        .await
        .unwrap();
    assert_eq!(
        EstablishSession::decode_response(&closed).unwrap(),
        Err(AdmissionRefusal::Unavailable)
    );
    assert!(
        receiver_refresh(pic, target, browser).await.is_err(),
        "claimed app membership cannot configure authority"
    );
    assert!(receiver_refresh(pic, target, installer).await.is_ok());
    let initial_enrollment = snapshot(pic, target).await;
    assert_eq!(initial_enrollment.grant, grant);
    assert_ne!(initial_enrollment.request.challenge, [0; 32]);
    assert_eq!(observe(pic, target).await.enrollment, Some((1, 1)));

    // Exports, directions and IDs are independently specified, including the
    // data-only credential envelope which must never become an endpoint.
    let before = observe(pic, target).await;
    for (name, id, query, valid, budget) in [
        (
            "establish_portable_session",
            0x415554480012,
            false,
            frame.clone(),
            EstablishSession::REQUEST_LIMITS,
        ),
        (
            "portable_session_read",
            0x415554480013,
            true,
            ReadSession::encode_request(&()).unwrap(),
            ReadSession::REQUEST_LIMITS,
        ),
        (
            "logout_portable_session",
            0x415554480014,
            false,
            Logout::encode_request(&()).unwrap(),
            Logout::REQUEST_LIMITS,
        ),
    ] {
        let mut malformed = valid.clone();
        malformed.push(0);
        let wrong_id = ic_wire::encode(
            &(),
            ic_wire::Encoding::Frame {
                method: id + 99,
                direction: ic_wire::Direction::Request,
            },
            LIMITS,
        )
        .unwrap();
        let wrong_direction = ic_wire::encode(
            &(),
            ic_wire::Encoding::Frame {
                method: id,
                direction: ic_wire::Direction::Response,
            },
            LIMITS,
        )
        .unwrap();
        let mut oversized = valid.clone();
        oversized.resize(budget.bytes + 1, 0);
        for bytes in [vec![], wrong_id, wrong_direction, malformed, oversized] {
            let result = if query {
                pic.query_call(target, browser, name, bytes).await
            } else {
                pic.update_call(target, browser, name, bytes).await
            };
            assert!(result.is_err(), "{name} admitted malformed framing");
            assert_eq!(observe(pic, target).await, before);
        }
        if !query {
            assert!(pic.query_call(target, browser, name, valid).await.is_err());
        }
    }
    assert!(
        pic.update_call(target, browser, CredentialEnvelope::NAME, vec![])
            .await
            .is_err()
    );
    for caller in [outsider, installer] {
        let mismatch = pic
            .update_call(target, caller, "establish_portable_session", frame.clone())
            .await
            .unwrap();
        assert_eq!(
            EstablishSession::decode_response(&mismatch).unwrap(),
            Err(AdmissionRefusal::InvalidCredential),
            "installation/control authority does not grant browser identity"
        );
    }

    // A real strict-verification refusal is an ordinary response: its charge
    // must commit, while no session or application projection appears.
    // Fresh receiver credit starts empty; actual IC time replenishes its bucket.
    pic.advance_time(Duration::from_secs(20)).await;
    let mut bad = input.clone();
    let signature = SigningKey::from_bytes(&[8; 32]).sign(&[0; 32]).to_bytes();
    bad.credential.proof =
        PortableProof::Ed25519(auth_wire::Bytes::new(signature.to_vec()).unwrap());
    let before = observe(pic, target).await;
    let refused = pic
        .update_call(
            target,
            browser,
            "establish_portable_session",
            EstablishSession::encode_request(&bad).unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        EstablishSession::decode_response(&refused).unwrap(),
        Err(AdmissionRefusal::InvalidCredential)
    );
    let after = observe(pic, target).await;
    assert_eq!(before.credit, 0);
    assert_eq!(after.credit, 18);
    assert_eq!(after.usage, before.usage);
    assert_eq!(after.writes, before.writes);
    assert_eq!(after.calls, before.calls + 1);

    // Encoding failure rolls real receiver publication and application state
    // back together; the same valid request can subsequently succeed.
    assert!(control(pic, target, installer, 2).await);
    let before = observe(pic, target).await;
    assert!(
        pic.update_call(target, browser, "establish_portable_session", frame.clone())
            .await
            .is_err()
    );
    assert_eq!(observe(pic, target).await, before);
    assert!(control(pic, target, installer, 3).await);
    let admitted = pic
        .update_call(target, browser, "establish_portable_session", frame.clone())
        .await
        .unwrap();
    let receipt = EstablishSession::decode_response(&admitted)
        .unwrap()
        .unwrap();
    assert_eq!(receipt.caller.as_slice(), browser.as_slice());
    assert_eq!(receipt.subject, Principal::from_slice(&[2]));
    assert_eq!(observe(pic, target).await.writes, 1);
    let original = observe(pic, target).await;
    let retry = pic
        .update_call(target, browser, "establish_portable_session", frame.clone())
        .await
        .unwrap();
    assert_eq!(
        EstablishSession::decode_response(&retry).unwrap(),
        Ok(receipt.clone())
    );
    let retry_state = observe(pic, target).await;
    assert_eq!(
        (retry_state.writes, retry_state.credit, retry_state.usage),
        (original.writes, original.credit, original.usage)
    );
    let read = ReadSession::encode_request(&()).unwrap();
    let bytes = pic
        .query_call(target, browser, "portable_session_read", read.clone())
        .await
        .unwrap();
    assert_eq!(
        ReadSession::decode_response(&bytes).unwrap(),
        Ok(receipt.clone())
    );
    let bytes = pic
        .query_call(target, outsider, "portable_session_read", read.clone())
        .await
        .unwrap();
    assert_eq!(
        ReadSession::decode_response(&bytes).unwrap(),
        Err(SessionReadRefusal::MissingOrExpired)
    );
    assert!(control(pic, target, installer, 4).await);
    let bytes = pic
        .query_call(target, browser, "portable_session_read", read.clone())
        .await
        .unwrap();
    assert_eq!(
        ReadSession::decode_response(&bytes).unwrap(),
        Err(SessionReadRefusal::AuthorityUnavailable)
    );
    assert!(control(pic, target, installer, 5).await);

    // A real later authority response commits at Root, then the receiver's
    // callback traps. Its two projections roll back together, while the
    // original pending receipt window remains busy instead of silently renewing.
    assert!(control(pic, target, installer, 2).await);
    let before = observe(pic, target).await;
    let retained = snapshot(pic, target).await;
    assert!(
        pic.update_call(
            target,
            installer,
            enrollment::REFRESH_METHOD,
            enrollment::encode_refresh_request().unwrap()
        )
        .await
        .is_err()
    );
    assert_eq!(observe(pic, target).await, before);
    assert_eq!(snapshot(pic, target).await, retained);
    assert!(control(pic, target, installer, 3).await);
    assert!(receiver_refresh(pic, target, installer).await.is_err());
    pic.advance_time(Duration::from_secs(31)).await;
    assert!(receiver_refresh(pic, target, installer).await.is_ok());
    let refreshed = snapshot(pic, target).await;
    assert_eq!(refreshed.grant, retained.grant);
    assert_eq!(refreshed.generation, retained.generation);
    assert!(refreshed.revision > retained.revision);
    assert_ne!(refreshed.request.challenge, retained.request.challenge);
    let bytes = pic
        .query_call(target, browser, "portable_session_read", read.clone())
        .await
        .unwrap();
    assert_eq!(ReadSession::decode_response(&bytes).unwrap(), Ok(receipt));

    let logout = Logout::encode_request(&()).unwrap();
    assert!(control(pic, target, installer, 2).await);
    let before = observe(pic, target).await;
    assert!(
        pic.update_call(target, browser, "logout_portable_session", logout.clone())
            .await
            .is_err()
    );
    assert_eq!(observe(pic, target).await, before);
    assert!(control(pic, target, installer, 3).await);
    let bytes = pic
        .update_call(target, browser, "logout_portable_session", logout)
        .await
        .unwrap();
    assert_eq!(
        Logout::decode_response(&bytes).unwrap(),
        LogoutResult::Revoked(now + 300 * SECOND)
    );
    let bytes = pic
        .update_call(target, browser, "establish_portable_session", frame.clone())
        .await
        .unwrap();
    assert_eq!(
        EstablishSession::decode_response(&bytes).unwrap(),
        Err(AdmissionRefusal::Revoked)
    );
    let bytes = pic
        .query_call(target, browser, "portable_session_read", read.clone())
        .await
        .unwrap();
    assert_eq!(
        ReadSession::decode_response(&bytes).unwrap(),
        Err(SessionReadRefusal::Revoked)
    );

    // Revoke while the receiver is awaiting real management entropy, before
    // its direct Root snapshot can be dispatched.
    let pending = pic
        .submit_call(
            target,
            installer,
            enrollment::REFRESH_METHOD,
            enrollment::encode_refresh_request().unwrap(),
        )
        .await
        .unwrap();
    pic.tick().await;
    assert!(pic.ingress_status(pending.clone()).await.is_none());
    let response = pic
        .update_call(
            root,
            installer,
            "revoke_auth",
            enrollment_request(enrollment::REVOKE, &grant),
        )
        .await
        .unwrap();
    assert_eq!(
        enrollment_response::<Result<(), enrollment::Error>>(enrollment::REVOKE, &response),
        Ok(())
    );
    assert_eq!(
        enrollment::decode_refresh_response(&pic.await_call(pending).await.unwrap()).unwrap(),
        Ok(())
    );
    let revoked = snapshot(pic, target).await;
    assert!(revoked.revoked);
    assert!(revoked.generation > refreshed.generation);
    // An exact approval retry cannot reenable a retired key.
    let response = pic
        .update_call(root, installer, "configure_auth", configure.clone())
        .await
        .unwrap();
    assert_eq!(
        enrollment_response::<Result<(), enrollment::Error>>(enrollment::CONFIGURE, &response),
        Ok(())
    );
    assert!(receiver_refresh(pic, target, installer).await.is_ok());
    assert!(snapshot(pic, target).await.revoked);
    let bytes = pic
        .query_call(target, browser, "portable_session_read", read.clone())
        .await
        .unwrap();
    assert_eq!(
        ReadSession::decode_response(&bytes).unwrap(),
        Err(SessionReadRefusal::AuthorityUnavailable)
    );
    canic_qualification::upgrade_stopped(pic, root, root_wasm, vec![], Some(installer))
        .await
        .unwrap();
    let response = pic
        .update_call(root, installer, "configure_auth", configure)
        .await
        .unwrap();
    assert_eq!(
        enrollment_response::<Result<(), enrollment::Error>>(enrollment::CONFIGURE, &response),
        Err(enrollment::Error::Closed)
    );
    assert!(receiver_refresh(pic, target, installer).await.is_err());

    canic_qualification::upgrade_stopped(pic, target, wasm, vec![], Some(installer))
        .await
        .unwrap();
    assert!(!observe(pic, target).await.present);
    assert!(
        receiver_refresh(pic, target, installer).await.is_err(),
        "enrollment cannot repair volatile safety loss"
    );
    let bytes = pic
        .update_call(target, browser, "establish_portable_session", frame)
        .await
        .unwrap();
    assert_eq!(
        EstablishSession::decode_response(&bytes).unwrap(),
        Err(AdmissionRefusal::Unavailable)
    );
    let bytes = pic
        .query_call(target, browser, "portable_session_read", read)
        .await
        .unwrap();
    assert_eq!(
        ReadSession::decode_response(&bytes).unwrap(),
        Err(SessionReadRefusal::Unavailable)
    );
    assert_eq!(
        owner.close(std::time::Duration::from_secs(10)).await,
        ic_testkit::server::Deletion::Attempted
    );
    let stopped = server.stop().unwrap();
    assert!(stopped.process.cleanup.is_empty(), "{stopped:?}");
}
