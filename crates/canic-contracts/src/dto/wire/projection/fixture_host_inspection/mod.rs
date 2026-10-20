//! Synthetic inspection replies retain only the Host decoder's required fields.
//!
//! Keep status, controllers, module hash, and cycles. Real Root replies include the
//! complete management record; selector and retained field parity is checked here.

use ic_platform::{
    candid::contract::{CandidType, Nat},
    principal::ExternalPrincipal as Principal,
};

#[derive(CandidType)]
#[candid_path("ic_platform::candid::contract")]
pub enum FixturePoolInspectionResponse {
    InspectCanister(FixturePoolInspection),
}
#[derive(CandidType)]
#[candid_path("ic_platform::candid::contract")]
pub struct FixturePoolInspection {
    pub status: crate::dto::canister::CanisterStatusType,
    pub cycles: Nat,
    pub module_hash: Option<Vec<u8>>,
    pub settings: FixturePoolControllers,
}
#[derive(CandidType)]
#[candid_path("ic_platform::candid::contract")]
pub struct FixturePoolControllers {
    pub controllers: Vec<Principal>,
}
