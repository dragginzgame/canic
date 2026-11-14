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
        management: None,
        network: Network::Mainnet,
        controller: input.controller,
        artifact: Artifact {
            sha256: "00".repeat(32),
            bytes: 8,
        },
        target: None,
        previous_module: None,
        creation_cycles: input.creation_cycles,
        created_at_ns: 10,
        expires_at_ns: 100,
    }
}

#[test]
fn mainnet_provider_request_binds_funding_identity_and_controller() {
    let plan = plan();
    let call = create_call(&plan).unwrap();
    let args: ledger::CreateCanisterArgs = decoder::decode_one(&call.args, DECODE).unwrap();
    assert_eq!(call.canister.to_text(), super::LEDGER);
    assert_eq!(call.method, "create_canister");
    assert_eq!(args.created_at_time, Some(plan.created_at_ns));
    assert_eq!(args.amount, plan.creation_cycles - LEDGER_FEE);
    assert_eq!(args.from_subaccount, None);
    let settings = args.creation_args.unwrap().settings.unwrap();
    assert_eq!(
        settings.controllers,
        Some(vec![Principal::from_text(&plan.controller).unwrap()])
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
