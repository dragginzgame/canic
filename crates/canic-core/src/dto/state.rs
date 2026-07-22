use crate::dto::prelude::*;

pub use crate::domain::state::{AppMode, AppStatus};

//
// AppCommand
//

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub enum AppCommand {
    SetStatus(AppStatus),
    SetCyclesFundingEnabled(bool),
}

//
// AppCommandResponse
//

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub enum AppCommandResponse {
    Status(SetStateResponse<AppStatus>),
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
// AppStateInput
//

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub struct AppStateInput {
    pub mode: AppMode,
    pub cycles_funding_enabled: bool,
}

//
// AppStateResponse
//

#[derive(CandidType, Deserialize)]
pub struct AppStateResponse {
    pub mode: AppMode,
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
    fn app_mode_roundtrips_candid_through_dto_path() {
        assert_enum_candid_contract(AppMode::Readonly);
    }

    #[test]
    fn app_status_roundtrips_candid_through_dto_path() {
        assert_enum_candid_contract(AppStatus::Readonly);
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
