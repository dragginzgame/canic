//! Module: pic::fleet_registry::baseline::tests::operator_shortfall
//!
//! Responsibility: reject changed operator balance before current reset issues effects.
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
    let snapshot = || {
        documents.map(|path| match fs::read(path) {
            Ok(bytes) => Some(bytes),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => panic!("read execution evidence: {error}"),
        })
    };
    let before = snapshot();
    let withdrawals: u64 = (|| -> Result<_, Box<dyn std::error::Error + Send + Sync>> {
        let request_bytes = ic_platform::candid::contract::encode_args(())?;
        let reply = ic_testkit::pic::try_pocket_ic_operation(|| {
            (input.pic).query_call(
                ledger,
                ic_platform::principal::ExternalPrincipal::anonymous(),
                "withdrawal_count",
                request_bytes,
            )
        })
        .map_err(ic_testkit::pic::PocketIcCallFailure::Operation)?
        .map_err(ic_testkit::pic::PocketIcCallFailure::Rejected)?;
        ic_platform::candid::decode_args(&reply, canic::testing::CANDID_REPLY_LIMITS)
            .map_err(Into::into)
    })()
    .map(|(reply,)| reply)
    .unwrap();
    assert!(plan.conservation.maximum_operator_debit_cycles > 0);

    move_fixture_balance(input.pic, ledger, operator, reserve, balance.clone());
    let icp = canic_host::icp::IcpCli::new(
        input.icp_wrapper.to_str().unwrap(),
        Some(desired.environment.clone()),
    )
    .with_cwd(input.adapter_root)
    .with_local_replica(Some(input.local_replica.clone()));
    let result = canic_host::fleet_ensure::workflow::clean_reinstall::apply(
        input.adapter_root,
        &desired.environment,
        &desired.fleet,
        &plan.plan_sha256,
        &mut literal_zero_journey_platform(
            desired,
            input.icp_wrapper,
            input.adapter_root,
            input.local_replica.clone(),
            true,
        ),
        &icp,
    );
    assert!(
        matches!(result,
            Err(EnsureWorkflowError::InfrastructureBootstrap(
                canic_host::fleet_ensure::ops::infrastructure_bootstrap::InfrastructureBootstrapError::Integrity
            ))
        ),
        "reject before creating execution intent or issuing effects: {result:?}"
    );
    assert_eq!(snapshot(), before);
    assert_eq!(
        (|| -> Result<(u64,), Box<dyn std::error::Error + Send + Sync>> {
            let request_bytes = ic_platform::candid::contract::encode_args(())?;
            let reply = ic_testkit::pic::try_pocket_ic_operation(|| {
                (input.pic).query_call(
                    ledger,
                    ic_platform::principal::ExternalPrincipal::anonymous(),
                    "withdrawal_count",
                    request_bytes,
                )
            })
            .map_err(ic_testkit::pic::PocketIcCallFailure::Operation)?
            .map_err(ic_testkit::pic::PocketIcCallFailure::Rejected)?;
            ic_platform::candid::decode_args(&reply, canic::testing::CANDID_REPLY_LIMITS)
                .map_err(Into::into)
        })()
        .map(|(reply,)| reply)
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
    let _: () = (|| -> Result<_, Box<dyn std::error::Error + Send + Sync>> {
        let request_bytes = ic_platform::candid::contract::encode_args((Some(Nat::from(0_u8)),))?;
        let reply = ic_testkit::pic::try_pocket_ic_operation(|| {
            (pic).update_call(
                ledger,
                ic_platform::principal::ExternalPrincipal::anonymous(),
                "set_transfer_fee_override",
                request_bytes,
            )
        })
        .map_err(ic_testkit::pic::PocketIcCallFailure::Operation)?
        .map_err(ic_testkit::pic::PocketIcCallFailure::Rejected)?;
        ic_platform::candid::decode_args(&reply, canic::testing::CANDID_REPLY_LIMITS)
            .map_err(Into::into)
    })()
    .map(|(reply,)| reply)
    .unwrap();
    let receipt: Result<Nat, QualificationIcrc1TransferError> =
        (|| -> Result<_, Box<dyn std::error::Error + Send + Sync>> {
            let request_bytes =
                ic_platform::candid::contract::encode_args((QualificationIcrc1TransferArg {
                    from_subaccount: None,
                    to: QualificationIcrc1Account {
                        owner: to,
                        subaccount: None,
                    },
                    fee: Some(Nat::from(0_u8)),
                    created_at_time: Some(1_800_000_000_000_000_099),
                    memo: Some(b"fresh funding admission fixture".to_vec()),
                    amount,
                },))?;
            let reply = ic_testkit::pic::try_pocket_ic_operation(|| {
                (pic).update_call(ledger, from, "icrc1_transfer", request_bytes)
            })
            .map_err(ic_testkit::pic::PocketIcCallFailure::Operation)?
            .map_err(ic_testkit::pic::PocketIcCallFailure::Rejected)?;
            ic_platform::candid::decode_args(&reply, canic::testing::CANDID_REPLY_LIMITS)
                .map_err(Into::into)
        })()
        .map(|(reply,)| reply)
        .unwrap();
    receipt.unwrap();
    let _: () = (|| -> Result<_, Box<dyn std::error::Error + Send + Sync>> {
        let request_bytes = ic_platform::candid::contract::encode_args((None::<Nat>,))?;
        let reply = ic_testkit::pic::try_pocket_ic_operation(|| {
            (pic).update_call(
                ledger,
                ic_platform::principal::ExternalPrincipal::anonymous(),
                "set_transfer_fee_override",
                request_bytes,
            )
        })
        .map_err(ic_testkit::pic::PocketIcCallFailure::Operation)?
        .map_err(ic_testkit::pic::PocketIcCallFailure::Rejected)?;
        ic_platform::candid::decode_args(&reply, canic::testing::CANDID_REPLY_LIMITS)
            .map_err(Into::into)
    })()
    .map(|(reply,)| reply)
    .unwrap();
}
