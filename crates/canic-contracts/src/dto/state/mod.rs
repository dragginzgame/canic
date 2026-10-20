use crate::dto::prelude::*;

pub use crate::values::state::{FleetMode, FleetStatus};

/// Explicit input for changing automatic cycles funding.
#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[candid_path("ic_platform::candid::contract")]
pub struct SetCyclesFundingRequest {
    pub enabled: bool,
}

/// Explicit input for changing the Fleet's public status.
#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[candid_path("ic_platform::candid::contract")]
pub struct SetFleetStatusRequest {
    pub status: FleetStatus,
}

//
// FleetCommand
//

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[candid_path("ic_platform::candid::contract")]
pub enum FleetCommand {
    SetStatus(FleetStatus),
    SetCyclesFundingEnabled(bool),
}

//
// FleetCommandResponse
//

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[candid_path("ic_platform::candid::contract")]
pub enum FleetCommandResponse {
    Status(SetStateResponse<FleetStatus>),
    CyclesFundingEnabled(SetStateResponse<bool>),
}

//
// SetStateResponse
//

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[candid_path("ic_platform::candid::contract")]
pub struct SetStateResponse<T> {
    pub previous: T,
    pub current: T,
    pub changed: bool,
}

/// Local mutation receipt and the independently observed downstream results.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
#[candid_path("ic_platform::candid::contract")]
pub struct FleetStateCommandResult<T> {
    pub change: SetStateResponse<T>,
    pub propagation: crate::dto::cascade::StateCascadeReport,
    pub reconciliation_error: Option<crate::dto::error::Error>,
}

/// Workflow result before the endpoint selects its command-specific response.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
#[candid_path("ic_platform::candid::contract")]
pub struct FleetCommandExecutionResponse {
    pub change: FleetCommandResponse,
    pub propagation: crate::dto::cascade::StateCascadeReport,
    pub reconciliation_error: Option<crate::dto::error::Error>,
}

//
// FleetStateInput
//

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[candid_path("ic_platform::candid::contract")]
pub struct FleetStateInput {
    pub mode: FleetMode,
    pub cycles_funding_enabled: bool,
}

//
// FleetStateResponse
//

#[derive(CandidType, Deserialize)]
#[candid_path("ic_platform::candid::contract")]
pub struct FleetStateResponse {
    pub mode: FleetMode,
    pub cycles_funding_enabled: bool,
}

//
// BootstrapStatusResponse
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
#[candid_path("ic_platform::candid::contract")]
pub struct BootstrapStatusResponse {
    pub ready: bool,
    pub phase: String,
    pub last_error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use ic_platform::candid::contract::{Decode, Encode};
    use serde::de::DeserializeOwned;
    use std::fmt::Debug;

    #[test]
    fn fleet_mode_roundtrips_candid_through_dto_path() {
        assert_enum_candid_contract(FleetMode::Readonly);
    }

    #[test]
    fn fleet_status_roundtrips_candid_through_dto_path() {
        assert_enum_candid_contract(FleetStatus::Readonly);
    }

    fn assert_enum_candid_contract<T>(value: T)
    where
        T: CandidType + Clone + Debug + DeserializeOwned + Eq,
    {
        let bytes = Encode!(&value).expect("encode state enum");
        let decoded = Decode!(&bytes, T).expect("decode state enum");

        assert_eq!(decoded, value);
    }
}
