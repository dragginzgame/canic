//! Exact bounded Wire observations; application errors retain the remote owner.
use crate::{
    InternalError,
    dto::{
        error::Error,
        observability::{CanisterObservabilityRequest, CanisterObservabilityResponse},
    },
    ops::ic::call::execute as execute_call,
};
use canic_contracts::protocol::observability_relay as protocol;
use ic_call::{Call, WaitPolicy};
use ic_platform::principal::ExternalPrincipal as Principal;
use serde::{Serialize, de::DeserializeOwned};

/// Perform one replicated read with the existing unbounded-wait call semantics.
pub async fn observe(
    target: Principal,
    request: CanisterObservabilityRequest,
) -> Result<CanisterObservabilityResponse, InternalError> {
    match request {
        CanisterObservabilityRequest::ChildFunding(request) => {
            let reply: canic_contracts::dto::child_funding::ChildFundingUsage = read(
                target,
                protocol::child_funding::METHOD,
                protocol::child_funding::METHOD_ID,
                &ic_platform::Principal::from(request),
                protocol::child_funding::REQUEST_LIMITS,
                protocol::child_funding::RESPONSE_LIMITS,
            )
            .await?;
            Ok(CanisterObservabilityResponse::ChildFunding(reply.into()))
        }
        CanisterObservabilityRequest::CycleBalance => {
            let reply: canic_contracts::dto::role::CycleBalanceStatusResponse = read(
                target,
                protocol::cycle_balance::METHOD,
                protocol::cycle_balance::METHOD_ID,
                &(),
                protocol::cycle_balance::REQUEST_LIMITS,
                protocol::cycle_balance::RESPONSE_LIMITS,
            )
            .await?;
            Ok(CanisterObservabilityResponse::CycleBalance(reply))
        }
        CanisterObservabilityRequest::CycleHistory(request) => {
            let reply: canic_contracts::dto::page::Page<
                canic_contracts::dto::cycles::CycleTrackerEntry,
            > = read(
                target,
                protocol::cycle_history::METHOD,
                protocol::cycle_history::METHOD_ID,
                &request,
                protocol::cycle_history::REQUEST_LIMITS,
                protocol::cycle_history::RESPONSE_LIMITS,
            )
            .await?;
            Ok(CanisterObservabilityResponse::CycleHistory(reply))
        }
        CanisterObservabilityRequest::CycleTopups(request) => {
            let reply: canic_contracts::dto::page::Page<
                canic_contracts::dto::cycle_topups::CycleTopupEvent,
            > = read(
                target,
                protocol::cycle_topups::METHOD,
                protocol::cycle_topups::METHOD_ID,
                &request,
                protocol::cycle_topups::REQUEST_LIMITS,
                protocol::cycle_topups::RESPONSE_LIMITS,
            )
            .await?;
            Ok(CanisterObservabilityResponse::CycleTopups(reply.into()))
        }
        CanisterObservabilityRequest::MemoryAllocations => {
            let reply: canic_contracts::dto::memory::MemoryAllocationsResponse = read(
                target,
                protocol::memory_allocations::METHOD,
                protocol::memory_allocations::METHOD_ID,
                &(),
                protocol::memory_allocations::REQUEST_LIMITS,
                protocol::memory_allocations::RESPONSE_LIMITS,
            )
            .await?;
            Ok(CanisterObservabilityResponse::MemoryAllocations(reply))
        }
        CanisterObservabilityRequest::Metrics(request) => {
            let reply: canic_contracts::dto::page::Page<
                canic_contracts::dto::metric_page::MetricEntry,
            > = read(
                target,
                protocol::metrics::METHOD,
                protocol::metrics::METHOD_ID,
                &request,
                protocol::metrics::REQUEST_LIMITS,
                protocol::metrics::RESPONSE_LIMITS,
            )
            .await?;
            Ok(CanisterObservabilityResponse::Metrics(reply.into()))
        }
    }
}

async fn read<T: Serialize + Sync, R: DeserializeOwned>(
    target: Principal,
    method: &str,
    id: u64,
    request: &T,
    request_limits: ic_wire::Limits,
    response_limits: ic_wire::Limits,
) -> Result<R, InternalError> {
    let bytes = ic_wire::native::encode_request(id, request, request_limits)
        .map_err(|_| InternalError::invariant())?;
    let reply =
        execute_call(Call::new(target.into(), method, WaitPolicy::Unbounded).take_raw_args(bytes))
            .await?
            .into_bytes();
    let value: Result<R, Error> = ic_wire::native::decode_response(id, &reply, response_limits)
        .map_err(|_| InternalError::invariant())?;
    value.map_err(InternalError::observed_public)
}
