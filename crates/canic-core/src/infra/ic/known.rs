//! Module: infra::ic::known
//!
//! Responsibility: expose known IC system canister principals.
//! Does not own: network selection, config overrides, or ledger metadata.
//! Boundary: infra adapters use these as canonical mainnet defaults.

use crate::cdk::types::Principal;
use std::sync::LazyLock;

/// Mainnet NNS registry canister principal.
pub static NNS_REGISTRY_CANISTER: LazyLock<Principal> = LazyLock::new(|| {
    Principal::from_text("rwlgt-iiaaa-aaaaa-aaaaa-cai")
        .expect("NNS registry principal literal must be valid")
});

/// Mainnet ICP ledger canister principal.
pub static ICP_LEDGER_CANISTER: LazyLock<Principal> = LazyLock::new(|| {
    Principal::from_text("ryjl3-tyaaa-aaaaa-aaaba-cai")
        .expect("ICP ledger principal literal must be valid")
});

/// Mainnet cycles minting canister principal.
pub static CYCLES_MINTING_CANISTER: LazyLock<Principal> = LazyLock::new(|| {
    Principal::from_text("rkp4c-7iaaa-aaaaa-aaaca-cai")
        .expect("cycles minting canister principal literal must be valid")
});
