//! Coordinate real fixture awaits, callback loss and the controlled callee gate.

use super::{
    ops,
    protocol::{AfterCall, CallObservation, CallRequest, ExternalRequest, TargetRequest},
};
use canic::Error;
use std::{future::Future, task::Poll};

pub(super) async fn execute(input: CallRequest) -> Result<CallObservation, Error> {
    let _invocation = ops::Invocation;
    let after = input.after.clone();
    let call = ops::prepare(input);
    let cost = ops::cost(&call);
    let mut future = Box::pin(ops::execute(call));
    if matches!(after, AfterCall::DropUnpolled | AfterCall::DropSubmitted) {
        if matches!(after, AfterCall::DropSubmitted) {
            // Poll the actual SDK future in its protected executor, then drop it.
            // This is local cancellation, not authority over the remote effect.
            std::future::poll_fn(|cx| {
                assert!(matches!(future.as_mut().poll(cx), Poll::Pending));
                Poll::Ready(())
            })
            .await;
        }
        drop(future);
        return Ok(ops::unobserved(cost));
    }
    let result = future.await;
    // Preserve the original callback's refund before decoding or another await.
    let callback = ops::callback(&result);
    if matches!(after, AfterCall::TrapCallback) {
        ops::trap_callback();
    }
    Ok(ops::project(result, callback, cost, after))
}
pub(super) async fn target(request: TargetRequest) {
    ops::enter(&request);
    // Each real self-call yields an IC boundary for release and deadline expiry.
    while ops::held(request.id) {
        ops::round().await;
    }
    ops::reply(request);
}
pub(super) async fn external(request: ExternalRequest) -> Result<(bool, u64), Error> {
    ops::status(request.target).await?;
    let fee = ops::fee(request.ledger).await?;
    Ok((true, fee))
}
