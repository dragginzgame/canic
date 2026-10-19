//! Bounded Canic transports for fleet_registry baseline tests capacity_import validation.
//!
//! Keep fixture command tables and selected reply decoding bounded to the
//! tested operation. Selectors may dispatch to several methods or roles in
//! authorization cases; compatibility tests name those canonical contracts.
//! Application probe payloads and external IC protocols stay with the fixture.

use crate::dto::{
    pool_import::{PoolImportContext, PoolImportIdentity, PoolImportStatus},
    root::RootPoolReleaseResponse,
};
use ic_platform::candid::contract::CandidType;
use serde::Deserialize;

/// Bounded transport selectors used by fleet_registry baseline tests capacity_import validation.
#[derive(CandidType)]
#[candid_path("ic_platform::candid::contract")]
pub enum StatusRequest {
    PoolImport(PoolImportIdentity),
    PoolImportContext,
    PoolRelease,
}

/// Bounded transport selectors used by fleet_registry baseline tests capacity_import validation.
#[derive(CandidType, Deserialize)]
#[candid_path("ic_platform::candid::contract")]
pub enum StatusResponse {
    PoolImport(Box<PoolImportStatus>),
    PoolImportContext(Box<PoolImportContext>),
    PoolRelease(Box<RootPoolReleaseResponse>),
}
