//! Bounded Canic transports for fleet_registry role_attestation validation.
//!
//! Keep fixture command tables and selected reply decoding bounded to the
//! tested operation. Selectors may dispatch to several methods or roles in
//! authorization cases; compatibility tests name those canonical contracts.
//! Application probe payloads and external IC protocols stay with the fixture.

use crate::dto::{
    auth::{
        RoleAttestationGetRequest, RoleAttestationPrepareResponse, RoleAttestationRequest,
        SignedRoleAttestation,
    },
    metrics::MetricEntry,
    page::Page,
    role::MetricsStatusRequest,
};
use ic_platform::candid::contract::CandidType;
use serde::Deserialize;

/// Bounded transport selectors used by fleet_registry role_attestation validation.
#[derive(CandidType)]
#[candid_path("ic_platform::candid::contract")]
pub enum RootCommand {
    PrepareRoleAttestation(RoleAttestationRequest),
}

/// Bounded transport selectors used by fleet_registry role_attestation validation.
#[derive(CandidType, Debug, Deserialize)]
#[candid_path("ic_platform::candid::contract")]
pub enum RootCommandResponse {
    PrepareRoleAttestation(RoleAttestationPrepareResponse),
}

/// Bounded transport selectors used by fleet_registry role_attestation validation.
#[derive(CandidType)]
#[candid_path("ic_platform::candid::contract")]
pub enum RootStatusRequest {
    RoleAttestation(RoleAttestationGetRequest),
}

/// Bounded transport selectors used by fleet_registry role_attestation validation.
#[derive(CandidType, Deserialize)]
#[candid_path("ic_platform::candid::contract")]
pub enum RootStatusResponse {
    RoleAttestation(SignedRoleAttestation),
}

/// Bounded transport selectors used by fleet_registry role_attestation validation.
#[derive(CandidType)]
#[candid_path("ic_platform::candid::contract")]
pub enum ManagedStatusRequest {
    Metrics(MetricsStatusRequest),
}

/// Bounded transport selectors used by fleet_registry role_attestation validation.
#[derive(CandidType, Deserialize)]
#[candid_path("ic_platform::candid::contract")]
pub enum ManagedStatusResponse {
    Metrics(Page<MetricEntry>),
}
