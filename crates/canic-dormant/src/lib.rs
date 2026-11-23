//! A parked canister retains only its installation receipt while Root holds custody.

use canic::topology::dormancy::{DormancyReceipt, Receipt};
use ic_platform_runtime::system;
use ic_storage::DefaultMemoryImpl;
use std::cell::OnceCell;

// Record the retirement binding on install and reopen the same cell on upgrade.
canic::mount_lifecycle!(
    input = Receipt,
    max_bytes = 512,
    decode = Receipt::decode,
    install = install,
    reopen = reopen,
);

// dormancy_receipt
// Return the retired allocation and its original proof deadline to any caller.
// limits : default
#[canic::update(contract = DormancyReceipt)]
fn dormancy_receipt() -> Receipt {
    RECEIPT.with(|state| state.get().expect("dormant receipt open").clone())
}

const KEY: &str = "dormant.receipt.v1";

thread_local! {
    static RECEIPT: OnceCell<Receipt> = const { OnceCell::new() };
}

fn install(receipt: Receipt) {
    canic::guard::require_sender(receipt.root).expect("installing Root required");
    assert!(
        system::is_controller(&receipt.root),
        "Root must control child"
    );
    open(Some(receipt));
}

fn reopen() {
    open(None);
}

fn open(initial: Option<Receipt>) {
    let runtime = canic::storage::bootstrap(
        DefaultMemoryImpl::default(),
        initial.is_some(),
        "dormant",
        &[KEY],
        8,
    );
    let cell = canic::storage::cell(
        &runtime,
        KEY,
        Receipt::LIMITS.bytes,
        initial.map(|receipt| receipt.encode().expect("encode retirement receipt")),
    );
    let receipt = Receipt::decode(cell.get()).expect("open retirement receipt");
    assert_eq!(
        receipt.subject,
        system::canister_self(),
        "wrong retired canister"
    );
    RECEIPT.with(|state| state.set(receipt).expect("receipt already open"));
}
