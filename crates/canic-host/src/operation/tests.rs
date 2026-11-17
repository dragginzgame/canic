use super::{create_call, created};
use crate::tests::wire_fixture;
use crate::{Artifact, Error, Network, Plan};
use ic_platform_types::Principal;

fn plan() -> Plan {
    let input = crate::tests::input();
    Plan {
        deployment: None,
        version: 1,
        network: Network::Mainnet,
        controller: input.controller,
        artifact: Artifact {
            sha256: "00".repeat(32),
            bytes: 8,
        },
        action: crate::PlanAction::Create {
            cycles: input.creation_cycles,
        },
        created_at_ns: 10,
        expires_at_ns: 100,
    }
}

#[test]
fn replacement_codecs_preserve_retained_management_and_ledger_arguments() {
    let mut value = plan();
    let call = create_call(&value, None).unwrap();
    assert_eq!(call.args, wire_fixture("create-mainnet"));
    value.network = Network::Local {
        url: "http://127.0.0.1:4943/".into(),
        root_key: "00".repeat(133),
        root_subnet: None,
        routing_canister: "rrkah-fqaaa-aaaaa-aaaaq-cai".into(),
    };
    let call = create_call(&value, None).unwrap();
    assert_eq!(call.args, wire_fixture("create-local"));
    let target = Principal::from_slice(&[3; 29]);
    for upgrade in [false, true] {
        if upgrade {
            value.action = crate::PlanAction::Upgrade(crate::UpgradeTarget {
                canister: target.to_text(),
                module: "11".repeat(32),
            });
        }
        let call = super::install_call(&value, b"retained exact artifact", target).unwrap();
        assert_eq!(
            call.args,
            wire_fixture(if upgrade { "upgrade" } else { "install" })
        );
    }
    let current = ic_platform_protocol::management::CanisterArgs {
        canister_id: target,
    }
    .encode(1024)
    .unwrap();
    assert_eq!(current, wire_fixture("canister-args"));
    value.action = crate::PlanAction::CreateStore {
        cycles: 1_000_000_000_000,
        root: crate::UpgradeTarget {
            canister: target.to_text(),
            module: "11".repeat(32),
        },
    };
    let call = crate::store_bootstrap::handover(&value, target).unwrap();
    assert_eq!(call.args, wire_fixture("handover"));
}

#[test]
fn management_requests_match_the_previous_root_wire_schema() {
    use ic_platform_protocol::{management as mgmt, management_chunks as chunks};
    let target = ic_platform_types::Principal::from_slice(&[3; 29]);
    let creation = mgmt::CreateCanisterArgs {
        settings: Some(mgmt::CanisterSettings {
            controllers: Some(vec![target]),
            ..Default::default()
        }),
        sender_canister_version: Some(17),
    }
    .encode(4096)
    .unwrap();
    assert_eq!(creation, wire_fixture("root-create"));
    for store_canister in [None, Some(target)] {
        let installation = mgmt::InstallChunkedCodeArgs {
            mode: mgmt::InstallMode::Upgrade(None),
            target_canister: target,
            store_canister,
            chunk_hashes_list: vec![chunks::ChunkHash { hash: vec![1; 32] }],
            wasm_module_hash: vec![2; 32],
            arg: b"retained application arguments".to_vec(),
            sender_canister_version: Some(17),
        }
        .encode(8192)
        .unwrap();
        assert_eq!(
            installation,
            wire_fixture(if store_canister.is_some() {
                "chunked-store"
            } else {
                "chunked"
            })
        );
    }
}

#[test]
fn new_chunk_uploads_preserve_target_and_payload() {
    // Upload type-table order may differ. Issued upload commitments are retained,
    // never reconstructed or resent; installed owner-reopen tests cover that boundary.
    let target = ic_platform_types::Principal::from_slice(&[3; 29]);
    let bytes = ic_platform_protocol::management_chunks::encode_upload_chunk(
        target,
        b"retained chunk",
        4096,
    )
    .unwrap();
    assert_eq!(bytes, wire_fixture("upload-chunk"));
}

#[test]
fn direct_root_installation_has_ingress_headroom_and_a_finite_plan_ceiling() {
    let mut plan = plan();
    plan.artifact.bytes = crate::artifact::MAX_WASM;
    plan.validate().unwrap();
    let bytes = vec![7; crate::artifact::MAX_WASM];
    let target = Principal::from_slice(&[3; 29]);
    let call = super::install_call(&plan, &bytes, target).unwrap();
    assert!(
        call.args.len() < 2 * 1024 * 1024 - 64 * 1024,
        "reserve ingress envelope headroom"
    );
    assert!(call.args.windows(bytes.len()).any(|window| window == bytes));
    plan.artifact.bytes += 1;
    assert!(plan.validate().is_err());
}

#[test]
fn typed_status_preserves_json_evidence_and_absent_field_shapes() {
    use crate::{CanisterStatus, ManagementStatus, OperationStatus, StatusDetail};
    let mut status = CanisterStatus {
        operation: "original".into(),
        plan: plan(),
        effects: std::collections::BTreeMap::from([(
            "install".into(),
            "signed_attempt_retained_outcome_unknown".into(),
        )]),
        receipt: None,
        management: None,
        detail: StatusDetail::Deployment {},
    };
    let mut expected = serde_json::json!({
        "operation": "original", "plan": status.plan,
        "effects": {"install":"signed_attempt_retained_outcome_unknown"},
        "receipt":null, "management":null,
    });
    assert_eq!(serde_json::to_value(&status).unwrap(), expected);
    let creation = canic_management_protocol::Status {
        phase: canic_management_protocol::Phase::CreateIssued,
        attempts: [1, 0, 0],
        failure: Some(canic_management_protocol::Failure::Uncertain),
        ..Default::default()
    };
    expected["management"] = serde_json::to_value(&creation).unwrap();
    status.management = Some(ManagementStatus::Creation(creation));
    assert_eq!(serde_json::to_value(&status).unwrap(), expected);
    let upgrade = canic_management_protocol::upgrade::Status::default();
    expected["management"] = serde_json::to_value(&upgrade).unwrap();
    status.management = Some(ManagementStatus::Upgrade(upgrade));
    assert_eq!(serde_json::to_value(&status).unwrap(), expected);
    status.management = None;
    expected["management"] = serde_json::Value::Null;
    status.detail = StatusDetail::StoredChild {
        delivery: None,
        maintenance: None,
    };
    expected["delivery"] = serde_json::Value::Null;
    assert_eq!(serde_json::to_value(&status).unwrap(), expected);
    expected.as_object_mut().unwrap().remove("delivery");
    status.detail = StatusDetail::Publication { publication: None };
    expected["publication"] = serde_json::Value::Null;
    assert_eq!(serde_json::to_value(&status).unwrap(), expected);
    expected.as_object_mut().unwrap().remove("publication");
    expected["store_maintenance"] = serde_json::Value::Null;
    status.detail = StatusDetail::StoreMaintenance {
        store_maintenance: None,
    };
    assert_eq!(
        serde_json::to_value(OperationStatus::Canister(Box::new(status))).unwrap(),
        expected
    );
}

#[test]
fn mainnet_provider_request_binds_funding_identity_and_controller() {
    let plan = plan();
    let call = create_call(&plan, None).unwrap();
    assert_eq!(call.canister.to_text(), super::LEDGER);
    assert_eq!(call.method, "create_canister");
    assert_eq!(call.args, wire_fixture("create-mainnet"));
}

#[test]
fn store_creation_binds_root_subnet_without_changing_direct_placement() {
    let mut value = plan();
    let root = crate::UpgradeTarget {
        canister: "rrkah-fqaaa-aaaaa-aaaaq-cai".into(),
        module: "11".repeat(32),
    };
    value.action = crate::PlanAction::CreateStore {
        cycles: 1_000_000_000_000,
        root: root.clone(),
    };
    let subnet = crate::network::principal("r7inp-6aaaa-aaaaa-aaabq-cai").unwrap();
    assert!(create_call(&value, None).is_err());
    let call = create_call(&value, Some(subnet)).unwrap();
    assert_eq!(call.args, wire_fixture("store-create"));
    value.network = crate::Network::Local {
        url: "http://127.0.0.1:8000/".into(),
        root_key: "00".repeat(133),
        root_subnet: None,
        routing_canister: "aaaaa-aa".into(),
    };
    assert_eq!(
        create_call(&value, Some(subnet))
            .unwrap()
            .effective
            .to_text(),
        root.canister
    );
}

#[test]
fn only_a_retained_creation_principal_can_advance_installation() {
    let plan = plan();
    let child = Principal::from_text("rrkah-fqaaa-aaaaa-aaaaq-cai").unwrap();
    for name in ["created", "created-duplicate"] {
        let bytes = wire_fixture(name);
        assert_eq!(created(&plan, &bytes).unwrap().as_slice(), child.as_slice());
        let mut trailing = bytes.clone();
        trailing.push(0);
        assert!(matches!(created(&plan, &trailing), Err(Error::Decode(_))));
        assert!(matches!(
            created(&plan, &bytes[..bytes.len() - 1]),
            Err(Error::Decode(_))
        ));
    }
    for name in ["created-no-id", "created-old", "created-no-funds"] {
        let bytes = wire_fixture(name);
        assert!(matches!(created(&plan, &bytes), Err(Error::Blocked(_))));
    }
}

#[test]
fn direct_plan_encodings_keep_their_original_digests() {
    let mut plan = plan();
    assert!(
        !serde_json::to_value(&plan)
            .unwrap()
            .as_object()
            .unwrap()
            .contains_key("management")
    );
    assert_eq!(
        plan.digest().unwrap(),
        "9764c46f9d8e48c6b56215dddf2d53a71c8e656d4d8027b295917affe7eff5ba"
    );
    plan.version = 2;
    plan.deployment = Some(crate::DeploymentName {
        fleet: "mainnet".into(),
        app: "hello".into(),
        canister: "backend".into(),
    });
    assert_eq!(
        plan.digest().unwrap(),
        "6ef3bfb1ec1a4a16885a158003d0c5331607c8e788f029ae60210d1c32b7a8cb"
    );
}
