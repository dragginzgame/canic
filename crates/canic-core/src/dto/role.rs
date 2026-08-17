//! Module: dto::role
//!
//! Responsibility: carry the shared role-owned status boundary values.
//! Does not own: capability derivation, authorization, status lookup, or workflow policy.
//! Boundary: role-specific status enums compose these passive bounded values.

use crate::{
    dto::{
        component_registry::ComponentRuntimeStatusResponse,
        fleet_activation::FleetActivationStatusResponse,
        fleet_registry::FleetSubnetRootDrainingReservationResponse,
        metadata::CanicMetadataResponse, metrics::MetricsKind, page::PageRequest, prelude::*,
    },
    log::Level,
};

/// Closed compiled capability identity exposed by the immutable role overview.
#[derive(
    CandidType, Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize,
)]
#[remain::sorted]
pub enum RoleCapability {
    AutomaticTopup,
    DelegatedTokenIssuer,
    DelegatedTokenVerifier,
    FleetCoordinator,
    Icrc21,
    Index,
    RoleAttestationSigner,
    RoleAttestationVerifier,
    Root,
    RootControlPlane,
    Runtime,
    Scaling,
    Sharding,
    WasmStore,
}

/// Immutable role, capability, release, and bootstrap profile-verification response.
#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct RoleOverviewResponse {
    pub role: CanisterRole,
    pub capabilities: Vec<RoleCapability>,
    pub protocol_profile_digest: [u8; 32],
    pub metadata: CanicMetadataResponse,
    pub bootstrap: BootstrapStatusResponse,
}

/// Lookup key shared by each role's local durable-operation status variant.
#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OperationStatusRequest {
    pub operation_id: [u8; 32],
}

/// Durable identity returned when one role-owned asynchronous command is accepted.
#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OperationReceipt {
    pub operation_id: [u8; 32],
}

/// Exact Coordinator reservation delivered with one accepted Root-removal intent.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootRemovalRequest {
    pub reservation: FleetSubnetRootDrainingReservationResponse,
}

/// Named response for the protected compiled configuration source.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ConfigStatusResponse {
    pub toml: String,
}

/// Named response for the canister's current cycle balance.
#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CycleBalanceStatusResponse {
    pub cycles: u128,
}

/// Bounded log observation selector used by each role status surface.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct LogStatusRequest {
    pub crate_name: Option<String>,
    pub topic: Option<String>,
    pub min_level: Option<Level>,
    pub page: PageRequest,
}

/// Bounded metrics observation selector used by each role status surface.
#[derive(CandidType, Clone, Copy, Debug, Deserialize)]
pub struct MetricsStatusRequest {
    pub kind: MetricsKind,
    pub page: PageRequest,
}

/// Managed runtime configuration detail projected through the operation lane.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ComponentRuntimeOperationStatus {
    pub operation_id: [u8; 32],
    pub fleet_activation: FleetActivationStatusResponse,
    pub runtime: ComponentRuntimeStatusResponse,
}

pub use crate::dto::state::BootstrapStatusResponse;

#[cfg(test)]
mod tests {
    use super::*;
    use candid::{Decode, Encode};

    #[test]
    fn shared_role_status_requests_round_trip_through_candid() {
        let operation = OperationStatusRequest {
            operation_id: [7; 32],
        };
        let bytes = Encode!(&operation).expect("encode operation status request");
        assert_eq!(
            Decode!(&bytes, OperationStatusRequest).expect("decode operation status request"),
            operation
        );

        let receipt = OperationReceipt {
            operation_id: operation.operation_id,
        };
        let bytes = Encode!(&receipt).expect("encode operation receipt");
        assert_eq!(
            Decode!(&bytes, OperationReceipt).expect("decode operation receipt"),
            receipt
        );

        let logs = LogStatusRequest {
            crate_name: Some("canic-core".to_string()),
            topic: Some("Cycles".to_string()),
            min_level: Some(Level::Warn),
            page: PageRequest {
                limit: 25,
                offset: 50,
            },
        };
        let bytes = Encode!(&logs).expect("encode log status request");
        assert_eq!(
            Decode!(&bytes, LogStatusRequest).expect("decode log status request"),
            logs
        );
    }
}
