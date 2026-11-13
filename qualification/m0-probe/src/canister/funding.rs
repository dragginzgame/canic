//! Direct deposits retain callback outcome and refund before later work.

use super::{controller, dispatch, private_continuation};
use crate::{
    FUND, FUNDING_LATER, FUNDING_STATUS, Fault, FundingOutcome, FundingRequest, FundingStatus,
    LIMITS, MAX_FUNDING_CYCLES, MAX_MANAGEMENT_REQUEST_BYTES, Phase,
};
use ic_platform::{
    candid_codec::{self, decoder},
    management_canister_types::CanisterIdRecord,
};
use ic_platform_runtime::{
    Principal,
    call::{Call, CallFailed, WaitPolicy},
    cdk::{self, api},
};
use std::cell::RefCell;

thread_local! {
    static JOURNAL: RefCell<FundingStatus> = RefCell::new(FundingStatus::default());
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "Platform's entrypoint hook owns the argument buffer"
)]
fn decode_fund(bytes: Vec<u8>) -> FundingRequest {
    ic_wire::native::decode_request(FUND, &bytes, LIMITS).expect("valid funding frame")
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "Platform's entrypoint hook transfers the returned value"
)]
fn encode_fund(value: FundingStatus) -> Vec<u8> {
    ic_wire::native::encode_response(FUND, &value, LIMITS).expect("bounded funding reply")
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "Platform's entrypoint hook owns the argument buffer"
)]
fn decode_status(bytes: Vec<u8>) {
    ic_wire::native::decode_request::<()>(FUNDING_STATUS, &bytes, LIMITS)
        .expect("valid funding status frame");
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "Platform's entrypoint hook transfers the returned value"
)]
fn encode_status(value: FundingStatus) -> Vec<u8> {
    ic_wire::native::encode_response(FUNDING_STATUS, &value, LIMITS)
        .expect("bounded funding status reply")
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "Platform's entrypoint hook owns the argument buffer"
)]
fn decode_later(bytes: Vec<u8>) {
    ic_wire::native::decode_request::<()>(FUNDING_LATER, &bytes, LIMITS)
        .expect("valid funding continuation");
}

#[cdk::query(
    guard = "controller",
    decode_with = "decode_status",
    encode_with = "encode_status"
)]
pub(super) fn funding_status() -> FundingStatus {
    JOURNAL.with_borrow(Clone::clone)
}

pub(super) fn restore(state: FundingStatus) {
    JOURNAL.with_borrow_mut(|journal| *journal = state);
}

#[cdk::update(
    guard = "controller",
    decode_with = "decode_fund",
    encode_with = "encode_fund"
)]
async fn fund(intent: FundingRequest) -> FundingStatus {
    let previous = funding_status();
    if previous.phase != Phase::Empty {
        assert_eq!(
            previous.intent.as_ref(),
            Some(&intent),
            "different funding intent refused"
        );
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
    let request = candid_codec::encode_one(
        &CanisterIdRecord {
            canister_id: Principal::from_slice(&intent.target),
        },
        MAX_MANAGEMENT_REQUEST_BYTES,
    )
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
    let (outcome, refunded) = match result {
        Ok(reply) => {
            let refunded = api::msg_cycles_refunded();
            decoder::decode_args::<()>(
                &reply,
                decoder::Limits {
                    bytes: 4096,
                    header: 1024,
                    types: 32,
                    work: 8192,
                    allocation: 8192,
                },
                decoder::Arity::Exact,
            )
            .expect("management deposit reply");
            assert_eq!(
                refunded, 0,
                "successful deposit must accept the full attachment"
            );
            (FundingOutcome::Deposited, refunded)
        }
        Err(CallFailed::CallRejected(rejection)) => (
            FundingOutcome::Rejected {
                code: rejection.raw_reject_code(),
            },
            api::msg_cycles_refunded(),
        ),
        Err(error) => api::trap(format!("local deposit dispatch failed: {error}")),
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
        let bytes = ic_wire::native::encode_request(FUNDING_LATER, &(), LIMITS)
            .expect("funding continuation");
        let result = Call::new(api::canister_self(), "funding_later", WaitPolicy::Unbounded)
            .take_raw_args(bytes)
            .execute()
            .await;
        assert!(result.is_err(), "injected later work must reject");
        api::trap("later work failed after result commit");
    }
    funding_status()
}

#[cdk::update(
    guard = "private_continuation",
    decode_with = "decode_later",
    manual_reply
)]
fn funding_later() {
    JOURNAL.with_borrow_mut(|state| state.later_work_entered = true);
    api::trap("injected later-work trap");
}
