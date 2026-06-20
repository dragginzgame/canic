#![expect(clippy::unused_async)]

use canic::{
    Error,
    cdk::{
        api::{msg_caller, msg_cycles_accept, msg_cycles_available},
        candid::{Int, Nat},
        types::Principal,
    },
    dto::blob_storage::{
        BlobStorageCashierAccountBalanceGetError, BlobStorageCashierAccountBalanceGetOk,
        BlobStorageCashierAccountBalanceGetRequest, BlobStorageCashierAccountBalanceGetResult,
        BlobStorageCashierAccountCycleBalances, BlobStorageCashierAccountTopUpError,
        BlobStorageCashierAccountTopUpOk, BlobStorageCashierAccountTopUpRequest,
        BlobStorageCashierAccountTopUpResult, BlobStorageCashierDebtTarget,
    },
    prelude::*,
};
use std::{cell::RefCell, collections::BTreeMap};

thread_local! {
    static BALANCES: RefCell<BTreeMap<Principal, u128>> = const { RefCell::new(BTreeMap::new()) };
    static GATEWAYS: RefCell<Vec<Principal>> = const { RefCell::new(Vec::new()) };
    static LAST_TOP_UP: RefCell<Option<MockTopUpRecord>> = const { RefCell::new(None) };
}

type MockTopUpRecordView = Option<(Option<Principal>, Option<Nat>, Nat)>;

#[derive(Clone)]
struct MockTopUpRecord {
    account: Option<Principal>,
    target_balance: Option<Nat>,
    attached_cycles: Nat,
}

canic::start!();

/// Run no-op setup for the blob-storage Cashier mock.
async fn canic_setup() {}

/// Accept no install payload for the blob-storage Cashier mock.
async fn canic_install(_: Option<Vec<u8>>) {}

/// Run no-op upgrade handling for the blob-storage Cashier mock.
async fn canic_upgrade() {}

#[canic_update(requires(caller::is_controller()))]
async fn blob_storage_cashier_mock_set_balance(
    account: Principal,
    balance: u128,
) -> Result<(), Error> {
    BALANCES.with_borrow_mut(|balances| {
        balances.insert(account, balance);
    });
    Ok(())
}

#[canic_update(requires(caller::is_controller()))]
async fn blob_storage_cashier_mock_set_gateways(gateways: Vec<Principal>) -> Result<(), Error> {
    GATEWAYS.with_borrow_mut(|stored| {
        *stored = gateways;
    });
    Ok(())
}

#[canic_query]
fn blob_storage_cashier_mock_last_top_up() -> Result<MockTopUpRecordView, Error> {
    Ok(LAST_TOP_UP.with_borrow(|record| {
        record.as_ref().map(|record| {
            (
                record.account,
                record.target_balance.clone(),
                record.attached_cycles.clone(),
            )
        })
    }))
}

#[ic_cdk::update(name = "account_balance_get_v1")]
async fn account_balance_get_v1(
    request: BlobStorageCashierAccountBalanceGetRequest,
) -> BlobStorageCashierAccountBalanceGetResult {
    BALANCES.with_borrow(|balances| {
        balances.get(&request.account).copied().map_or(
            BlobStorageCashierAccountBalanceGetResult::Err(
                BlobStorageCashierAccountBalanceGetError::AccountNotFound,
            ),
            |balance| {
                BlobStorageCashierAccountBalanceGetResult::Ok(
                    BlobStorageCashierAccountBalanceGetOk {
                        account_cycle_balances: cycle_balances(balance),
                        account: request.account,
                    },
                )
            },
        )
    })
}

#[ic_cdk::update(name = "account_top_up_v1")]
async fn account_top_up_v1(
    request: Option<BlobStorageCashierAccountTopUpRequest>,
) -> BlobStorageCashierAccountTopUpResult {
    let attached_cycles = msg_cycles_accept(msg_cycles_available());
    if attached_cycles == 0 {
        return BlobStorageCashierAccountTopUpResult::Err(
            BlobStorageCashierAccountTopUpError::TopUpWithoutCycles,
        );
    }

    let account = request
        .as_ref()
        .and_then(|request| request.account)
        .unwrap_or_else(msg_caller);
    let target_balance = request
        .as_ref()
        .and_then(|request| request.target_balance.clone());

    let Some(balance) = BALANCES.with_borrow_mut(|balances| {
        let current = balances.get(&account).copied().unwrap_or(0);
        let next = current.checked_add(attached_cycles)?;
        balances.insert(account, next);
        Some(next)
    }) else {
        return BlobStorageCashierAccountTopUpResult::Err(
            BlobStorageCashierAccountTopUpError::AccountBalanceOverflow,
        );
    };

    LAST_TOP_UP.with_borrow_mut(|record| {
        *record = Some(MockTopUpRecord {
            account: Some(account),
            target_balance,
            attached_cycles: nat_from_u128(attached_cycles),
        });
    });

    BlobStorageCashierAccountTopUpResult::Ok(BlobStorageCashierAccountTopUpOk {
        balance: cycle_balances(balance),
        message: "mock top-up accepted".to_string(),
    })
}

#[ic_cdk::update(name = "storage_gateway_principal_list_v1")]
async fn storage_gateway_principal_list_v1() -> Vec<Principal> {
    GATEWAYS.with_borrow(Clone::clone)
}

#[ic_cdk::update]
async fn blob_storage_cashier_mock_delay_tick() {}

fn cycle_balances(total: u128) -> BlobStorageCashierAccountCycleBalances {
    BlobStorageCashierAccountCycleBalances {
        total: int_from_u128(total),
        cycles_prepaid: int_from_u128(total),
        cycles_promo: int_from_u128(0),
        debt_target: BlobStorageCashierDebtTarget::Prepaid,
        cycles_ledger: int_from_u128(0),
    }
}

fn int_from_u128(value: u128) -> Int {
    Int::parse(value.to_string().as_bytes()).expect("u128 must encode as Candid int")
}

fn nat_from_u128(value: u128) -> Nat {
    Nat::parse(value.to_string().as_bytes()).expect("u128 must encode as Candid nat")
}

canic::finish!();
