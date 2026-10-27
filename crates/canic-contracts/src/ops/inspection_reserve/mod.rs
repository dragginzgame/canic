//! Lossless reserve projection; management admission retains its existing owner.
use crate::dto::{
    canister::CanisterInspectionReserveResponse, inspection_reserve::InspectionReserve,
};
impl From<CanisterInspectionReserveResponse> for InspectionReserve {
    fn from(value: CanisterInspectionReserveResponse) -> Self {
        Self {
            caller: value.caller.into(),
            canister_id: value.canister_id.into(),
            native_cycles: value.native_cycles,
            available_liquid_cycles: value.available_liquid_cycles,
            required_liquid_cycles: value.required_liquid_cycles,
        }
    }
}
impl From<InspectionReserve> for CanisterInspectionReserveResponse {
    fn from(value: InspectionReserve) -> Self {
        Self {
            caller: value.caller.into(),
            canister_id: value.canister_id.into(),
            native_cycles: value.native_cycles,
            available_liquid_cycles: value.available_liquid_cycles,
            required_liquid_cycles: value.required_liquid_cycles,
        }
    }
}
