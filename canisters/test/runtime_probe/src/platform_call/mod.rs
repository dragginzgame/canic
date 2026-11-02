//! Qualification endpoints for the actual production call adapters and IC callbacks.

mod codec;
mod ops;
pub mod protocol;
mod workflow;

use canic::Error;
use protocol::{
    CallMetrics, CallObservation, CallRequest, LIMITS, TargetObservation, TargetRequest,
};

#[canic::update(public, wire(id = protocol::EXECUTE, request = LIMITS, response = LIMITS))]
async fn platform_call_execute(input: CallRequest) -> Result<CallObservation, Error> {
    workflow::execute(input).await
}
#[canic::query(public, wire(id = protocol::METRICS, request = LIMITS, response = LIMITS))]
fn platform_call_metrics() -> Result<CallMetrics, Error> {
    Ok(ops::metrics())
}
canic::__canic_register_raw_wire_schema!("platform_call_target",Update,protocol::TARGET,TargetRequest,Result<u64,Error>,LIMITS,LIMITS);
#[ic_platform::cdk::update(decode_with = "codec::target", manual_reply = true)]
async fn platform_call_target(request: TargetRequest) {
    workflow::target(request).await;
}
#[canic::update(public, wire(id = protocol::ROUND, request = LIMITS, response = LIMITS))]
async fn platform_call_round() -> Result<(), Error> {
    Ok(())
}
#[canic::update(public, wire(id = protocol::RELEASE, request = LIMITS, response = LIMITS))]
async fn platform_call_release(id: u8) -> Result<(), Error> {
    ops::release(id)
}
#[canic::query(public, wire(id = protocol::OBSERVE, request = LIMITS, response = LIMITS))]
fn platform_call_observe() -> Result<TargetObservation, Error> {
    Ok(ops::observe())
}
#[canic::update(public, wire(id = protocol::EXTERNAL, request = LIMITS, response = LIMITS))]
async fn platform_call_external(request: protocol::ExternalRequest) -> Result<(bool, u64), Error> {
    workflow::external(request).await
}
#[canic::update(public, wire(id = protocol::CASCADE, request = LIMITS, response = LIMITS))]
async fn platform_call_cascade(target: ic_platform::Principal) -> Result<(), Error> {
    ops::cascade(target).await
}
