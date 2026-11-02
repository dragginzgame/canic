//! Module: workflow::runtime::cycles::fixture
//!
//! Responsibility: drive the maintained funding attempt and RPC in disposable IC fixtures.
//! Does not own: funding policy, replay storage or an alternative transfer implementation.
//! Boundary: available only with internal-test-fixtures; endpoint fixtures authenticate callers.

use crate::{
    dto::{error::Error, rpc::CyclesResponse},
    model::replay::OperationId,
    ops::{
        ic::IcOps,
        rpc::request::RequestOps,
        storage::async_job_recovery::{AsyncJobCompletion, AsyncJobOwner, AsyncJobRecoveryOps},
    },
    workflow::runtime::async_job::AsyncJobWorkflow,
};
use std::cell::Cell;

/// Exact Wire identity for the disposable application's held self-call.
pub const CALLBACK_GATE_ID: u64 = 0x5445_5354_0000_050b;
/// Wire identity for one replicated round within the disposable held method.
pub const CALLBACK_ROUND_ID: u64 = 0x5445_5354_0000_050c;
pub const CALLBACK_GATE_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 256,
    allocation: 1024,
    nodes: 32,
    depth: 8,
};

/// Selected fault boundary for a disposable production top-up callback.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CallbackFault {
    None,
    BeforeAwait,
    AfterAwait,
    HoldReply,
}

/// Observations of actual callback entry and its retained business attempt.
pub struct CallbackProbe {
    pub entered: u64,
    pub operation_id: Option<[u8; 32]>,
    pub lease_deadline_ns: Option<u64>,
    pub held: bool,
}

thread_local! {
    static CALLBACK_FAULT: Cell<CallbackFault> = const { Cell::new(CallbackFault::None) };
    static CALLBACK_ENTRIES: Cell<u64> = const { Cell::new(0) };
    static CALLBACK_OPERATION: Cell<Option<[u8; 32]>> = const { Cell::new(None) };
    static CALLBACK_GATE: Cell<bool> = const { Cell::new(false) };
}

/// Change the fixture boundary, releasing a real held self-call when selected.
pub fn configure_callback(fault: CallbackFault) {
    CALLBACK_FAULT.set(fault);
}

/// Inspect current attempt evidence without changing scheduling or storage.
#[must_use]
pub fn callback_probe() -> CallbackProbe {
    CallbackProbe {
        entered: CALLBACK_ENTRIES.get(),
        operation_id: CALLBACK_OPERATION.get(),
        lease_deadline_ns: AsyncJobRecoveryOps::active_lease_deadline(AsyncJobOwner::CycleTopup),
        held: CALLBACK_GATE.get(),
    }
}

/// Keep the application's real self-call pending until its controller releases it.
pub async fn hold_callback_reply() {
    CALLBACK_GATE.set(true);
    while CALLBACK_FAULT.get() == CallbackFault::HoldReply {
        // Each actual IC call keeps the protected gate method alive and gives
        // the controller a replicated-message boundary at which to release it.
        let request = ic_wire::native::encode_request(CALLBACK_ROUND_ID, &(), CALLBACK_GATE_LIMITS)
            .expect("encode fixture gate round");
        let response = ic_platform::cdk::call::Call::unbounded_wait(
            IcOps::canister_self().into(),
            "test_topup_callback_round",
        )
        .take_raw_args(request)
        .await
        .expect("fixture gate round");
        ic_wire::native::decode_response::<Result<(), Error>>(
            CALLBACK_ROUND_ID,
            &response.into_bytes(),
            CALLBACK_GATE_LIMITS,
        )
        .expect("decode fixture gate round")
        .expect("admit fixture gate round");
    }
    CALLBACK_GATE.set(false);
}

pub(super) fn before_callback(attempt: crate::ops::storage::async_job_recovery::AsyncJobAttempt) {
    CALLBACK_ENTRIES.set(CALLBACK_ENTRIES.get().saturating_add(1));
    CALLBACK_OPERATION.set(
        attempt
            .operation_id(IcOps::canister_self())
            .map(OperationId::into_bytes),
    );
    if CALLBACK_FAULT.get() == CallbackFault::BeforeAwait {
        IcOps::trap("disposable top-up first-segment trap");
    }
}

pub(super) async fn after_callback() {
    match CALLBACK_FAULT.get() {
        CallbackFault::AfterAwait => IcOps::trap("disposable top-up post-await trap"),
        CallbackFault::HoldReply => {
            let request =
                ic_wire::native::encode_request(CALLBACK_GATE_ID, &(), CALLBACK_GATE_LIMITS)
                    .expect("encode fixture self-call");
            let response = ic_platform::cdk::call::Call::unbounded_wait(
                IcOps::canister_self().into(),
                "test_topup_callback_gate",
            )
            .take_raw_args(request)
            .await
            .expect("fixture held self-call");
            ic_wire::native::decode_response::<Result<(), Error>>(
                CALLBACK_GATE_ID,
                &response.into_bytes(),
                CALLBACK_GATE_LIMITS,
            )
            .expect("decode fixture self-call")
            .expect("admit fixture self-call");
        }
        CallbackFault::None | CallbackFault::BeforeAwait => {}
    }
}

/// Exercise the real durable identity and parent RPC, optionally discarding its reply.
pub async fn request(
    cycles: u128,
    discard_reply: bool,
) -> Result<([u8; 32], Option<CyclesResponse>), Error> {
    let attempt = AsyncJobWorkflow::claim(AsyncJobOwner::CycleTopup)
        .map_err(|_| Error::from_registered(crate::diagnostics::codes::STATE_CONFLICT))?;
    let operation = attempt
        .operation_id(IcOps::canister_self())
        .expect("cycle owner identity");
    let response = RequestOps::request_cycles_with_operation_id(cycles, operation).await;
    let response = match response {
        Ok(response) => response,
        Err(error) => {
            let outcome = super::CycleWorkflow::finish_parent_funding_failure(
                &cycles.into(),
                operation,
                &error,
            );
            let _ = AsyncJobWorkflow::finish(attempt, outcome);
            return Err(error.into());
        }
    };
    let completion = if discard_reply {
        AsyncJobCompletion::RetryableFailure
    } else {
        AsyncJobCompletion::Success
    };
    assert!(AsyncJobRecoveryOps::finish(
        attempt,
        completion,
        IcOps::now_nanos()
    )?);
    if discard_reply {
        // Deliberately lose the successfully delivered result at the caller's commit boundary.
        // The next invocation must use its persisted pending generation.
        Ok((operation.into_bytes(), None))
    } else {
        Ok((operation.into_bytes(), Some(response)))
    }
}

/// Deliberately submit another actor's identity through the maintained parent transport.
pub async fn collide(cycles: u128, operation_id: [u8; 32]) -> Result<CyclesResponse, Error> {
    RequestOps::request_cycles_with_operation_id(cycles, OperationId::from_bytes(operation_id))
        .await
        .map_err(Into::into)
}
