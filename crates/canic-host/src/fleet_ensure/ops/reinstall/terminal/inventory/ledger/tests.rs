//! Original-account reconciliation and authenticated production-Ledger query qualification.

use super::*;
use crate::fleet_ensure::{
    CompletedCanisterInventoryView, CompletedReceiptAuditView, model::FleetTerminalSourceRecord,
};
use canic_core::ids::{FleetBinding, FleetKey, SubnetId};

fn id(byte: u8) -> Principal {
    Principal::from_slice(&[byte])
}

fn source() -> CompletedEstateInventoryView {
    let canisters = [
        ("coordinator", id(2), DesiredCanisterKind::Coordinator),
        ("root", id(3), DesiredCanisterKind::Root),
        ("idle", id(4), DesiredCanisterKind::Pool),
    ]
    .into_iter()
    .map(|(name, principal, kind)| {
        (
            name.into(),
            CompletedCanisterInventoryView {
                principal,
                subnet: SubnetId::from_principal(id(5)),
                kind,
                parent: None,
                root: (kind != DesiredCanisterKind::Coordinator).then(|| "root".into()),
                module_sha256: None,
                protocol_binding: None,
                originally_declared_controllers: vec![id(1)],
                recorded_cycles: 0,
            },
        )
    })
    .collect();
    CompletedEstateInventoryView {
        receipts: CompletedReceiptAuditView {
            documents: FleetTerminalSourceRecord {
                operation_id: "11".repeat(32),
                plan_sha256: "22".repeat(32),
                plan_document_sha256: "33".repeat(32),
                journal_document_sha256: "44".repeat(32),
                state_document_sha256: "55".repeat(32),
                phase_document_sha256: BTreeMap::new(),
            },
            effect_count: 1,
            phase_count: 0,
            source_operator: id(1).to_text(),
            cycles_ledger: id(9).to_text(),
            initial_controlled_cycles: 1_000,
            initial_operator_cycles: 1_000,
            initial_estate_funding_cycles_by_root: BTreeMap::from([("root".into(), 100)]),
            recorded_funding_cycles: 90,
            recorded_operator_debit_cycles: 100,
            original_maximum_execution_burn_cycles: 50,
        },
        coordinator_registry: crate::fleet_ensure::CompletedCoordinatorMembershipView {
            fleet: FleetBinding {
                app: "ledger_test".into(),
                fleet: FleetKey {
                    canonical_network_id: CanonicalNetworkId::ic_mainnet(),
                    fleet_id: "66".repeat(32).parse().unwrap(),
                },
            },
            coordinator: id(2),
            coordinator_subnet: SubnetId::from_principal(id(5)),
            epoch: 1,
            revision: 3,
            roots: Vec::new(),
        },
        fleet: FleetBinding {
            app: "ledger_test".into(),
            fleet: FleetKey {
                canonical_network_id: CanonicalNetworkId::ic_mainnet(),
                fleet_id: "66".repeat(32).parse().unwrap(),
            },
        },
        release_build_id: "77".repeat(32).parse().unwrap(),
        coordinator: id(2),
        canisters,
        recorded_controlled_canister_cycles: 0,
    }
}

fn balances() -> BTreeMap<Principal, u128> {
    BTreeMap::from([(id(1), 900), (id(2), 17), (id(3), 100), (id(4), 19)])
}

#[test]
fn preserves_nonzero_other_accounts_without_rebasing_original_operator_or_root() {
    let source = source();
    let view = reconcile(accounts(&source).unwrap(), balances()).unwrap();
    assert_eq!(view.operator().cycles, 900);
    assert_eq!(view.root_accounts()["root"].cycles, 100);
    assert_eq!(view.other_canister_accounts()["coordinator"].cycles, 17);
    assert_eq!(view.other_canister_accounts()["idle"].cycles, 19);
    assert_eq!(view.total_canister_ledger_cycles(), 136);
    assert_eq!(source.receipts.initial_operator_cycles, 1_000);
    assert_eq!(
        source.receipts.initial_estate_funding_cycles_by_root["root"],
        100
    );
}

#[test]
fn both_unexplained_debits_and_credits_reject_even_if_other_accounts_offset_them() {
    for amount in [899, 901] {
        let mut balances = balances();
        balances.insert(id(1), amount);
        balances.insert(id(2), 1_000_000);
        assert!(
            matches!(reconcile(accounts(&source()).unwrap(), balances), Err(CompletedLedgerError::OperatorMovement { expected:900, observed }) if observed == amount)
        );
    }
    for amount in [99, 101] {
        let mut balances = balances();
        balances.insert(id(3), amount);
        balances.insert(id(4), 1_000_000);
        assert!(
            matches!(reconcile(accounts(&source()).unwrap(), balances), Err(CompletedLedgerError::RootMovement { expected:100, observed, .. }) if observed == amount)
        );
    }
}

#[test]
fn missing_extra_and_overflowing_accounts_are_not_zeroed_or_truncated() {
    let mut missing = balances();
    missing.remove(&id(4));
    let mut extra = balances();
    extra.insert(id(7), 0);
    for invalid in [missing, extra] {
        assert!(matches!(
            reconcile(accounts(&source()).unwrap(), invalid),
            Err(CompletedLedgerError::AccountSet)
        ));
    }
    let mut overflow = balances();
    overflow.insert(id(4), u128::MAX);
    assert!(matches!(
        reconcile(accounts(&source()).unwrap(), overflow),
        Err(CompletedLedgerError::Overflow)
    ));
    let mut source = source();
    source.receipts.initial_operator_cycles = 99;
    assert!(matches!(
        accounts(&source),
        Err(CompletedLedgerError::Overflow)
    ));
}

#[test]
fn duplicate_owners_and_incomplete_original_root_accounts_reject() {
    for mutation in [
        "operator",
        "ledger",
        "duplicate",
        "missing-root",
        "extra-root",
        "invalid-ledger",
    ] {
        let mut source = source();
        match mutation {
            "operator" => source.canisters.get_mut("idle").unwrap().principal = id(1),
            "ledger" => source.canisters.get_mut("idle").unwrap().principal = id(9),
            "duplicate" => source.canisters.get_mut("idle").unwrap().principal = id(2),
            "missing-root" => source
                .receipts
                .initial_estate_funding_cycles_by_root
                .clear(),
            "extra-root" => {
                source
                    .receipts
                    .initial_estate_funding_cycles_by_root
                    .insert("other".into(), 0);
            }
            "invalid-ledger" => source.receipts.cycles_ledger = Principal::anonymous().to_text(),
            _ => unreachable!(),
        }
        assert!(
            matches!(accounts(&source), Err(CompletedLedgerError::AccountSet)),
            "{mutation}"
        );
    }
}

#[test]
fn ledger_reply_requires_bounded_nat_and_exact_u128_arithmetic() {
    assert_eq!(
        decode(id(1), &candid::encode_one(Nat::from(u128::MAX)).unwrap()).unwrap(),
        u128::MAX
    );
    let overflow = Nat::from(u128::MAX) + Nat::from(1_u8);
    assert!(matches!(
        decode(id(1), &candid::encode_one(overflow).unwrap()),
        Err(CompletedLedgerError::Overflow)
    ));
    for bytes in [
        vec![0; REPLY_BYTES + 1],
        candid::encode_one("123").unwrap(),
        Vec::new(),
    ] {
        assert!(matches!(
            decode(id(1), &bytes),
            Err(CompletedLedgerError::Response { .. })
        ));
    }
}

#[test]
#[ignore = "governed PocketIC proof reads the production Cycles Ledger over authenticated HTTP"]
#[expect(
    clippy::too_many_lines,
    reason = "one production-Ledger fixture qualifies exact balances, authority mismatch and query-only replay"
)]
fn governed_pocketic_completed_estate_ledger_accounts() {
    use ic_agent::{Identity, identity::BasicIdentity};
    use ic_testkit::{
        pic::{CandidCallExt, PocketIcBuilder},
        pocket_ic::common::rest::{IcpFeatures, IcpFeaturesConfig},
    };
    use icrc_ledger_types::icrc1::transfer::{TransferArg, TransferError};

    let mut pic = crate::test_support::start_pocket_ic(
        PocketIcBuilder::new()
            .with_nns_subnet()
            .with_application_subnet()
            .with_icp_features(IcpFeatures {
                cycles_token: Some(IcpFeaturesConfig::DefaultConfig),
                ..IcpFeatures::default()
            }),
    );
    pic.set_time(std::time::SystemTime::now().into());
    let identity = BasicIdentity::from_raw_key(&[43; 32]);
    let operator = identity.sender().unwrap();
    let ledger = Principal::from_text("um5iw-rqaaa-aaaaq-qaaba-cai").unwrap();
    let mut source = source();
    source.receipts.source_operator = operator.to_text();
    source.receipts.cycles_ledger = ledger.to_text();
    let subnet = pic.topology().get_app_subnets()[0];
    for entry in source.canisters.values_mut() {
        entry.principal = pic.create_canister_on_subnet(Some(operator), None, subnet);
    }
    source.coordinator = source.canisters["coordinator"].principal;
    let funded = [
        (operator, 900_u128),
        (source.canisters["root"].principal, 100),
        (source.coordinator, 17),
        (source.canisters["idle"].principal, 19),
    ];
    for (owner, amount) in funded {
        let result: Result<Nat, TransferError> = pic
            .update_candid(
                ledger,
                "icrc1_transfer",
                (TransferArg {
                    from_subaccount: None,
                    to: Account {
                        owner,
                        subaccount: None,
                    },
                    amount: Nat::from(amount),
                    fee: None,
                    memo: None,
                    created_at_time: None,
                },),
            )
            .unwrap();
        result.unwrap();
    }
    let controller = pic.get_controllers(ledger)[0];
    let before = pic
        .canister_status(ledger, Some(controller))
        .unwrap()
        .version;
    let key = pic.root_key().unwrap();
    source.fleet.fleet.canonical_network_id =
        CanonicalNetworkId::from_der_root_trust_anchor(&key).unwrap();
    let url = pic.make_live(None);
    let agent = Agent::builder()
        .with_url(url)
        .with_identity(identity)
        .with_max_response_body_size(8 * 1024 * 1024)
        .build()
        .unwrap();
    agent.set_root_key(key);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    for _ in 0..2 {
        let view = runtime
            .block_on(observe(&agent, &source, Instant::now()))
            .unwrap();
        assert_eq!(view.operator().cycles, 900);
        assert_eq!(view.total_canister_ledger_cycles(), 136);
        assert_eq!(view.other_canister_accounts()["coordinator"].cycles, 17);
    }
    assert_eq!(
        pic.canister_status(ledger, Some(controller))
            .unwrap()
            .version,
        before
    );
    source.receipts.initial_operator_cycles += 1;
    assert!(matches!(
        runtime.block_on(observe(&agent, &source, Instant::now())),
        Err(CompletedLedgerError::OperatorMovement { .. })
    ));
    source.receipts.initial_operator_cycles -= 1;
    source.receipts.source_operator = id(1).to_text();
    assert!(matches!(
        runtime.block_on(observe(&agent, &source, Instant::now())),
        Err(CompletedLedgerError::Authority)
    ));
    source.receipts.source_operator = operator.to_text();
    source.fleet.fleet.canonical_network_id = CanonicalNetworkId::ic_mainnet();
    assert!(matches!(
        runtime.block_on(observe(&agent, &source, Instant::now())),
        Err(CompletedLedgerError::Authority)
    ));
    source.fleet.fleet.canonical_network_id =
        CanonicalNetworkId::from_der_root_trust_anchor(&agent.read_root_key()).unwrap();
    let expired = Instant::now().checked_sub(Duration::from_secs(61)).unwrap();
    assert!(matches!(
        runtime.block_on(observe(&agent, &source, expired)),
        Err(CompletedLedgerError::Expired)
    ));
}
