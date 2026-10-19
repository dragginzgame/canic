//! Bounded Canic transports for root topology validation.
//!
//! Keep fixture command tables and selected reply decoding bounded to the
//! tested operation. Selectors may dispatch to several methods or roles in
//! authorization cases; compatibility tests name those canonical contracts.
//! Application probe payloads and external IC protocols stay with the fixture.

use crate::dto::{
    canister::CanisterInfo,
    page::{Page, PageRequest},
    template::WasmStoreOverviewResponse,
};
use ic_platform::candid::contract::CandidType;
use serde::Deserialize;

/// Bounded transport selectors used by root topology validation.
#[derive(CandidType)]
#[candid_path("ic_platform::candid::contract")]
pub enum RootStatusRequest {
    Children(PageRequest),
    StoreOverview,
}

/// Bounded transport selectors used by root topology validation.
#[derive(CandidType, Deserialize)]
#[candid_path("ic_platform::candid::contract")]
pub enum RootStatusResponse {
    Children(Page<CanisterInfo>),
    StoreOverview(WasmStoreOverviewResponse),
}
