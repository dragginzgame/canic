//! Direct deposits retain callback outcome and refund before later work.

use super::{controller, dispatch, private_continuation};
use crate::{
    FUND, FUNDING_LATER, FUNDING_STATUS, Fault, FundingOutcome, FundingRequest, FundingStatus,
    LIMITS, MAX_FUNDING_CYCLES, MAX_MANAGEMENT_REQUEST_BYTES, Phase,
};
use ic_platform_protocol::{
    candid::Limits,
    management::{self as mgmt, CanisterArgs},
};
use ic_platform_runtime::{
    self as runtime,
    call::{Call, Outcome, Wait},
    system as api,
};
use std::cell::RefCell;

thread_local! {
    static JOURNAL: RefCell<FundingStatus> = RefCell::new(FundingStatus::default());
}

fn decode_fund(bytes: &[u8]) -> Result<FundingRequest, ic_wire::Error> {
    ic_wire::decode(
        bytes,
        ic_wire::Encoding::Frame {
            method: FUND,
            direction: ic_wire::Direction::Request,
        },
        LIMITS,
    )
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "Runtime transfers the owned endpoint result to its encoder"
)]
fn encode_fund(value: FundingStatus) -> Result<Vec<u8>, ic_wire::Error> {
    ic_wire::encode(
        &value,
        ic_wire::Encoding::Frame {
            method: FUND,
            direction: ic_wire::Direction::Response,
        },
        LIMITS,
    )
}

fn decode_status(bytes: &[u8]) -> Result<(), ic_wire::Error> {
    ic_wire::decode::<()>(
        bytes,
        ic_wire::Encoding::Frame {
            method: FUNDING_STATUS,
            direction: ic_wire::Direction::Request,
        },
        LIMITS,
    )
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "Runtime transfers the owned endpoint result to its encoder"
)]
fn encode_status(value: FundingStatus) -> Result<Vec<u8>, ic_wire::Error> {
    ic_wire::encode(
        &value,
        ic_wire::Encoding::Frame {
            method: FUNDING_STATUS,
            direction: ic_wire::Direction::Response,
        },
        LIMITS,
    )
}

fn decode_later(bytes: &[u8]) -> Result<(), ic_wire::Error> {
    ic_wire::decode::<()>(
        bytes,
        ic_wire::Encoding::Frame {
            method: FUNDING_LATER,
            direction: ic_wire::Direction::Request,
        },
        LIMITS,
    )
}

#[runtime::query(runtime = runtime, guard = controller, decode = decode_status, max_bytes = 2097152, on_error = endpoint_error, name = "funding_status", encode = encode_status)]
pub(super) fn funding_status((): ()) -> FundingStatus {
    JOURNAL.with_borrow(Clone::clone)
}

pub(super) fn restore(state: FundingStatus) {
    JOURNAL.with_borrow_mut(|journal| *journal = state);
}

#[runtime::update(runtime = runtime, guard = controller, decode = decode_fund, max_bytes = 2097152, on_error = endpoint_error, name = "fund", encode = encode_fund)]
async fn fund(intent: FundingRequest) -> FundingStatus {
    let previous = funding_status(());
    if previous.phase != Phase::Empty {
        if previous.intent.as_ref() != Some(&intent) {
            api::trap("different funding intent refused");
        }
        if previous.phase == Phase::Issued {
            api::trap("blocked: original funding outcome is not retained");
        }
        return previous;
    }
    assert!(intent.target.len() <= 29, "invalid funding target");
    assert!(
        intent.cycles > 0 && intent.cycles <= MAX_FUNDING_CYCLES,
        "funding amount outside fixture bound"
    );
    let request = CanisterArgs {
        canister_id: ic_platform_types::Principal::from_slice(&intent.target),
    }
    .encode(MAX_MANAGEMENT_REQUEST_BYTES)
    .expect("bounded deposit request");
    JOURNAL.with_borrow_mut(|state| {
        state.phase = Phase::Issued;
        state.intent = Some(intent.clone());
        state.attempts = 1;
        state.request.clone_from(&request);
        state.reserved_cycles = intent.cycles;
    });
    let result = dispatch("deposit_cycles", request, intent.cycles, intent.fault).await;
    // Local dispatch errors occur before a callback exists. Trap that execution
    // instead of reading callback-only refund state or fabricating a receipt.
    let completion = result.expect("local deposit dispatch");
    let refunded = completion
        .refunded_cycles()
        .expect("callback refund retained");
    let outcome = match completion.into_parts().0 {
        Outcome::Reply(reply) => {
            mgmt::decode_empty_response(
                &reply,
                Limits {
                    bytes: 4096,
                    header: 1024,
                    types: 32,
                    work: 8192,
                    allocation: 8192,
                    vector: 16_384,
                    skipping: 65_536,
                },
            )
            .expect("management deposit reply");
            assert_eq!(
                refunded, 0,
                "successful deposit must accept the full attachment"
            );
            FundingOutcome::Deposited
        }
        Outcome::Rejected(rejection) => FundingOutcome::Rejected {
            code: rejection.code(),
        },
        Outcome::OutcomeUnknown(_) => api::trap("unbounded deposit outcome unknown"),
    };
    assert!(
        refunded <= intent.cycles,
        "refund exceeds original attachment"
    );
    if intent.fault == Fault::BeforeResultWrite {
        api::trap("injected trap before result write");
    }
    JOURNAL.with_borrow_mut(|state| {
        // A rejection alone does not prove no effect. Keep any unrefunded
        // attachment unresolved; even a recorded rejection never permits retry.
        state.reserved_cycles = if outcome == FundingOutcome::Deposited {
            0
        } else {
            intent.cycles - refunded
        };
        state.outcome = Some(outcome);
        state.refunded_cycles = Some(refunded);
        state.phase = Phase::Recorded;
    });
    if intent.fault == Fault::AfterResultWrite {
        api::trap("injected trap after result write");
    }
    if intent.fault == Fault::LaterWork {
        let bytes = ic_wire::encode(
            &(),
            ic_wire::Encoding::Frame {
                method: FUNDING_LATER,
                direction: ic_wire::Direction::Request,
            },
            LIMITS,
        )
        .expect("funding continuation");
        let result = Call::new(api::canister_self(), "funding_later", Wait::Unbounded)
            .owned_args(bytes)
            .execute()
            .await;
        assert!(
            matches!(result.unwrap().into_parts().0, Outcome::Rejected(_)),
            "injected later work must reject"
        );
        api::trap("later work failed after result commit");
    }
    funding_status(())
}

#[runtime::update(runtime = runtime, guard = private_continuation, decode = decode_later, max_bytes = 2097152, on_error = endpoint_error, name = "funding_later", manual_reply)]
async fn funding_later((): ()) {
    JOURNAL.with_borrow_mut(|state| state.later_work_entered = true);
    api::trap("injected later-work trap");
}

fn endpoint_error(error: impl std::fmt::Debug) -> ! {
    ic_platform_runtime::system::trap(&format!("endpoint refused: {error:?}"))
}
