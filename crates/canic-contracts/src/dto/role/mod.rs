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

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[remain::sorted]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub enum RoleCapability {
    AutomaticTopup,
    CallerAuthority,
    ChildProvisioning,
    DelegatedTokenIssuer,
    DelegatedTokenVerifier,
    FleetAdmissionProjection,
    FleetCoordinator,
    Icrc21,
    Index,
    ObservabilityDiagnostics,
    ObservabilityHistory,
    ObservabilityLogs,
    ObservabilityMetrics,
    RoleAttestationSigner,
    RoleAttestationVerifier,
    Root,
    RootControlPlane,
    RootDelegation,
    Runtime,
    Scaling,
    Sharding,
    WasmStore,
}

/// Immutable role, capability, release, and bootstrap profile-verification response.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct RoleOverviewResponse {
    pub role: CanisterRole,
    pub capabilities: Vec<RoleCapability>,
    pub protocol_profile_digest: [u8; 32],
    pub metadata: CanicMetadataResponse,
    pub bootstrap: BootstrapStatusResponse,
}

/// Lookup key shared by each role's local durable-operation status variant.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct OperationStatusRequest {
    pub operation_id: [u8; 32],
}

/// Durable identity returned when one role-owned asynchronous command is accepted.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct OperationReceipt {
    pub operation_id: [u8; 32],
}

/// Exact Coordinator reservation delivered with one accepted Root-removal intent.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootRemovalRequest {
    pub reservation: FleetSubnetRootDrainingReservationResponse,
}

/// Named response for the protected compiled configuration source.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct ConfigStatusResponse {
    pub toml: String,
}

/// Named response for the canister's current cycle balance.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct CycleBalanceStatusResponse {
    pub cycles: u128,
}

/// Bounded log observation selector used by each role status surface.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct LogStatusRequest {
    pub crate_name: Option<String>,
    pub topic: Option<String>,
    pub min_level: Option<Level>,
    pub page: PageRequest,
}

/// Bounded metrics observation selector used by each role status surface.
#[derive(Clone, Copy, Debug, Deserialize, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct MetricsStatusRequest {
    pub kind: MetricsKind,
    pub page: PageRequest,
}

/// Managed runtime configuration detail projected through the operation lane.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ComponentRuntimeOperationStatus {
    pub operation_id: [u8; 32],
    pub fleet_activation: FleetActivationStatusResponse,
    pub runtime: ComponentRuntimeStatusResponse,
}

pub use crate::dto::state::BootstrapStatusResponse;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_role_status_requests_round_trip_through_wire() {
        let operation = OperationStatusRequest {
            operation_id: [7; 32],
        };
        let bytes = crate::protocol::authority::encode(&operation)
            .expect("encode operation status request");
        assert_eq!(
            crate::protocol::authority::decode::<OperationStatusRequest>(&bytes)
                .expect("decode operation status request"),
            operation
        );

        let receipt = OperationReceipt {
            operation_id: operation.operation_id,
        };
        let bytes = crate::protocol::authority::encode(&receipt).expect("encode operation receipt");
        assert_eq!(
            crate::protocol::authority::decode::<OperationReceipt>(&bytes)
                .expect("decode operation receipt"),
            receipt
        );
    }

    #[test]
    fn log_filters_round_trip_through_native_wire() {
        use crate::protocol::protected_logs::{METHOD_ID, REQUEST_LIMITS};
        let logs = LogStatusRequest {
            crate_name: Some("canic-core".to_string()),
            topic: Some("Cycles".to_string()),
            min_level: Some(Level::Warn),
            page: PageRequest {
                limit: 25,
                offset: 50,
            },
        };
        let bytes = ic_wire::native::encode_request(METHOD_ID, &logs, REQUEST_LIMITS)
            .expect("encode log request");
        assert_eq!(
            ic_wire::native::decode_request::<LogStatusRequest>(METHOD_ID, &bytes, REQUEST_LIMITS)
                .expect("decode log request"),
            logs
        );
    }
}
