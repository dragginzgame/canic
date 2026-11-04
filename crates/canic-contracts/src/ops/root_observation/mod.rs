//! Convert internal observation routing results to fixed native reply shapes.
use crate::dto::{self, observability::CanisterObservabilityResponse};

impl TryFrom<CanisterObservabilityResponse> for dto::child_funding::ChildFundingUsage {
    type Error = CanisterObservabilityResponse;
    fn try_from(value: CanisterObservabilityResponse) -> Result<Self, Self::Error> {
        match value {
            CanisterObservabilityResponse::ChildFunding(value) => Ok(value),
            other => Err(other),
        }
    }
}

impl TryFrom<CanisterObservabilityResponse> for dto::role::CycleBalanceStatusResponse {
    type Error = CanisterObservabilityResponse;
    fn try_from(value: CanisterObservabilityResponse) -> Result<Self, Self::Error> {
        match value {
            CanisterObservabilityResponse::CycleBalance(value) => Ok(value),
            other => Err(other),
        }
    }
}

impl TryFrom<CanisterObservabilityResponse> for dto::page::Page<dto::cycles::CycleTrackerEntry> {
    type Error = CanisterObservabilityResponse;
    fn try_from(value: CanisterObservabilityResponse) -> Result<Self, Self::Error> {
        match value {
            CanisterObservabilityResponse::CycleHistory(value) => Ok(value),
            other => Err(other),
        }
    }
}

impl TryFrom<CanisterObservabilityResponse>
    for dto::page::Page<dto::cycle_topups::CycleTopupEvent>
{
    type Error = CanisterObservabilityResponse;
    fn try_from(value: CanisterObservabilityResponse) -> Result<Self, Self::Error> {
        match value {
            CanisterObservabilityResponse::CycleTopups(value) => Ok(value),
            other => Err(other),
        }
    }
}

impl TryFrom<CanisterObservabilityResponse> for dto::memory::MemoryAllocationsResponse {
    type Error = CanisterObservabilityResponse;
    fn try_from(value: CanisterObservabilityResponse) -> Result<Self, Self::Error> {
        match value {
            CanisterObservabilityResponse::MemoryAllocations(value) => Ok(value),
            other => Err(other),
        }
    }
}

impl TryFrom<CanisterObservabilityResponse> for dto::page::Page<dto::metric_page::MetricEntry> {
    type Error = CanisterObservabilityResponse;
    fn try_from(value: CanisterObservabilityResponse) -> Result<Self, Self::Error> {
        match value {
            CanisterObservabilityResponse::Metrics(value) => Ok(value),
            other => Err(other),
        }
    }
}

impl TryFrom<CanisterObservabilityResponse>
    for Result<dto::portable_inspection::PolicyReport, dto::portable_inspection::Refusal>
{
    type Error = CanisterObservabilityResponse;
    fn try_from(value: CanisterObservabilityResponse) -> Result<Self, Self::Error> {
        match value {
            CanisterObservabilityResponse::PortablePolicy(value) => Ok(value),
            other => Err(other),
        }
    }
}
impl TryFrom<CanisterObservabilityResponse>
    for Result<dto::portable_inspection::Page, dto::portable_inspection::Refusal>
{
    type Error = CanisterObservabilityResponse;
    fn try_from(value: CanisterObservabilityResponse) -> Result<Self, Self::Error> {
        match value {
            CanisterObservabilityResponse::PortableSessions(value) => Ok(value),
            other => Err(other),
        }
    }
}
