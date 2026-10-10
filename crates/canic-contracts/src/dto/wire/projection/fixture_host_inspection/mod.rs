//! Synthetic inspection replies retain only the Host decoder's required fields.
//!
//! Keep status, controllers, module hash, and cycles. Real Root replies include the
//! complete management record; selector and retained field parity is checked here.

use candid::{CandidType, Nat, Principal};

#[derive(CandidType)]
pub enum FixturePoolInspectionResponse {
    InspectCanister(FixturePoolInspection),
}
#[derive(CandidType)]
pub struct FixturePoolInspection {
    pub status: crate::dto::canister::CanisterStatusType,
    pub cycles: Nat,
    pub module_hash: Option<Vec<u8>>,
    pub settings: FixturePoolControllers,
}
#[derive(CandidType)]
pub struct FixturePoolControllers {
    pub controllers: Vec<Principal>,
}
