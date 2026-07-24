use crate::dto::prelude::*;

pub use crate::domain::state::{FleetMode, FleetStatus};

//
// FleetCommand
//

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub enum FleetCommand {
    SetStatus(FleetStatus),
    SetCyclesFundingEnabled(bool),
}

//
// FleetCommandResponse
//

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub enum FleetCommandResponse {
    Status(SetStateResponse<FleetStatus>),
    CyclesFundingEnabled(SetStateResponse<bool>),
}

//
// SetStateResponse
//

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub struct SetStateResponse<T> {
    pub previous: T,
    pub current: T,
    pub changed: bool,
}

//
// FleetStateInput
//

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub struct FleetStateInput {
    pub mode: FleetMode,
    pub cycles_funding_enabled: bool,
}

//
// FleetStateResponse
//

#[derive(CandidType, Deserialize)]
pub struct FleetStateResponse {
    pub mode: FleetMode,
    pub cycles_funding_enabled: bool,
}

//
// BootstrapStatusResponse
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct BootstrapStatusResponse {
    pub ready: bool,
    pub phase: String,
    pub last_error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use candid::{Decode, Encode};
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
