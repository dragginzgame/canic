use crate::dto::prelude::*;

pub use crate::values::state::{FleetMode, FleetStatus};

/// Explicit input for changing automatic cycles funding.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct SetCyclesFundingRequest {
    pub enabled: bool,
}

/// Explicit input for changing the Fleet's public status.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct SetFleetStatusRequest {
    pub status: FleetStatus,
}

//
// FleetCommand
//

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, serde::Serialize)]
pub enum FleetCommand {
    SetStatus(FleetStatus),
    SetCyclesFundingEnabled(bool),
}

//
// FleetCommandResponse
//

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, serde::Serialize)]
pub enum FleetCommandResponse {
    Status(SetStateResponse<FleetStatus>),
    CyclesFundingEnabled(SetStateResponse<bool>),
}

//
// SetStateResponse
//

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct SetStateResponse<T> {
    pub previous: T,
    pub current: T,
    pub changed: bool,
}

/// Local mutation receipt and the independently observed downstream results.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, serde::Serialize)]
pub struct FleetStateCommandResult<T> {
    pub change: SetStateResponse<T>,
    pub propagation: crate::dto::cascade::StateCascadeReport,
    pub reconciliation_error: Option<crate::dto::error::Error>,
}

/// Workflow result before the endpoint selects its command-specific response.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, serde::Serialize)]
pub struct FleetCommandExecutionResponse {
    pub change: FleetCommandResponse,
    pub propagation: crate::dto::cascade::StateCascadeReport,
    pub reconciliation_error: Option<crate::dto::error::Error>,
}

//
// FleetStateInput
//

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct FleetStateInput {
    pub mode: FleetMode,
    pub cycles_funding_enabled: bool,
}

//
// FleetStateResponse
//

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct FleetStateResponse {
    pub mode: FleetMode,
    pub cycles_funding_enabled: bool,
}

//
// BootstrapStatusResponse
//

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct BootstrapStatusResponse {
    pub ready: bool,
    pub phase: String,
    pub last_error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    use serde::de::DeserializeOwned;
    use std::fmt::Debug;

    #[test]
    fn fleet_mode_roundtrips_wire_through_dto_path() {
        assert_enum_wire_contract(FleetMode::Readonly);
    }

    #[test]
    fn fleet_status_roundtrips_wire_through_dto_path() {
        assert_enum_wire_contract(FleetStatus::Readonly);
    }

    fn assert_enum_wire_contract<T>(value: T)
    where
        T: serde::Serialize + Clone + Debug + DeserializeOwned + Eq,
    {
        let bytes = crate::protocol::authority::encode(&value).expect("encode state enum");
        let decoded = crate::protocol::authority::decode::<T>(&bytes).expect("decode state enum");

        assert_eq!(decoded, value);
    }
}
