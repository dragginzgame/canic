use super::{DECODE, LEDGER_FEE, create_call, created};
use crate::{Artifact, Error, Network, Plan};
use ic_platform::{
    candid_codec::{self, decoder},
    cycles_ledger_types as ledger,
    principal::Principal,
};

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
    status.detail = StatusDetail::StoredChild { delivery: None };
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
    let args: ledger::CreateCanisterArgs = decoder::decode_one(&call.args, DECODE).unwrap();
    assert_eq!(call.canister.to_text(), super::LEDGER);
    assert_eq!(call.method, "create_canister");
    assert_eq!(args.created_at_time, Some(plan.created_at_ns));
    assert_eq!(args.amount, plan.creation_cycles() - LEDGER_FEE);
    assert_eq!(args.from_subaccount, None);
    let settings = args.creation_args.unwrap().settings.unwrap();
    assert_eq!(
        settings.controllers,
        Some(vec![Principal::from_text(&plan.controller).unwrap()])
    );
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
    let request: ic_platform::cycles_ledger_types::CreateCanisterArgs =
        ic_platform::candid_codec::decoder::decode_one(&call.args, DECODE).unwrap();
    assert_eq!(
        request.creation_args.unwrap().subnet_selection,
        Some(ic_platform::cycles_ledger_types::SubnetSelection::Subnet { subnet })
    );
    value.network = crate::Network::Local {
        url: "http://127.0.0.1:8000/".into(),
        root_key: "00".repeat(133),
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
    for result in [
        Ok(ledger::CreateCanisterSuccess {
            block_id: 1_u64.into(),
            canister_id: child,
        }),
        Err(ledger::CreateCanisterError::Duplicate {
            duplicate_of: 1_u64.into(),
            canister_id: Some(child),
        }),
    ] {
        let bytes = candid_codec::encode_one(&result, 4096).unwrap();
        assert_eq!(created(&plan, &bytes).unwrap(), child);
    }
    for error in [
        ledger::CreateCanisterError::Duplicate {
            duplicate_of: 1_u64.into(),
            canister_id: None,
        },
        ledger::CreateCanisterError::TooOld,
        ledger::CreateCanisterError::InsufficientFunds {
            balance: 0_u64.into(),
        },
    ] {
        let result: ledger::CreateCanisterResult = Err(error);
        let bytes = candid_codec::encode_one(&result, 4096).unwrap();
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
