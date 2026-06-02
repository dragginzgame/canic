//! Known IC system canister principals used by Canic infra adapters.

use crate::cdk::types::Principal;
use std::sync::LazyLock;

///
/// NNS_REGISTRY_CANISTER
///

pub static NNS_REGISTRY_CANISTER: LazyLock<Principal> = LazyLock::new(|| {
    Principal::from_text("rwlgt-iiaaa-aaaaa-aaaaa-cai")
        .expect("NNS registry principal literal must be valid")
});

///
/// ICP_LEDGER_CANISTER
///

#[allow(dead_code)]
pub static ICP_LEDGER_CANISTER: LazyLock<Principal> = LazyLock::new(|| {
    Principal::from_text("ryjl3-tyaaa-aaaaa-aaaba-cai")
        .expect("ICP ledger principal literal must be valid")
});

///
/// CYCLES_MINTING_CANISTER
///

#[allow(dead_code)]
pub static CYCLES_MINTING_CANISTER: LazyLock<Principal> = LazyLock::new(|| {
    Principal::from_text("rkp4c-7iaaa-aaaaa-aaaca-cai")
        .expect("cycles minting canister principal literal must be valid")
});

///
/// EXCHANGE_RATE_CANISTER
///

pub static EXCHANGE_RATE_CANISTER: LazyLock<Principal> = LazyLock::new(|| {
    Principal::from_text("uf6dk-hyaaa-aaaaq-qaaaq-cai")
        .expect("exchange rate canister principal literal must be valid")
});

///
/// CKUSDC_LEDGER_CANISTER
///

pub static CKUSDC_LEDGER_CANISTER: LazyLock<Principal> = LazyLock::new(|| {
    Principal::from_text("cngnf-vqaaa-aaaar-qag4q-cai")
        .expect("ckUSDC ledger principal literal must be valid")
});

///
/// CKUSDT_LEDGER_CANISTER
///

pub static CKUSDT_LEDGER_CANISTER: LazyLock<Principal> = LazyLock::new(|| {
    Principal::from_text("xevnm-gaaaa-aaaar-qafnq-cai")
        .expect("ckUSDT ledger principal literal must be valid")
});
