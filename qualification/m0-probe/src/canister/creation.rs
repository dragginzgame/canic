//! Installed experiment using Platform's one-shot call runtime.

use super::{controller, dispatch, private_continuation};
use crate::{
    CREATION_CYCLES, Fault, LATER_WORK, LIMITS, MAX_MANAGEMENT_REQUEST_BYTES, Phase, START, STATUS,
    Status,
};
use ic_platform::{
    candid_codec::{self, decoder},
    management_canister_types::{CanisterIdRecord, CanisterSettings, CreateCanisterArgs},
};
use ic_platform_runtime::{
    call::{Call, WaitPolicy},
    cdk::{self, api},
};
use std::cell::RefCell;

thread_local! {
    // Message commits retain heap state; maintenance checkpoints this slot.
    static JOURNAL: RefCell<Status> = RefCell::new(Status::default());
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "Platform's entrypoint hook owns the argument buffer"
)]
fn decode_start(bytes: Vec<u8>) -> Fault {
    ic_wire::native::decode_request(START, &bytes, LIMITS).expect("valid start frame")
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "Platform's entrypoint hook transfers the returned value"
)]
fn encode_start(value: Status) -> Vec<u8> {
    ic_wire::native::encode_response(START, &value, LIMITS).expect("bounded start reply")
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "Platform's entrypoint hook owns the argument buffer"
)]
fn decode_status(bytes: Vec<u8>) {
    ic_wire::native::decode_request::<()>(STATUS, &bytes, LIMITS).expect("valid status frame");
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "Platform's entrypoint hook transfers the returned value"
)]
fn encode_status(value: Status) -> Vec<u8> {
    ic_wire::native::encode_response(STATUS, &value, LIMITS).expect("bounded status reply")
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "Platform's entrypoint hook owns the argument buffer"
)]
fn decode_later(bytes: Vec<u8>) {
    ic_wire::native::decode_request::<()>(LATER_WORK, &bytes, LIMITS).expect("valid later frame");
}

#[cdk::query(
    guard = "controller",
    decode_with = "decode_status",
    encode_with = "encode_status"
)]
pub(super) fn status() -> Status {
    JOURNAL.with_borrow(Clone::clone)
}

pub(super) fn restore(state: Status) {
    JOURNAL.with_borrow_mut(|journal| *journal = state);
}

#[cdk::update(
    guard = "controller",
    decode_with = "decode_start",
    encode_with = "encode_start"
)]
async fn start(fault: Fault) -> Status {
    let previous = status();
    if previous.phase != Phase::Empty {
        assert_eq!(previous.fault, Some(fault), "different intent refused");
        if previous.phase == Phase::Issued {
            api::trap("blocked: original creation outcome is not retained");
        }
        return previous;
    }

    // Required external Candid is confined to the management adapter; owned
    // commands/status use Wire. Default creation controllers name this executor.
    let args = CreateCanisterArgs {
        settings: Some(CanisterSettings {
            controllers: Some(vec![api::canister_self()]),
            ..CanisterSettings::default()
        }),
        sender_canister_version: Some(api::canister_version()),
    };
    let request = candid_codec::encode_one(&args, MAX_MANAGEMENT_REQUEST_BYTES)
        .expect("bounded creation request");
    JOURNAL.with_borrow_mut(|state| {
        state.phase = Phase::Issued;
        state.fault = Some(fault);
        state.attempts = 1;
        state.request.clone_from(&request);
        state.reserved_cycles = CREATION_CYCLES;
    });
    let reply = dispatch("create_canister", request, CREATION_CYCLES, fault)
        .await
        .expect("management creation reply");
    let refunded = api::msg_cycles_refunded();
    let result: CanisterIdRecord = decoder::decode_one(
        &reply,
        decoder::Limits {
            bytes: 4096,
            header: 1024,
            types: 32,
            work: 8192,
            allocation: 8192,
        },
    )
    .expect("management creation principal");
    if fault == Fault::BeforeResultWrite {
        api::trap("injected trap before result write");
    }
    JOURNAL.with_borrow_mut(|state| {
        state.child = Some(result.canister_id.as_slice().to_vec());
        state.refunded_cycles = Some(refunded);
        state.reserved_cycles = 0;
        state.phase = Phase::Recorded;
    });
    if fault == Fault::AfterResultWrite {
        api::trap("injected trap after result write");
    }
    if fault == Fault::LaterWork {
        // This actual inter-canister call commits the result before the later
        // execution can trap. A Rust-only await would not establish this boundary.
        let bytes = ic_wire::native::encode_request(LATER_WORK, &(), LIMITS).expect("later frame");
        let result = Call::new(api::canister_self(), "later_work", WaitPolicy::Unbounded)
            .take_raw_args(bytes)
            .execute()
            .await;
        assert!(result.is_err(), "injected later work must reject");
        api::trap("later work failed after result commit");
    }
    status()
}

#[cdk::update(
    guard = "private_continuation",
    decode_with = "decode_later",
    manual_reply
)]
fn later_work() {
    JOURNAL.with_borrow_mut(|state| state.later_work_entered = true);
    api::trap("injected later-work trap");
}
