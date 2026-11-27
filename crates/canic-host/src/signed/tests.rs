use super::{Call, admit};
use crate::{Artifact, Error, Network, Plan, network::principal};
use base64::{Engine, prelude::BASE64_STANDARD};
use ic_platform::{
    ed25519::{Signer, SigningKey},
    ingress::{self, Content},
    native::{PreparedRequestStatus, PreparedUpdate, ReadError},
};

fn fixture_expiry(expired: bool) -> (Vec<u8>, Plan, Call<'static>) {
    let time = crate::now().unwrap();
    let plan = Plan {
        deployment: None,
        version: 1,
        network: Network::Local {
            url: "http://127.0.0.1:8000/".into(),
            root_key: crate::hex(include_bytes!("../network/mainnet-root.der")),
            root_subnet: Some(crate::network::MAINNET_ROOT_SUBNET.into()),
            routing_canister: "aaaaa-aa".into(),
        },
        controller: controller(),
        artifact: Artifact {
            sha256: "00".repeat(32),
            bytes: 8,
        },
        action: crate::PlanAction::Create {
            bootstrap: None,
            cycles: 1_000_000_000_000,
        },
        created_at_ns: time,
        expires_at_ns: time + 3_600_000_000_000,
    };
    let call = Call {
        canister: principal("aaaaa-aa").unwrap(),
        effective: principal("aaaaa-aa").unwrap(),
        method: "test_effect",
        args: vec![1, 2, 3],
    };
    let bytes = signed_fixture(
        &plan,
        &call,
        if expired {
            time - 1
        } else {
            time + 300_000_000_000
        },
    );
    (bytes, plan, call)
}

// Public synthetic key, with no user identity or production signing path involved.
pub(crate) fn controller() -> String {
    ic_platform_types::Principal::self_authenticating(public_key()).to_text()
}

fn child_source() -> crate::StoredArtifact {
    crate::tests::stored_source("child", b"test")
}

fn public_key() -> Vec<u8> {
    let mut der = crate::unhex("302a300506032b6570032100").unwrap();
    der.extend_from_slice(SigningKey::from_bytes(&[9; 32]).verifying_key().as_bytes());
    der
}

fn sign_envelope(envelope: &mut ingress::Envelope) {
    let signature = SigningKey::from_bytes(&[9; 32]).sign(&envelope.content.signable().unwrap());
    envelope.sender_sig = Some(signature.to_bytes().to_vec());
    envelope.sender_pubkey = Some(public_key());
}

pub(crate) fn signed_fixture(plan: &Plan, call: &Call<'_>, expiry: u64) -> Vec<u8> {
    assert_eq!(plan.controller, controller());
    let sender = principal(&plan.controller).unwrap();
    let mut update = ingress::Envelope {
        content: Content::Call {
            nonce: None,
            ingress_expiry: expiry,
            sender,
            canister_id: call.canister,
            method_name: call.method.into(),
            arg: call.args.clone(),
            sender_info: None,
        },
        sender_pubkey: None,
        sender_sig: None,
        sender_delegation: None,
    };
    sign_envelope(&mut update);
    let request_id = update.content.request_id().unwrap();
    let update_bytes = ingress::encode_envelope(&update, 4096).unwrap();
    let mut status = update;
    status.content = Content::ReadState {
        sender,
        ingress_expiry: expiry,
        paths: vec![vec![b"request_status".to_vec(), request_id.to_vec()]],
    };
    sign_envelope(&mut status);
    let status = ingress::encode_envelope(&status, 4096).unwrap();
    let message = serde_json::json!({"format":"icp-signed-message", "version":1,
        "request": {"type":"update", "envelope":BASE64_STANDARD.encode(&update_bytes), "request_id":crate::hex(&request_id), "status_check":BASE64_STANDARD.encode(&status)},
        "network":{"url":plan.network.url(),"root_key":plan.network.root()}, "destination":{"canister":call.effective.to_text()}});
    serde_json::to_vec(&message).unwrap()
}

fn fixture() -> (Vec<u8>, Plan, Call<'static>) {
    fixture_expiry(false)
}

#[tokio::test]
async fn maintenance_observation_cannot_sign_another_root_or_effect() {
    let (_, mut plan, mut call) = fixture();
    plan.version = 1;
    plan.action = crate::PlanAction::MaintainRoot {
        store: None,
        target: crate::UpgradeTarget {
            canister: "rrkah-fqaaa-aaaaa-aaaaq-cai".into(),
            module: "11".repeat(32),
        },
        children: vec![canic_management_protocol::custody::Custody {
            creation: canic_management_protocol::Status {
                approval: Some(canic_management_protocol::Approval {
                    manager: "rrkah-fqaaa-aaaaa-aaaaq-cai".into(),
                    module_hash: [1; 32],
                    creation_cycles: canic_management_protocol::CREATION_CYCLES,
                    execute_at_ns: 1,
                    expires_at_ns: 2,
                }),
                worker: Some("ryjl3-tyaaa-aaaaa-aaaba-cai".into()),
                phase: canic_management_protocol::Phase::Ready,
                ..Default::default()
            },
            upgrades: vec![],
        }],
    };
    let client = plan.network.client().unwrap();
    let dir = crate::tests::directory();
    for case in 0..4 {
        call.canister = principal(if case == 0 {
            "ryjl3-tyaaa-aaaaa-aaaba-cai"
        } else {
            "rrkah-fqaaa-aaaaa-aaaaq-cai"
        })
        .unwrap();
        call.effective = call.canister;
        call.method = if case == 1 {
            "install_code"
        } else {
            "observe_custody"
        };
        call.args = ic_wire::encode(
            &(),
            ic_wire::Encoding::Frame {
                method: canic_management_protocol::custody::OBSERVE,
                direction: ic_wire::Direction::Request,
            },
            canic_management_protocol::custody::LIMITS,
        )
        .unwrap();
        if case == 2 {
            call.args.push(0);
        }
        if case == 3 {
            let crate::PlanAction::MaintainRoot { children, .. } = &mut plan.action else {
                panic!("maintenance")
            };
            children[0].creation.phase = canic_management_protocol::Phase::CreateIssued;
        }
        assert!(matches!(
            super::observe(
                &dir,
                &plan,
                &call,
                "must-not-run",
                std::path::Path::new("must-not-run"),
                &client
            )
            .await,
            Err(Error::Invalid(_))
        ));
        assert!(!dir.join("signed.json").exists());
    }
}

#[tokio::test]
async fn recovery_retains_exact_attempt_and_deadline_after_effect_expiry() {
    let (_, mut plan, _) = fixture();
    plan.action = crate::PlanAction::CreateChild(crate::ManagedOwner {
        child_evidence_only: false,
        arguments: None,
        blob_attempt: None,
        source: child_source(),
        slot: canic_management_protocol::children::ChildId::FIRST,
        root: "rrkah-fqaaa-aaaaa-aaaaq-cai".into(),
        root_module: "00".repeat(32),
        execute_at_ns: 1,
    });
    plan.expires_at_ns = 2;
    let before = plan.clone();
    let call = crate::managed::recovery::call(&plan).unwrap();
    let expiry = crate::now().unwrap() - 1;
    let signed = signed_fixture(&plan, &call, expiry);
    let dir = crate::tests::directory();
    crate::store::put_json(&dir.join("authority.json"), &expiry).unwrap();
    crate::store::put(&dir.join("signed.json"), &signed).unwrap();
    let client = plan.network.client().unwrap();
    let signer = std::path::Path::new("must-not-run");
    for _ in 0..2 {
        assert!(matches!(
            super::reconcile(&dir, &plan, "unused", signer, &client).await,
            Err(Error::Blocked(_))
        ));
        assert_eq!(std::fs::read(dir.join("signed.json")).unwrap(), signed);
        assert_eq!(
            crate::store::json::<u64>(&dir.join("authority.json")).unwrap(),
            expiry
        );
    }
    // A result already committed before host interruption can be read after expiry.
    crate::store::put(&dir.join("reply.bin"), b"retained reply").unwrap();
    assert_eq!(
        super::reconcile(&dir, &plan, "unused", signer, &client)
            .await
            .unwrap(),
        b"retained reply"
    );
    assert_eq!(plan, before);
    plan.expires_at_ns += 1;
    assert!(matches!(
        super::reconcile(&dir, &plan, "unused", signer, &client).await,
        Err(Error::Invalid(_))
    ));
    std::fs::remove_dir_all(dir).unwrap();
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
        Err(Error::Supplier(_))
    ));
}

#[test]
fn admission_preserves_bytes_and_keeps_authentication_profiles_distinct() {
    let (bytes, plan, call) = fixture();
    let attempt = admit(&bytes, &super::Authority::from(&plan), &call).unwrap();
    let effective = call.effective;
    let update =
        PreparedUpdate::from_bytes(attempt.update.envelope_bytes(), effective, 4096).unwrap();
    let status =
        PreparedRequestStatus::from_bytes(attempt.status.envelope_bytes(), &update, 4096).unwrap();
    assert_eq!(update.envelope_bytes(), attempt.update.envelope_bytes());
    assert_eq!(status.envelope_bytes(), attempt.status.envelope_bytes());
    assert_eq!(update.request_id(), attempt.update.request_id().as_slice());
    assert_eq!(update.expiry(), attempt.update.expiry());
    assert_eq!(status.expiry(), attempt.update.expiry());

    // The installed ICP anonymous journey is a supported existing profile.
    // The approved controller selects it explicitly, never a failed signature.
    let mut message: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let mut anonymous_plan = plan;
    anonymous_plan.controller = "2vxsx-fae".into();
    let mut anonymous_update = ingress::decode_envelope(attempt.update.envelope_bytes(), 4096)
        .unwrap()
        .value;
    let Content::Call { sender, .. } = &mut anonymous_update.content else {
        unreachable!()
    };
    *sender = anonymous_plan.controller.parse().unwrap();
    anonymous_update.sender_sig = None;
    anonymous_update.sender_pubkey = None;
    let id = anonymous_update.content.request_id().unwrap();
    let anonymous_update = ingress::encode_envelope(&anonymous_update, 4096).unwrap();
    let mut anonymous_status = ingress::decode_envelope(attempt.status.envelope_bytes(), 4096)
        .unwrap()
        .value;
    let Content::ReadState { sender, paths, .. } = &mut anonymous_status.content else {
        unreachable!()
    };
    *sender = anonymous_plan.controller.parse().unwrap();
    paths[0][1] = id.to_vec();
    anonymous_status.sender_sig = None;
    anonymous_status.sender_pubkey = None;
    message["request"]["request_id"] = crate::hex(&id).into();
    message["request"]["envelope"] = BASE64_STANDARD.encode(&anonymous_update).into();
    message["request"]["status_check"] = BASE64_STANDARD
        .encode(ingress::encode_envelope(&anonymous_status, 4096).unwrap())
        .into();
    let admitted = admit(
        &serde_json::to_vec(&message).unwrap(),
        &super::Authority::from(&anonymous_plan),
        &call,
    )
    .unwrap();
    assert_eq!(admitted.update.envelope_bytes(), anonymous_update);
    assert_eq!(
        admitted.status.envelope_bytes(),
        ingress::encode_envelope(&anonymous_status, 4096).unwrap()
    );
    assert!(matches!(
        PreparedUpdate::from_bytes(&anonymous_update, effective, 4096),
        Err(ReadError::Packet)
    ));
    for field in ["envelope", "status_check"] {
        let raw = BASE64_STANDARD
            .decode(message["request"][field].as_str().unwrap())
            .unwrap();
        for authentication in ["sender_pubkey", "sender_sig", "sender_delegation"] {
            let mut packet = ingress::decode_envelope(&raw, 4096).unwrap().value;
            match authentication {
                "sender_pubkey" => packet.sender_pubkey = Some(vec![]),
                "sender_sig" => packet.sender_sig = Some(vec![]),
                "sender_delegation" => packet.sender_delegation = Some(vec![]),
                _ => unreachable!(),
            }
            let mut mixed = message.clone();
            mixed["request"][field] = BASE64_STANDARD
                .encode(ingress::encode_envelope(&packet, 4096).unwrap())
                .into();
            assert!(
                admit(
                    &serde_json::to_vec(&mixed).unwrap(),
                    &super::Authority::from(&anonymous_plan),
                    &call
                )
                .is_err()
            );
        }
    }
}

#[test]
fn tampered_signatures_never_fall_back_to_anonymous_admission() {
    let (bytes, plan, call) = fixture();
    let message: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    for field in ["envelope", "status_check"] {
        let original = BASE64_STANDARD
            .decode(message["request"][field].as_str().unwrap())
            .unwrap();
        let mut envelope = ingress::decode_envelope(&original, 4096).unwrap().value;
        envelope.sender_sig.as_mut().unwrap()[0] ^= 1;
        let mut changed = message.clone();
        changed["request"][field] = BASE64_STANDARD
            .encode(ingress::encode_envelope(&envelope, 4096).unwrap())
            .into();
        assert!(matches!(
            admit(
                &serde_json::to_vec(&changed).unwrap(),
                &super::Authority::from(&plan),
                &call
            ),
            Err(Error::Supplier(_))
        ));
    }
}

#[test]
fn valid_status_signature_cannot_renew_the_attempt_window() {
    let (bytes, plan, call) = fixture();
    let mut message: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let original = BASE64_STANDARD
        .decode(message["request"]["status_check"].as_str().unwrap())
        .unwrap();
    let mut status = ingress::decode_envelope(&original, 4096).unwrap().value;
    let Content::ReadState { ingress_expiry, .. } = &mut status.content else {
        unreachable!()
    };
    *ingress_expiry += 1;
    sign_envelope(&mut status);
    message["request"]["status_check"] = BASE64_STANDARD
        .encode(ingress::encode_envelope(&status, 4096).unwrap())
        .into();
    assert!(matches!(
        admit(
            &serde_json::to_vec(&message).unwrap(),
            &super::Authority::from(&plan),
            &call
        ),
        Err(Error::Invalid(_))
    ));
}

#[test]
fn packet_admission_retains_bytes_and_refuses_malformed_packets() {
    let (bytes, plan, call) = fixture();
    let original: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let admitted = admit(&bytes, &super::Authority::from(&plan), &call).unwrap();
    for (field, retained) in [
        ("envelope", admitted.update.envelope_bytes()),
        ("status_check", admitted.status.envelope_bytes()),
    ] {
        let raw = BASE64_STANDARD
            .decode(original["request"][field].as_str().unwrap())
            .unwrap();
        assert_eq!(
            retained, raw,
            "admission must retain original authentication bytes"
        );
        // Platform owns structural CBOR rejection. This consumer check proves
        // that a malformed original packet in either signed-message field refuses.
        let mut changed = original.clone();
        changed["request"][field] = BASE64_STANDARD.encode(&raw[..raw.len() - 1]).into();
        assert!(
            admit(
                &serde_json::to_vec(&changed).unwrap(),
                &super::Authority::from(&plan),
                &call
            )
            .is_err()
        );
    }
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
    crate::output_publication::prepare(&path, 0o700).unwrap();
    crate::store::put(&path.join("signed.json"), &bytes).unwrap();
    let client = plan.network.client().unwrap();
    let result = super::execute(
        &path,
        &plan,
        &call,
        "not-an-identity",
        std::path::Path::new("/missing-signer"),
        &client,
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
    plan.action = crate::PlanAction::CreateChild(crate::ManagedOwner {
        child_evidence_only: false,
        arguments: None,
        blob_attempt: None,
        source: child_source(),
        slot: canic_management_protocol::children::ChildId::FIRST,
        root: "rrkah-fqaaa-aaaaa-aaaaq-cai".into(),
        root_module: "00".repeat(32),
        execute_at_ns: 1,
    });
    plan.expires_at_ns = 1;
    call.canister = principal("rrkah-fqaaa-aaaaa-aaaaq-cai").unwrap();
    call.effective = call.canister;
    let client = plan.network.client().unwrap();
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
        let owner = plan.management().unwrap().clone();
        plan.action = if upgrade {
            crate::PlanAction::UpgradeChild {
                target: crate::UpgradeTarget {
                    canister: "ryjl3-tyaaa-aaaaa-aaaba-cai".into(),
                    module: "11".repeat(32),
                },
                owner,
            }
        } else {
            crate::PlanAction::CreateChild(owner)
        };
        call.method = method;
        assert!(matches!(
            super::observe(
                &dir,
                &plan,
                &call,
                "must-not-run",
                std::path::Path::new("must-not-run"),
                &client
            )
            .await,
            Err(Error::Invalid(_))
        ));
    }
    // Even the correct read method cannot use arbitrary payload bytes.
    call.method = "observe_root_custody";
    assert!(matches!(
        super::observe(
            &dir,
            &plan,
            &call,
            "must-not-run",
            std::path::Path::new("must-not-run"),
            &client
        )
        .await,
        Err(Error::Invalid(_))
    ));
    assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 0);
    std::fs::remove_dir(dir).unwrap();
}

#[tokio::test]
async fn stored_observation_rejects_other_contracts_before_signing() {
    let (_, mut plan, _) = fixture();
    plan.action = crate::PlanAction::CreateChild(crate::ManagedOwner {
        child_evidence_only: false,
        arguments: None,
        blob_attempt: None,
        source: child_source(),
        root: "rrkah-fqaaa-aaaaa-aaaaq-cai".into(),
        root_module: "00".repeat(32),
        execute_at_ns: 1,
        slot: canic_management_protocol::children::ChildId::SECOND,
    });
    let client = plan.network.client().unwrap();
    let dir = crate::tests::directory();
    for method in ["observe_child", "observe", "observe_child_deployments"] {
        let mut call = crate::managed::observation(&plan).unwrap();
        call.method = method;
        assert!(matches!(
            super::observe(
                &dir,
                &plan,
                &call,
                "must-not-run",
                std::path::Path::new("must-not-run"),
                &client
            )
            .await,
            Err(Error::Invalid(_))
        ));
    }
    assert!(!dir.join("signed.json").exists());
}

#[tokio::test]
async fn publication_read_authority_survives_effect_expiry_without_renewing_mutations() {
    let (_, mut plan, _) = fixture();
    let now = crate::now().unwrap();
    plan.version = 1;
    plan.created_at_ns = now - 3_600_000_000_000;
    plan.expires_at_ns = now - 1;
    plan.deployment = Some(crate::DeploymentName {
        app: "demo".into(),
        fleet: "local".into(),
        canister: "app".into(),
    });
    plan.action = crate::PlanAction::Publish(crate::Publication {
        after_removal: None,
        root: crate::UpgradeTarget {
            canister: "rrkah-fqaaa-aaaaa-aaaaq-cai".into(),
            module: "11".repeat(32),
        },
        store: crate::UpgradeTarget {
            canister: "ryjl3-tyaaa-aaaaa-aaaba-cai".into(),
            module: "22".repeat(32),
        },
        release: ic_wasm_store_protocol::Release {
            name: "app".into(),
            version: "v1".into(),
        },
    });
    let path = crate::tests::directory();
    let deadline = now + 300_000_000_000;
    let call = crate::publication::observation(&plan, deadline).unwrap();
    crate::store::put_json(&path.join("authority.json"), &(deadline + 300_000_000_000)).unwrap();
    crate::store::put(
        &path.join("signed.json"),
        &signed_fixture(&plan, &call, deadline),
    )
    .unwrap();
    crate::store::put(&path.join("reply.bin"), &[42]).unwrap();
    let client = plan.network.client().unwrap();
    let absent = path.join("no-signer");
    assert_eq!(
        super::observe_operation(&path, &plan, "unused", &absent, &client)
            .await
            .unwrap(),
        [42]
    );
    assert_eq!(super::retained_operation(&path, &plan).unwrap(), [42]);
    let mutation = crate::publication::call(
        &plan,
        canic_management_protocol::publication::Command::Publish(
            plan.publication().unwrap().release.clone(),
        ),
        deadline,
    )
    .unwrap();
    let bytes = signed_fixture(&plan, &mutation, deadline);
    // Fresh read authority cannot admit a mutation, even with the same release and target.
    std::fs::write(path.join("signed.json"), bytes).unwrap();
    assert!(super::retained_operation(&path, &plan).is_err());
    std::fs::remove_file(path.join("reply.bin")).unwrap();
    assert!(
        super::execute(&path, &plan, &mutation, "unused", &absent, &client)
            .await
            .is_err()
    );
    assert!(!absent.exists());
}

#[tokio::test]
async fn store_readiness_authority_cannot_change_target_or_renew_handover() {
    let (_, mut plan, _) = fixture();
    let now = crate::now().unwrap();
    plan.version = 1;
    plan.created_at_ns = now - 3_600_000_000_000;
    plan.expires_at_ns = now - 1;
    plan.deployment = Some(crate::DeploymentName {
        app: "demo".into(),
        fleet: "local".into(),
        canister: "store".into(),
    });
    plan.action = crate::PlanAction::CreateStore {
        cycles: 1_000_000_000_000,
        root: crate::UpgradeTarget {
            canister: "rrkah-fqaaa-aaaaa-aaaaq-cai".into(),
            module: "11".repeat(32),
        },
    };
    let canister = principal("ryjl3-tyaaa-aaaaa-aaaba-cai").unwrap();
    let path = crate::tests::directory();
    let deadline = now + 300_000_000_000;
    let call = crate::store_bootstrap::observation(&plan, canister, deadline).unwrap();
    crate::store::put_json(&path.join("authority.json"), &(deadline + 300_000_000_000)).unwrap();
    crate::store::put(
        &path.join("signed.json"),
        &signed_fixture(&plan, &call, deadline),
    )
    .unwrap();
    crate::store::put(&path.join("reply.bin"), &[42]).unwrap();
    let client = plan.network.client().unwrap();
    let absent = path.join("no-signer");
    assert_eq!(
        super::observe_store(&path, &plan, canister, "unused", &absent, &client)
            .await
            .unwrap(),
        [42]
    );
    assert_eq!(super::retained_store(&path, &plan, canister).unwrap(), [42]);
    assert!(
        super::retained_store(
            &path,
            &plan,
            principal("rrkah-fqaaa-aaaaa-aaaaq-cai").unwrap()
        )
        .is_err()
    );
    let handover = crate::store_bootstrap::handover(&plan, canister).unwrap();
    std::fs::write(
        path.join("signed.json"),
        signed_fixture(&plan, &handover, deadline),
    )
    .unwrap();
    assert!(super::retained_store(&path, &plan, canister).is_err());
    assert!(super::validate_retained(&path, &plan, &handover).is_err());
    std::fs::remove_file(path.join("reply.bin")).unwrap();
    assert!(
        super::execute(&path, &plan, &handover, "unused", &absent, &client)
            .await
            .is_err()
    );
    assert!(!absent.exists());
    std::fs::remove_dir_all(path).unwrap();
}
