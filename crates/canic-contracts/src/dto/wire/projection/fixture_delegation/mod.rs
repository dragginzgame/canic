//! Bounded Canic transports for delegation validation.
//!
//! Keep fixture command tables and selected reply decoding bounded to the
//! tested operation. Selectors may dispatch to several methods or roles in
//! authorization cases; compatibility tests name those canonical contracts.
//! Application probe payloads and external IC protocols stay with the fixture.

use crate::dto::auth::DelegatedToken;
use crate::dto::auth::DelegatedTokenGetRequest;
use crate::dto::auth::DelegatedTokenPrepareRequest;
use crate::dto::auth::DelegatedTokenPrepareResponse;
use candid::CandidType;
use serde::Deserialize;

/// Bounded transport selectors used by delegation validation.
#[derive(CandidType)]
pub enum CanisterCommand {
    PrepareDelegatedToken(DelegatedTokenPrepareRequest),
}

/// Bounded transport selectors used by delegation validation.
#[derive(CandidType, Deserialize)]
pub enum CanisterCommandResponse {
    PrepareDelegatedToken(DelegatedTokenPrepareResponse),
}

/// Bounded transport selectors used by delegation validation.
#[derive(CandidType)]
pub enum CanisterStatusRequest {
    DelegatedToken(DelegatedTokenGetRequest),
}

/// Bounded transport selectors used by delegation validation.
#[derive(CandidType, Deserialize)]
pub enum CanisterStatusResponse {
    DelegatedToken(DelegatedToken),
}
