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
        action: crate::PlanAction::Create {
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

fn signed_fixture(plan: &Plan, call: &Call<'_>, expiry: u64) -> Vec<u8> {
    let agent = Agent::builder()
        .with_url(plan.network.url())
        .build()
        .unwrap();
    let mut update = agent
        .update(&call.canister.to_text().parse().unwrap(), call.method)
        .with_arg(call.args.as_slice())
        .sign()
        .unwrap();
    {
        let mut envelope = ic_protocol::decode_envelope(&update.signed_update, 4096).unwrap();
        let EnvelopeContent::Call { ingress_expiry, .. } = envelope.content.to_mut() else {
            unreachable!()
        };
        *ingress_expiry = expiry;
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
        "network":{"url":plan.network.url(),"root_key":plan.network.root()}, "destination":{"canister":call.effective.to_text()}});
    serde_json::to_vec(&message).unwrap()
}

fn fixture() -> (Vec<u8>, Plan, Call<'static>) {
    fixture_expiry(false)
}

#[tokio::test]
async fn maintenance_observation_cannot_sign_another_root_or_effect() {
    let (_, mut plan, mut call) = fixture();
    plan.version = 2;
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
    let agent = plan.network.agent().unwrap();
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
        call.args = ic_wire::native::encode_request(
            canic_management_protocol::custody::OBSERVE,
            &(),
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
                &agent
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
        source: None,
        slot: None,
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
    let agent = plan.network.agent().unwrap();
    let signer = std::path::Path::new("must-not-run");
    for _ in 0..2 {
        assert!(matches!(
            super::reconcile(&dir, &plan, "unused", signer, &agent).await,
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
        super::reconcile(&dir, &plan, "unused", signer, &agent)
            .await
            .unwrap(),
        b"retained reply"
    );
    assert_eq!(plan, before);
    plan.expires_at_ns += 1;
    assert!(matches!(
        super::reconcile(&dir, &plan, "unused", signer, &agent).await,
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
    plan.action = crate::PlanAction::CreateChild(crate::ManagedOwner {
        source: None,
        slot: None,
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

#[tokio::test]
async fn scoped_observation_rejects_sibling_and_legacy_payloads_before_signing() {
    let (_, mut plan, _) = fixture();
    plan.action = crate::PlanAction::CreateChild(crate::ManagedOwner {
        source: None,
        root: "rrkah-fqaaa-aaaaa-aaaaq-cai".into(),
        root_module: "00".repeat(32),
        execute_at_ns: 1,
        slot: Some(canic_management_protocol::children::Slot::Second),
    });
    let agent = plan.network.agent().unwrap();
    let dir = crate::tests::directory();
    for slot in [None, Some(canic_management_protocol::children::Slot::First)] {
        let mut other = plan.clone();
        let crate::PlanAction::CreateChild(owner) = &mut other.action else {
            panic!("child creation")
        };
        owner.slot = slot;
        let call = crate::managed::observation(&other).unwrap();
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
    assert!(!dir.join("signed.json").exists());
}

#[tokio::test]
async fn publication_read_authority_survives_effect_expiry_without_renewing_mutations() {
    let (_, mut plan, _) = fixture();
    let now = crate::now().unwrap();
    plan.version = 3;
    plan.created_at_ns = now - 3_600_000_000_000;
    plan.expires_at_ns = now - 1;
    plan.deployment = Some(crate::DeploymentName {
        app: "demo".into(),
        fleet: "local".into(),
        canister: "app".into(),
    });
    plan.action = crate::PlanAction::Publish(crate::Publication {
        root: crate::UpgradeTarget {
            canister: "rrkah-fqaaa-aaaaa-aaaaq-cai".into(),
            module: "11".repeat(32),
        },
        store: crate::UpgradeTarget {
            canister: "ryjl3-tyaaa-aaaaa-aaaba-cai".into(),
            module: "22".repeat(32),
        },
        release: canic_wasm_store_protocol::Release {
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
    let agent = plan.network.agent().unwrap();
    let absent = path.join("no-signer");
    assert_eq!(
        super::observe_operation(&path, &plan, "unused", &absent, &agent)
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
        super::execute(&path, &plan, &mutation, "unused", &absent, &agent)
            .await
            .is_err()
    );
    assert!(!absent.exists());
}

#[tokio::test]
async fn store_readiness_authority_cannot_change_target_or_renew_handover() {
    let (_, mut plan, _) = fixture();
    let now = crate::now().unwrap();
    plan.version = 2;
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
    let agent = plan.network.agent().unwrap();
    let absent = path.join("no-signer");
    assert_eq!(
        super::observe_store(&path, &plan, canister, "unused", &absent, &agent)
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
        super::execute(&path, &plan, &handover, "unused", &absent, &agent)
            .await
            .is_err()
    );
    assert!(!absent.exists());
    std::fs::remove_dir_all(path).unwrap();
}
