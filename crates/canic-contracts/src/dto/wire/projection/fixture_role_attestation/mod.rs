//! Bounded Canic transports for fleet_registry role_attestation validation.
//!
//! Keep fixture command tables and selected reply decoding bounded to the
//! tested operation. Selectors may dispatch to several methods or roles in
//! authorization cases; compatibility tests name those canonical contracts.
//! Application probe payloads and external IC protocols stay with the fixture.

use crate::dto::auth::RoleAttestationGetRequest;
use crate::dto::auth::RoleAttestationPrepareResponse;
use crate::dto::auth::RoleAttestationRequest;
use crate::dto::auth::SignedRoleAttestation;
use crate::dto::metrics::MetricEntry;
use crate::dto::page::Page;
use crate::dto::role::MetricsStatusRequest;
use candid::CandidType;
use serde::Deserialize;

/// Bounded transport selectors used by fleet_registry role_attestation validation.
#[derive(CandidType)]
pub enum RootCommand {
    PrepareRoleAttestation(RoleAttestationRequest),
}

/// Bounded transport selectors used by fleet_registry role_attestation validation.
#[derive(CandidType, Debug, Deserialize)]
pub enum RootCommandResponse {
    PrepareRoleAttestation(RoleAttestationPrepareResponse),
}

/// Bounded transport selectors used by fleet_registry role_attestation validation.
#[derive(CandidType)]
pub enum RootStatusRequest {
    RoleAttestation(RoleAttestationGetRequest),
}

/// Bounded transport selectors used by fleet_registry role_attestation validation.
#[derive(CandidType, Deserialize)]
pub enum RootStatusResponse {
    RoleAttestation(SignedRoleAttestation),
}

/// Bounded transport selectors used by fleet_registry role_attestation validation.
#[derive(CandidType)]
pub enum ManagedStatusRequest {
    Metrics(MetricsStatusRequest),
}

/// Bounded transport selectors used by fleet_registry role_attestation validation.
#[derive(CandidType, Deserialize)]
pub enum ManagedStatusResponse {
    Metrics(Page<MetricEntry>),
}
