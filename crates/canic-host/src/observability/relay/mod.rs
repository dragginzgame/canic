//! Typed outer relay calls after exact release-schema authority has been verified.
use crate::{
    canister_protocol::{CanisterProtocolError, call_wire},
    icp::IcpCli,
    release_set::BuiltWireSchema,
};
use canic_contracts::{
    dto::{
        self,
        observability::{CanisterObservabilityRequest, CanisterObservabilityResponse},
    },
    protocol::root_observation as protocol,
};
use ic_platform::principal::ExternalPrincipal as Principal;

/// Read one exact target through Root, retaining the selected schema proof at the call boundary.
pub fn observe(
    icp: &IcpCli,
    _schema: &BuiltWireSchema,
    root: Principal,
    target: Principal,
    request: CanisterObservabilityRequest,
) -> Result<CanisterObservabilityResponse, CanisterProtocolError> {
    match request {
        CanisterObservabilityRequest::ChildFunding(input) => {
            let reply: dto::child_funding::ChildFundingUsage = call_wire(
                icp,
                root,
                protocol::child_funding::METHOD,
                protocol::child_funding::METHOD_ID,
                &dto::root_observation::Request {
                    canister_id: target.into(),
                    input: ic_platform::Principal::from(input),
                },
                protocol::child_funding::REQUEST_LIMITS,
                protocol::child_funding::RESPONSE_LIMITS,
            )?;
            Ok(CanisterObservabilityResponse::ChildFunding(reply.into()))
        }
        CanisterObservabilityRequest::CycleBalance => {
            let reply: dto::role::CycleBalanceStatusResponse = call_wire(
                icp,
                root,
                protocol::cycle_balance::METHOD,
                protocol::cycle_balance::METHOD_ID,
                &dto::root_observation::Request {
                    canister_id: target.into(),
                    input: (),
                },
                protocol::cycle_balance::REQUEST_LIMITS,
                protocol::cycle_balance::RESPONSE_LIMITS,
            )?;
            Ok(CanisterObservabilityResponse::CycleBalance(reply))
        }
        CanisterObservabilityRequest::CycleHistory(input) => {
            let reply: dto::page::Page<dto::cycles::CycleTrackerEntry> = call_wire(
                icp,
                root,
                protocol::cycle_history::METHOD,
                protocol::cycle_history::METHOD_ID,
                &dto::root_observation::Request {
                    canister_id: target.into(),
                    input,
                },
                protocol::cycle_history::REQUEST_LIMITS,
                protocol::cycle_history::RESPONSE_LIMITS,
            )?;
            Ok(CanisterObservabilityResponse::CycleHistory(reply))
        }
        CanisterObservabilityRequest::CycleTopups(input) => {
            let reply: dto::page::Page<dto::cycle_topups::CycleTopupEvent> = call_wire(
                icp,
                root,
                protocol::cycle_topups::METHOD,
                protocol::cycle_topups::METHOD_ID,
                &dto::root_observation::Request {
                    canister_id: target.into(),
                    input,
                },
                protocol::cycle_topups::REQUEST_LIMITS,
                protocol::cycle_topups::RESPONSE_LIMITS,
            )?;
            Ok(CanisterObservabilityResponse::CycleTopups(reply.into()))
        }
        CanisterObservabilityRequest::MemoryAllocations => {
            let reply: dto::memory::MemoryAllocationsResponse = call_wire(
                icp,
                root,
                protocol::memory_allocations::METHOD,
                protocol::memory_allocations::METHOD_ID,
                &dto::root_observation::Request {
                    canister_id: target.into(),
                    input: (),
                },
                protocol::memory_allocations::REQUEST_LIMITS,
                protocol::memory_allocations::RESPONSE_LIMITS,
            )?;
            Ok(CanisterObservabilityResponse::MemoryAllocations(reply))
        }
        CanisterObservabilityRequest::Metrics(input) => {
            let reply: dto::page::Page<dto::metric_page::MetricEntry> = call_wire(
                icp,
                root,
                protocol::metrics::METHOD,
                protocol::metrics::METHOD_ID,
                &dto::root_observation::Request {
                    canister_id: target.into(),
                    input,
                },
                protocol::metrics::REQUEST_LIMITS,
                protocol::metrics::RESPONSE_LIMITS,
            )?;
            Ok(CanisterObservabilityResponse::Metrics(reply.into()))
        }
    }
}
