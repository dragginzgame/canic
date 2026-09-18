//! Module: pic::fleet_registry::baseline::tests::operator_shortfall
//!
//! Responsibility: qualify fresh reinstall admission against a disposable Ledger account.
//! Boundary: fixture transfers bracket admission; they do not qualify retained mint recovery.

use super::*;
use canic_host::fleet_ensure::ops::EnsurePaths;
use std::fs;

pub(super) fn assert_fresh_reinstall_rejection(
    input: &ReinstallJourney<'_>,
    plan: &FleetEnsurePlan,
) {
    let desired = input.desired;
    let operator = Principal::from_text(&desired.operator).unwrap();
    let ledger = Principal::from_text(&desired.cycles_ledger).unwrap();
    let reserve = Principal::self_authenticating(b"disposable operator shortfall reserve");
    let balance = ledger_account_balance(input.pic, ledger, operator);
    let paths = EnsurePaths::under(input.adapter_root, &desired.environment, &desired.fleet);
    let documents = [&paths.plan, &paths.journal, &paths.state];
    let before = documents.map(|path| fs::read(path).unwrap());
    let withdrawals: u64 = input
        .pic
        .query_candid(ledger, "withdrawal_count", ())
        .unwrap();
    assert!(plan.conservation.maximum_operator_debit_cycles > 0);

    move_fixture_balance(input.pic, ledger, operator, reserve, balance.clone());
    let result = fleet_ensure_workflow::apply(
        input.adapter_root,
        desired,
        &desired_sha256(desired),
        &desired.fleet,
        &plan.plan_sha256,
        &mut literal_zero_journey_platform(
            desired,
            input.icp_wrapper,
            input.adapter_root,
            input.local_replica.clone(),
            true,
        ),
    );
    assert!(
        matches!(result,
            Err(EnsureWorkflowError::InsufficientOperatorCycles { actual: 0, required })
                if required == plan.conservation.maximum_operator_debit_cycles
        ),
        "reject before replacing the completed preparation journal: {result:?}"
    );
    assert_eq!(documents.map(|path| fs::read(path).unwrap()), before);
    assert_eq!(
        input
            .pic
            .query_candid::<u64, _>(ledger, "withdrawal_count", ())
            .unwrap(),
        withdrawals
    );
    move_fixture_balance(input.pic, ledger, reserve, operator, balance.clone());
    assert_eq!(ledger_account_balance(input.pic, ledger, operator), balance);
}

fn move_fixture_balance(
    pic: &PocketIc,
    ledger: Principal,
    from: Principal,
    to: Principal,
    amount: Nat,
) {
    // The fixture-only override makes the out-and-back setup conserve the exact
    // original balance. Restore the reviewed fee before production admission.
    let _: () = pic
        .update_candid(
            ledger,
            "set_transfer_fee_override",
            (Some(Nat::from(0_u8)),),
        )
        .unwrap();
    let receipt: Result<Nat, QualificationIcrc1TransferError> = pic
        .update_candid_as(
            ledger,
            from,
            "icrc1_transfer",
            (QualificationIcrc1TransferArg {
                from_subaccount: None,
                to: QualificationIcrc1Account {
                    owner: to,
                    subaccount: None,
                },
                fee: Some(Nat::from(0_u8)),
                created_at_time: Some(1_800_000_000_000_000_099),
                memo: Some(b"fresh funding admission fixture".to_vec()),
                amount,
            },),
        )
        .unwrap();
    receipt.unwrap();
    let _: () = pic
        .update_candid(ledger, "set_transfer_fee_override", (None::<Nat>,))
        .unwrap();
}
