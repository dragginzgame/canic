//! Bounded fixture observations and single-step production/platform operations.

use super::protocol::{
    AfterCall, CallMetrics, CallObservation, CallRequest, LIMITS, ROUND, TARGET, TargetObservation,
    TargetReply, TargetRequest,
};
use canic::{
    __internal::core::{
        api::runtime::call_fixture::{
            self as production, PlatformCallMetricMode, PlatformCallMetricOutcome,
            PlatformCallMetricSurface, PlatformCallMetrics,
        },
        control_plane_support::error::InternalError,
    },
    Error,
    diagnostics::codes,
};
use ic_platform::{
    call::{Call, Response, WaitPolicy},
    cdk::api,
};
use std::cell::{Cell, RefCell};

pub(super) use production::execute;

thread_local! {
    static DROPPED: Cell<u64> = const { Cell::new(0) };
    static CALLS: RefCell<[u64; 8]> = const { RefCell::new([0; 8]) };
    static ACCEPTED: RefCell<[u128; 8]> = const { RefCell::new([0; 8]) };
    static HELD: RefCell<[bool; 8]> = const { RefCell::new([false; 8]) };
}

/// Witness that the actual suspended invocation is destroyed, including IC cleanup.
pub(super) struct Invocation;
impl Drop for Invocation {
    fn drop(&mut self) {
        DROPPED.set(DROPPED.get() + 1);
    }
}

/// Callback-only facts, independent of application settlement or retry authority.
pub(super) struct CallbackFacts {
    rejection_code: Option<u32>,
    refunded: Option<u128>,
}

pub(super) fn prepare(input: CallRequest) -> Call<'static, 'static> {
    let wait = input
        .timeout_seconds
        .map_or(WaitPolicy::Unbounded, |timeout_seconds| {
            WaitPolicy::Bounded { timeout_seconds }
        });
    Call::new(input.target, input.method, wait)
        .take_raw_args(input.bytes)
        .with_cycles(input.cycles)
}
pub(super) fn cost(call: &Call<'_, '_>) -> u128 {
    call.get_cost()
}
pub(super) fn callback(result: &Result<Response, InternalError>) -> CallbackFacts {
    let rejection_code = result
        .as_ref()
        .err()
        .and_then(InternalError::platform_rejection_code);
    let refunded = (result.is_ok() || rejection_code.is_some()).then(api::msg_cycles_refunded);
    CallbackFacts {
        rejection_code,
        refunded,
    }
}
pub(super) const fn unobserved(cost: u128) -> CallObservation {
    CallObservation {
        result: None,
        decoded: None,
        rejection_code: None,
        refunded: None,
        cost,
    }
}
pub(super) fn project(
    result: Result<Response, InternalError>,
    callback: CallbackFacts,
    cost: u128,
    after: AfterCall,
) -> CallObservation {
    let result = result.map(Response::into_bytes).map_err(Error::from);
    let decoded = if matches!(after, AfterCall::DecodeWire) {
        result.as_ref().ok().map(|bytes| {
            ic_wire::native::decode_response::<Result<u64, Error>>(TARGET, bytes, LIMITS)
                .unwrap_or_else(|_| Err(Error::from_registered(codes::CODEC_INVALID)))
        })
    } else {
        None
    };
    CallObservation {
        result: Some(result),
        decoded,
        rejection_code: callback.rejection_code,
        refunded: callback.refunded,
        cost,
    }
}
pub(super) fn trap_callback() -> ! {
    api::trap("controlled call callback trap")
}
pub(super) fn metrics() -> CallMetrics {
    let mut metrics = CallMetrics {
        started: 0,
        completed: 0,
        failed: 0,
        dropped: DROPPED.get(),
        bounded_started: 0,
        unbounded_started: 0,
    };
    for (key, count) in PlatformCallMetrics::snapshot() {
        if key.surface != PlatformCallMetricSurface::Generic {
            continue;
        }
        let count = u64::try_from(count.exact_total().expect("bounded fixture counter"))
            .expect("bounded count");
        match key.outcome {
            PlatformCallMetricOutcome::Started => {
                metrics.started += count;
                match key.mode {
                    PlatformCallMetricMode::BoundedWait => metrics.bounded_started += count,
                    PlatformCallMetricMode::UnboundedWait => metrics.unbounded_started += count,
                    PlatformCallMetricMode::Update => {}
                }
            }
            PlatformCallMetricOutcome::Completed => metrics.completed += count,
            PlatformCallMetricOutcome::Failed => metrics.failed += count,
        }
    }
    metrics
}
pub(super) fn enter(request: &TargetRequest) {
    let index = usize::from(request.id);
    assert!(index < 8);
    CALLS.with_borrow_mut(|counts| counts[index] += 1);
    let accepted = api::msg_cycles_accept(request.accept);
    ACCEPTED.with_borrow_mut(|counts| counts[index] += accepted);
    HELD.with_borrow_mut(|held| held[index] = request.hold);
}
pub(super) fn held(id: u8) -> bool {
    HELD.with_borrow(|held| held[usize::from(id)])
}
pub(super) async fn round() {
    let bytes = ic_wire::native::encode_request(ROUND, &(), LIMITS).unwrap();
    Call::new(
        api::canister_self(),
        "platform_call_round",
        WaitPolicy::Unbounded,
    )
    .take_raw_args(bytes)
    .execute()
    .await
    .expect("fixture gate round");
}
pub(super) fn reply(request: TargetRequest) {
    match request.reply {
        TargetReply::Wire => api::msg_reply(
            ic_wire::native::encode_response(TARGET, &Ok::<u64, Error>(request.token), LIMITS)
                .unwrap(),
        ),
        TargetReply::Malformed => api::msg_reply([0, 1, 2]),
        TargetReply::Reject => api::msg_reject("controlled target rejection"),
        TargetReply::Trap => api::trap("controlled target trap"),
    }
}
pub(super) fn release(id: u8) -> Result<(), Error> {
    if id >= 8 {
        return Err(Error::from_registered(codes::CODEC_INVALID));
    }
    HELD.with_borrow_mut(|held| held[usize::from(id)] = false);
    Ok(())
}
pub(super) fn observe() -> TargetObservation {
    TargetObservation {
        calls: CALLS.with_borrow(|v| v.to_vec()),
        accepted: ACCEPTED.with_borrow(|v| v.to_vec()),
        held: HELD.with_borrow(|v| v.to_vec()),
    }
}
pub(super) async fn status(target: ic_platform::Principal) -> Result<(), Error> {
    production::MgmtOps::canister_status(target.into())
        .await
        .map(|_| ())
        .map_err(Into::into)
}
pub(super) async fn fee(ledger: ic_platform::Principal) -> Result<u64, Error> {
    let fee = production::IcpRefillOps::icrc1_fee(ledger.into()).await?;
    u64::try_from(fee.0).map_err(|_| Error::from_registered(codes::CODEC_INVALID))
}
pub(super) async fn cascade(target: ic_platform::Principal) -> Result<(), Error> {
    production::CascadeOps::send_topology_snapshot(
        target.into(),
        &canic::dto::cascade::TopologySnapshotInput {
            parents: Vec::new(),
            children_map: Vec::new(),
        },
    )
    .await
    .map_err(Into::into)
}
