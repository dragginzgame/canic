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
    self as runtime,
    call::{Call, Wait},
    system as api,
};
use std::cell::RefCell;

thread_local! {
    // Message commits retain heap state; maintenance checkpoints this slot.
    static JOURNAL: RefCell<Status> = RefCell::new(Status::default());
}

fn decode_start(bytes: &[u8]) -> Result<Fault, ic_wire::Error> {
    ic_wire::decode(
        bytes,
        ic_wire::Encoding::Frame {
            method: START,
            direction: ic_wire::Direction::Request,
        },
        LIMITS,
    )
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "Runtime transfers the owned endpoint result to its encoder"
)]
fn encode_start(value: Status) -> Result<Vec<u8>, ic_wire::Error> {
    ic_wire::encode(
        &value,
        ic_wire::Encoding::Frame {
            method: START,
            direction: ic_wire::Direction::Response,
        },
        LIMITS,
    )
}

fn decode_status(bytes: &[u8]) -> Result<(), ic_wire::Error> {
    ic_wire::decode::<()>(
        bytes,
        ic_wire::Encoding::Frame {
            method: STATUS,
            direction: ic_wire::Direction::Request,
        },
        LIMITS,
    )
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "Runtime transfers the owned endpoint result to its encoder"
)]
fn encode_status(value: Status) -> Result<Vec<u8>, ic_wire::Error> {
    ic_wire::encode(
        &value,
        ic_wire::Encoding::Frame {
            method: STATUS,
            direction: ic_wire::Direction::Response,
        },
        LIMITS,
    )
}

fn decode_later(bytes: &[u8]) -> Result<(), ic_wire::Error> {
    ic_wire::decode::<()>(
        bytes,
        ic_wire::Encoding::Frame {
            method: LATER_WORK,
            direction: ic_wire::Direction::Request,
        },
        LIMITS,
    )
}

#[runtime::query(runtime = runtime, guard = controller, decode = decode_status, max_bytes = 2097152, on_error = endpoint_error, name = "status", encode = encode_status)]
pub(super) fn status((): ()) -> Status {
    JOURNAL.with_borrow(Clone::clone)
}

pub(super) fn restore(state: Status) {
    JOURNAL.with_borrow_mut(|journal| *journal = state);
}

#[runtime::update(runtime = runtime, guard = controller, decode = decode_start, max_bytes = 2097152, on_error = endpoint_error, name = "start", encode = encode_start)]
async fn start(fault: Fault) -> Status {
    let previous = status(());
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
            controllers: Some(vec![ic_platform::principal::Principal::from_slice(
                api::canister_self().as_slice(),
            )]),
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
    let refunded = reply.refunded_cycles().expect("callback refund retained");
    let ic_platform_runtime::call::Outcome::Reply(reply) = reply.into_parts().0 else {
        api::trap("management creation refused");
    };
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
        let bytes = ic_wire::encode(
            &(),
            ic_wire::Encoding::Frame {
                method: LATER_WORK,
                direction: ic_wire::Direction::Request,
            },
            LIMITS,
        )
        .expect("later frame");
        let result = Call::new(api::canister_self(), "later_work", Wait::Unbounded)
            .owned_args(bytes)
            .execute()
            .await;
        assert!(
            matches!(
                result.unwrap().into_parts().0,
                ic_platform_runtime::call::Outcome::Rejected(_)
            ),
            "injected later work must reject"
        );
        api::trap("later work failed after result commit");
    }
    status(())
}

#[runtime::update(runtime = runtime, guard = private_continuation, decode = decode_later, max_bytes = 2097152, on_error = endpoint_error, name = "later_work", manual_reply)]
async fn later_work((): ()) {
    JOURNAL.with_borrow_mut(|state| state.later_work_entered = true);
    api::trap("injected later-work trap");
}

fn endpoint_error(error: impl std::fmt::Debug) -> ! {
    ic_platform_runtime::system::trap(&format!("endpoint refused: {error:?}"))
}
