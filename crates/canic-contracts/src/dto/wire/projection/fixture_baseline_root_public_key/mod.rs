//! Bounded Canic transports for fleet_registry baseline tests root_public_key validation.
//!
//! Keep fixture command tables and selected reply decoding bounded to the
//! tested operation. Selectors may dispatch to several methods or roles in
//! authorization cases; compatibility tests name those canonical contracts.
//! Application probe payloads and external IC protocols stay with the fixture.

use crate::dto::auth::RootChainKeyPublicKeyRequest;
use candid::CandidType;
use serde::Deserialize;

/// Bounded transport selectors used by fleet_registry baseline tests root_public_key validation.
#[derive(CandidType)]
pub enum Command {
    GetChainKeyPublicKey(RootChainKeyPublicKeyRequest),
}

/// Bounded transport selectors used by fleet_registry baseline tests root_public_key validation.
#[derive(CandidType, Deserialize)]
pub enum Response {
    GetChainKeyPublicKey(Vec<u8>),
}
