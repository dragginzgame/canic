//! Bounded Canic transports for instruction audit validation.
//!
//! Keep command tables and reply decoding limited to the exercised methods.
//! Canonical compatibility checks cover every retained selector and payload.

use crate::dto::{
    auth::{RootIssuerConfigureRequest, RootIssuerConfigureResponse},
    capability::{RootCapabilityEnvelopeV1, RootCapabilityResponseV1},
};

/// Bounded RootCommand selectors for instruction audit validation.
#[derive(ic_platform::candid::contract::CandidType)]
#[candid_path("ic_platform::candid::contract")]
pub enum RootCommand {
    RespondCapability(RootCapabilityEnvelopeV1),
    ConfigureIssuer(RootIssuerConfigureRequest),
}

/// Bounded RootCommandResponse selectors for instruction audit validation.
#[derive(ic_platform::candid::contract::CandidType, serde::Deserialize)]
#[candid_path("ic_platform::candid::contract")]
pub enum RootCommandResponse {
    RespondCapability(RootCapabilityResponseV1),
    ConfigureIssuer(RootIssuerConfigureResponse),
}
