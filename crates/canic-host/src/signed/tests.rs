use super::{Call, admit};
use crate::{Artifact, Error, Network, Plan, network::principal};
use base64::{Engine, prelude::BASE64_STANDARD};
use ic_platform::{
    agent::Agent,
    ic_protocol::{self, EnvelopeContent},
};
use std::borrow::Cow;

fn fixture_expiry(expired: bool) -> (Vec<u8>, Plan, Call<'static>) {
    let time = crate::now().unwrap();
    let plan = Plan {
        deployment: None,
        version: 1,
        management: None,
        network: Network::Local {
            url: "http://127.0.0.1:8000/".into(),
            root_key: "00".repeat(133),
            routing_canister: "aaaaa-aa".into(),
        },
        controller: "2vxsx-fae".into(),
        artifact: Artifact {
            sha256: "00".repeat(32),
            bytes: 8,
        },
        target: None,
        previous_module: None,
        creation_cycles: 1_000_000_000_000,
        created_at_ns: time,
        expires_at_ns: time + 3_600_000_000_000,
    };
    let call = Call {
        canister: principal("aaaaa-aa").unwrap(),
        effective: principal("aaaaa-aa").unwrap(),
        method: "test_effect",
        args: vec![1, 2, 3],
    };
    let agent = Agent::builder()
        .with_url("http://127.0.0.1:8000")
        .build()
        .unwrap();
    let mut update = agent
        .update(&"aaaaa-aa".parse().unwrap(), call.method)
        .with_arg(call.args.as_slice())
        .sign()
        .unwrap();
    if expired {
        let mut envelope = ic_protocol::decode_envelope(&update.signed_update, 4096).unwrap();
        let EnvelopeContent::Call { ingress_expiry, .. } = envelope.content.to_mut() else {
            unreachable!()
        };
        *ingress_expiry = time - 1;
        update.ingress_expiry = *ingress_expiry;
        update.request_id = ic_platform::agent::to_request_id(envelope.content.as_ref()).unwrap();
        update.signed_update = ic_protocol::encode_envelope(&envelope, 4096).unwrap();
    }
    let mut status = ic_protocol::decode_envelope(&update.signed_update, 4096).unwrap();
    status.content = Cow::Owned(EnvelopeContent::ReadState {
        sender: update.sender,
        ingress_expiry: update.ingress_expiry,
        paths: vec![vec![
            b"request_status".to_vec().into(),
            update.request_id.to_vec().into(),
        ]],
    });
    let status = ic_protocol::encode_envelope(&status, 4096).unwrap();
    let message = serde_json::json!({"format":"icp-signed-message", "version":1,
        "request": {"type":"update", "envelope":BASE64_STANDARD.encode(&update.signed_update), "request_id":crate::hex(update.request_id.as_slice()), "status_check":BASE64_STANDARD.encode(&status)},
        "network":{"url":plan.network.url(),"root_key":plan.network.root()}, "destination":{"canister":"aaaaa-aa"}});
    (serde_json::to_vec(&message).unwrap(), plan, call)
}

fn fixture() -> (Vec<u8>, Plan, Call<'static>) {
    fixture_expiry(false)
}

#[test]
fn exact_attempt_binding_rejects_changed_authority_argument_network_and_id() {
    let (bytes, plan, call) = fixture();
    assert!(admit(&bytes, &super::Authority::from(&plan), &call).is_ok());
    let mut different = plan.clone();
    different.controller = "aaaaa-aa".into();
    assert!(matches!(
        admit(&bytes, &super::Authority::from(&different), &call),
        Err(Error::Invalid(_))
    ));
    let mut different = plan.clone();
    different.network = Network::Mainnet;
    assert!(matches!(
        admit(&bytes, &super::Authority::from(&different), &call),
        Err(Error::Invalid(_))
    ));
    let changed = Call {
        args: vec![1, 2, 4],
        ..call
    };
    assert!(matches!(
        admit(&bytes, &super::Authority::from(&plan), &changed),
        Err(Error::Invalid(_))
    ));
    let mut changed: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    changed["request"]["request_id"] = "00".repeat(32).into();
    assert!(matches!(
        admit(
            &serde_json::to_vec(&changed).unwrap(),
            &super::Authority::from(&plan),
            &call
        ),
        Err(Error::Invalid(_))
    ));
}

#[test]
fn status_authority_must_bind_the_same_request() {
    let (bytes, plan, call) = fixture();
    let mut changed: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    changed["request"]["status_check"] = changed["request"]["envelope"].clone();
    assert!(matches!(
        admit(
            &serde_json::to_vec(&changed).unwrap(),
            &super::Authority::from(&plan),
            &call
        ),
        Err(Error::Invalid(_))
    ));
}

#[tokio::test(flavor = "current_thread")]
async fn expired_uncertain_attempt_stays_retained_without_a_new_signature() {
    let (bytes, plan, call) = fixture_expiry(true);
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .join("target/host-tests")
        .join(format!(
            "expired-{}-{}",
            std::process::id(),
            crate::now().unwrap()
        ));
    ic_host_fs::durable::prepare_private_parents(&path).unwrap();
    crate::store::put(&path.join("signed.json"), &bytes).unwrap();
    let agent = plan.network.agent().unwrap();
    let result = super::execute(
        &path,
        &plan,
        &call,
        "not-an-identity",
        std::path::Path::new("/missing-signer"),
        &agent,
    )
    .await;
    assert!(matches!(result, Err(Error::Blocked(_))));
    assert_eq!(
        crate::store::read(&path.join("signed.json"), 4096).unwrap(),
        bytes
    );
    assert!(!path.join("reply.bin").exists());
}

#[tokio::test]
async fn observation_authority_cannot_dispatch_effects_after_approval_expiry() {
    let (_, mut plan, mut call) = fixture_expiry(false);
    plan.management = Some(crate::ManagedOwner {
        root: "rrkah-fqaaa-aaaaa-aaaaq-cai".into(),
        root_module: "00".repeat(32),
        execute_at_ns: 1,
    });
    plan.expires_at_ns = 1;
    call.canister = principal("rrkah-fqaaa-aaaaa-aaaaq-cai").unwrap();
    call.effective = call.canister;
    let agent = plan.network.agent().unwrap();
    let dir = crate::tests::directory();
    for (upgrade, method) in [
        (false, "stage"),
        (false, "reconcile"),
        (false, "install_code"),
        (false, "observe_upgrade"),
        (true, "stage_upgrade"),
        (true, "reconcile_upgrade"),
        (true, "install_chunked_code"),
        (true, "observe"),
    ] {
        plan.target = upgrade.then(|| "ryjl3-tyaaa-aaaaa-aaaba-cai".into());
        call.method = method;
        assert!(matches!(
            super::observe(
                &dir,
                &plan,
                &call,
                "must-not-run",
                std::path::Path::new("must-not-run"),
                &agent
            )
            .await,
            Err(Error::Invalid(_))
        ));
    }
    // Even the correct read method cannot use arbitrary payload bytes.
    call.method = "observe_upgrade";
    assert!(matches!(
        super::observe(
            &dir,
            &plan,
            &call,
            "must-not-run",
            std::path::Path::new("must-not-run"),
            &agent
        )
        .await,
        Err(Error::Invalid(_))
    ));
    assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 0);
    std::fs::remove_dir(dir).unwrap();
}
