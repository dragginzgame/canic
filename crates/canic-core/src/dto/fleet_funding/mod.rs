//! Module: dto::fleet_funding
//!
//! Responsibility: carry bounded Root/Coordinator funding requests and exact results.
//! Does not own: caller identity, policy decisions, persistence, accounting, or effects.
//! Boundary: transport callers supply no recipient; the Coordinator derives it from the caller.

use crate::{cdk::types::Cycles, dto::fleet_registry::FleetRegistryVersion};
use candid::CandidType;
use serde::{Deserialize, Serialize};

/// Frozen ingress envelope used by both sides of the funding command exchange.
pub const MAX_FLEET_ROOT_FUNDING_COMMAND_PAYLOAD_BYTES: usize =
    crate::ingress::payload::DEFAULT_UPDATE_INGRESS_MAX_BYTES;

/// Root-authored request for one exact Coordinator operating-cycle decision.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetRootFundingRequest {
    pub operation_id: [u8; 32],
    pub operation_sequence: u64,
    pub expected_registry: FleetRegistryVersion,
    pub observed_balance: Cycles,
    pub requested_cycles: Cycles,
    pub policy_hash: [u8; 32],
}

/// Coordinator-authored exact same-Root acceptance request carrying cycles.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetRootFundingAcceptanceRequest {
    pub operation_id: [u8; 32],
    pub operation_sequence: u64,
    pub expected_registry: FleetRegistryVersion,
    pub observed_balance: Cycles,
    pub granted_cycles: Cycles,
    pub policy_hash: [u8; 32],
}

/// Root-authored exact receipt for one fresh or replayed grant acceptance.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetRootFundingAcceptanceReceipt {
    pub request: FleetRootFundingAcceptanceRequest,
    pub fleet_subnet_root: candid::Principal,
    pub coordinator: candid::Principal,
    pub accepted_at_ns: u64,
}

/// Terminal reason that one authenticated request transferred no cycles.
#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum FleetRootFundingNoGrantReason {
    CooldownActive,
    CoordinatorReserveUnavailable,
    FleetAutomaticCapExhausted,
    FleetWindowExhausted,
    FundingDisabled,
    InvalidRequest,
    PolicyMismatch,
    RegistryStale,
    RootIneligible,
    RootRejected,
    RootAutomaticCapExhausted,
    RootWindowExhausted,
}

/// Durable exact zero-transfer result for one Root operation.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetRootFundingNoGrantReceipt {
    pub request: FleetRootFundingRequest,
    pub reason: FleetRootFundingNoGrantReason,
    pub decided_at_ns: u64,
}

/// Exact terminal outcome returned to the authenticated requesting Root.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum FleetRootFundingResponse {
    Granted(FleetRootFundingAcceptanceReceipt),
    NoGrant(FleetRootFundingNoGrantReceipt),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{
        AppId, CanonicalNetworkId, FleetBinding, FleetCoordinatorBinding, FleetId, FleetKey,
        FleetRegistryAuthority, SubnetId,
    };
    use candid::{Decode, Encode, Principal};

    #[test]
    fn exact_funding_commands_roundtrip_below_the_frozen_ingress_envelope() {
        let request = FleetRootFundingRequest {
            operation_id: [1; 32],
            operation_sequence: u64::MAX,
            expected_registry: registry(),
            observed_balance: Cycles::new(u128::MAX),
            requested_cycles: Cycles::new(u128::MAX),
            policy_hash: [2; 32],
        };
        let bytes = Encode!(&request).expect("encode Root funding request");
        assert!(bytes.len() <= MAX_FLEET_ROOT_FUNDING_COMMAND_PAYLOAD_BYTES);
        assert_eq!(
            Decode!(&bytes, FleetRootFundingRequest).expect("decode Root funding request"),
            request
        );

        let acceptance = FleetRootFundingAcceptanceRequest {
            operation_id: request.operation_id,
            operation_sequence: request.operation_sequence,
            expected_registry: request.expected_registry,
            observed_balance: request.observed_balance,
            granted_cycles: request.requested_cycles,
            policy_hash: request.policy_hash,
        };
        let bytes = Encode!(&acceptance).expect("encode funding acceptance request");
        assert!(bytes.len() <= MAX_FLEET_ROOT_FUNDING_COMMAND_PAYLOAD_BYTES);
        assert_eq!(
            Decode!(&bytes, FleetRootFundingAcceptanceRequest)
                .expect("decode funding acceptance request"),
            acceptance
        );
    }

    fn registry() -> FleetRegistryVersion {
        let coordinator = Principal::from_slice(&[3; 29]);
        FleetRegistryVersion {
            authority: FleetRegistryAuthority {
                binding: FleetCoordinatorBinding {
                    fleet: FleetBinding {
                        fleet: FleetKey {
                            canonical_network_id: CanonicalNetworkId::ic_mainnet(),
                            fleet_id: FleetId::from_generated_bytes([4; 32]),
                        },
                        app: AppId::from("funding-payload-proof"),
                    },
                    coordinator_subnet: SubnetId::from_principal(Principal::from_slice(&[5; 29])),
                    coordinator,
                },
                epoch: u64::MAX,
            },
            revision: u64::MAX,
            content_hash: [6; 32],
        }
    }
}
