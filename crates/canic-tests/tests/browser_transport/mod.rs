//! Explicit npm-backed acceptance of the delivered Auth client's real HTTP transports.
mod issuer_http;
use super::*;
use canic::dto::portable_enrollment::{
    PortableIssuerChallenge, PortableIssuerCustody, PortableIssuerEnrollment,
    PortableIssuerRegistrationRefusal, PreparePortableIssuerKey,
};
use std::{
    io::Write,
    process::{Command, Stdio},
};

#[test]
#[ignore = "requires prepared frontend-consumer npm dependencies and the selected Node runtime"]
fn delivered_client_signs_admission_and_device_scoped_logout() {
    let mut fixture = canic_tests::pic::setup_http_active_component_registry();
    let host = fixture.fresh_pic_mut().unwrap().make_live(None).unwrap();
    enroll_live(&fixture);
    let targets = [fixture.issuer.canister_id, fixture.verifier.canister_id];
    let devices = [61, 62];
    let pic = fixture.pic();
    for target in targets {
        await_missing_session(pic, target, 61, Duration::from_secs(30));
    }
    let credentials: Vec<_> = devices
        .iter()
        .map(|device| {
            let issued = issue_fixture(
                &fixture,
                portable_issue_request(&fixture, *device),
                Principal::from_slice(&[43]),
            )
            .unwrap();
            ic_auth_wire::CredentialEnvelope::encode_request(&issued.credential).unwrap()
        })
        .collect();
    let mut input = serde_json::json!({
        "action": "admit-lost", "host": host.as_str(),
        "rootKey": pic.root_key().unwrap().unwrap(),
        "targets": targets.map(|target| target.to_text()),
        "devices": devices, "callers": devices.map(|device| portable_browser(device).to_text()),
        "credentials": credentials,
        "scopes": ["app00:xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"]
    });
    run_client(&input);
    let original: Vec<_> = targets
        .into_iter()
        .flat_map(|target| {
            devices.map(|device| {
                let deadline = std::time::Instant::now() + Duration::from_secs(5);
                loop {
                    if let Ok(session) = portable_read(pic, target, portable_browser(device)) {
                        break (target, device, session);
                    }
                    assert!(
                        std::time::Instant::now() < deadline,
                        "accepted admission must settle"
                    );
                    std::thread::sleep(Duration::from_millis(50));
                }
            })
        })
        .collect();
    input["action"] = "admit".into();
    run_client(&input);
    for (target, device, session) in &original {
        assert_eq!(
            portable_read(pic, *target, portable_browser(*device)),
            Ok(session.clone())
        );
    }
    input["action"] = "logout-lost".into();
    run_client(&input);
    for target in targets {
        await_missing_session(pic, target, 61, Duration::from_secs(5));
    }
    input["action"] = "logout".into();
    run_client(&input);
    for target in targets {
        assert_eq!(
            portable_read(pic, target, portable_browser(61)),
            Err(ic_auth_wire::model::SessionReadRefusal::MissingOrExpired)
        );
        let retained = original
            .iter()
            .find(|(receiver, device, _)| *receiver == target && *device == 62)
            .unwrap();
        assert_eq!(
            portable_read(pic, target, portable_browser(62)),
            Ok(retained.2.clone())
        );
    }
    fixture.fresh_pic_mut().unwrap().stop_live().unwrap();
    drop(fixture);
}

fn await_missing_session(pic: &PocketIc, target: Principal, device: u8, timeout: Duration) {
    let deadline = std::time::Instant::now() + timeout;
    loop {
        let observed = portable_read(pic, target, portable_browser(device));
        if observed == Err(ic_auth_wire::model::SessionReadRefusal::MissingOrExpired) {
            return;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "receiver state: {observed:?}"
        );
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn enroll_live(fixture: &ActiveComponentRegistryFixture) {
    let key = prepare_volatile_key(
        fixture,
        &PreparePortableIssuerKey {
            operation_id: [21; 32],
            custody: PortableIssuerCustody::TrustedVolatileLocal,
        },
    )
    .unwrap();
    let request = portable_registration_request(
        fixture,
        fixture
            .pic()
            .get_time()
            .unwrap()
            .as_nanos_since_unix_epoch(),
        key.public_key,
    );
    let challenge: Result<PortableIssuerChallenge, PortableIssuerRegistrationRefusal> =
        ic_testkit::pic::update_wire(
            fixture.pic(),
            fixture.root,
            Principal::anonymous(),
            registration_spec(
                canic::protocol::portable_enrollment::registration::prepare::METHOD,
                canic::protocol::portable_enrollment::registration::prepare::METHOD_ID,
            ),
            &request,
        )
        .unwrap();
    let result: Result<PortableIssuerEnrollment, PortableIssuerRegistrationRefusal> =
        ic_testkit::pic::update_wire(
            fixture.pic(),
            fixture.issuer.canister_id,
            fixture.root,
            registration_spec(
                "test_register_volatile_portable_issuer",
                0x5445_5354_0000_080a,
            ),
            &challenge.unwrap(),
        )
        .unwrap();
    result.unwrap();
}

fn run_client(input: &serde_json::Value) {
    run_script("auth-transport.mjs", input);
}

fn run_script(script: &str, input: &serde_json::Value) {
    let root = canic_tests::workspace::discover().unwrap();
    let mut child = Command::new("node")
        .arg(
            root.join("crates/canic-host/examples/frontend-consumer")
                .join(script),
        )
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(&serde_json::to_vec(input).unwrap())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    println!("{}", String::from_utf8_lossy(&output.stdout));
}

#[test]
#[ignore = "requires prepared frontend npm dependencies and an explicit Chromium executable"]
fn delivered_client_reopens_durable_logout() {
    let mut fixture = canic_tests::pic::setup_http_active_component_registry();
    let host = fixture.fresh_pic_mut().unwrap().make_live(None).unwrap();
    enroll_live(&fixture);
    let targets = [fixture.issuer.canister_id, fixture.verifier.canister_id];
    let devices = [63, 64];
    let pic = fixture.pic();
    for target in targets {
        await_missing_session(pic, target, 63, Duration::from_secs(30));
    }
    let issued: Vec<_> = devices
        .iter()
        .map(|device| {
            issue_fixture(
                &fixture,
                portable_issue_request(&fixture, *device),
                Principal::from_slice(&[43]),
            )
            .unwrap()
        })
        .collect();
    let input = serde_json::json!({
        "host": host.as_str(), "rootKey": pic.root_key().unwrap().unwrap(),
        "issuer": fixture.issuer.canister_id.to_text(),
        "targets": targets.map(|target| target.to_text()), "devices": devices,
        "credentials": issued.iter().map(|row| ic_auth_wire::CredentialEnvelope::encode_request(&row.credential).unwrap()).collect::<Vec<_>>(),
        "until": issued.iter().map(|row| row.admission_deadline_ns.to_string()).collect::<Vec<_>>(),
        "scopes": ["app00:xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"]
    });
    run_script("auth-browser-runner.mjs", &input);
    for target in targets {
        assert_eq!(
            portable_read(pic, target, portable_browser(63)),
            Err(ic_auth_wire::model::SessionReadRefusal::MissingOrExpired)
        );
        let peer = portable_read(pic, target, portable_browser(64)).unwrap();
        assert_eq!(peer.caller, portable_browser(64).into());
    }
    fixture.fresh_pic_mut().unwrap().stop_live().unwrap();
    drop(fixture);
}
