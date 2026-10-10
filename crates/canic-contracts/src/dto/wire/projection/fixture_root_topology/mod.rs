//! Bounded Canic transports for root topology validation.
//!
//! Keep fixture command tables and selected reply decoding bounded to the
//! tested operation. Selectors may dispatch to several methods or roles in
//! authorization cases; compatibility tests name those canonical contracts.
//! Application probe payloads and external IC protocols stay with the fixture.

use crate::dto::canister::CanisterInfo;
use crate::dto::page::Page;
use crate::dto::page::PageRequest;
use crate::dto::template::WasmStoreOverviewResponse;
use candid::CandidType;
use serde::Deserialize;

/// Bounded transport selectors used by root topology validation.
#[derive(CandidType)]
pub enum RootStatusRequest {
    Children(PageRequest),
    StoreOverview,
}

/// Bounded transport selectors used by root topology validation.
#[derive(CandidType, Deserialize)]
pub enum RootStatusResponse {
    Children(Page<CanisterInfo>),
    StoreOverview(WasmStoreOverviewResponse),
}
