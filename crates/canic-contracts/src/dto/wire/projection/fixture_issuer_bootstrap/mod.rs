//! Bounded Canic transports for issuer bootstrap validation.
//!
//! Keep command tables and reply decoding limited to the exercised methods.
//! Canonical compatibility checks cover every retained selector and payload.

use crate::dto::auth::{
    ActiveDelegationProofStatusResponse, DelegatedToken, DelegatedTokenGetRequest,
    RootIssuerConfigureRequest, RootIssuerConfigureResponse, RootIssuerRenewalStatusRequest,
};

/// Bounded RootCommand selectors for issuer bootstrap validation.
#[derive(candid::CandidType)]
pub enum RootCommand {
    ConfigureIssuer(RootIssuerConfigureRequest),
}

/// Bounded RootCommandResponse selectors for issuer bootstrap validation.
#[derive(candid::CandidType, serde::Deserialize)]
pub enum RootCommandResponse {
    ConfigureIssuer(RootIssuerConfigureResponse),
}

/// Bounded RootStatusRequest selectors for issuer bootstrap validation.
#[derive(candid::CandidType)]
pub enum RootStatusRequest {
    IssuerRenewal(RootIssuerRenewalStatusRequest),
}

/// Bounded IssuerStatusRequest selectors for issuer bootstrap validation.
#[derive(candid::CandidType)]
pub enum IssuerStatusRequest {
    ActiveDelegationProof,
    DelegatedToken(DelegatedTokenGetRequest),
}

/// Bounded IssuerStatusResponse selectors for issuer bootstrap validation.
#[derive(candid::CandidType, serde::Deserialize)]
#[expect(
    clippy::large_enum_variant,
    reason = "mirrors the canonical issuer status wire variants"
)]
pub enum IssuerStatusResponse {
    ActiveDelegationProof(ActiveDelegationProofStatusResponse),
    DelegatedToken(DelegatedToken),
}
