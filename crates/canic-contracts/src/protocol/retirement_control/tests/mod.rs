//! Native lifecycle receipts preserve canonical evidence and strict method identity.
use super::*;
use crate::{
    dto::{
        error::Error,
        fleet_registry::{
            FleetLedgerTransferIntent, FleetLedgerTransferReceipt, FleetRegistryVersion,
            FleetRetirementRequest, FleetRetirementStatus,
        },
        retirement_control,
    },
    ids::*,
};
use ic_platform::principal::ExternalPrincipal as Principal;

fn version() -> FleetRegistryVersion {
    FleetRegistryVersion {
        authority: FleetRegistryAuthority {
            binding: FleetCoordinatorBinding {
                fleet: FleetBinding {
                    fleet: FleetKey {
                        canonical_network_id: CanonicalNetworkId::ic_mainnet(),
                        fleet_id: FleetId::from_generated_bytes([7; 32]),
                    },
                    app: AppId::from("demo"),
                },
                coordinator_subnet: SubnetId::from_principal(Principal::from_slice(&[2; 29])),
                coordinator: Principal::from_slice(&[3; 29]),
                recovery_controllers: vec![Principal::from_slice(&[4; 29])],
            },
            epoch: u64::MAX,
        },
        revision: u64::MAX,
        content_hash: [255; 32],
    }
}

#[test]
fn retirement_receipt_preserves_original_transfer_and_canonical_bytes() {
    let intent = FleetLedgerTransferIntent {
        source: Principal::from_slice(&[5; 29]),
        destination: Principal::from_slice(&[6; 29]),
        balance_before: u128::MAX,
        fee: 10,
        created_at_time: u64::MAX,
        memo: [7; 32],
    };
    let original = FleetRetirementStatus {
        request: FleetRetirementRequest {
            operation_id: [8; 32],
            expected_registry: version(),
            destination: intent.destination,
            maximum_ledger_fee: u128::MAX,
        },
        prepared_at_ns: u64::MAX - 1,
        ledger_transfer: Some(intent.clone()),
        ledger_receipt: Some(FleetLedgerTransferReceipt {
            intent,
            block_index: Some(u128::MAX),
        }),
    };
    let response: Result<retirement_control::RetirementStatus, Error> = Ok(original.clone().into());
    let bytes =
        ic_wire::native::encode_response(retire::METHOD_ID, &response, RESPONSE_LIMITS).unwrap();
    let decoded: Result<retirement_control::RetirementStatus, Error> =
        ic_wire::native::decode_response(retire::METHOD_ID, &bytes, RESPONSE_LIMITS).unwrap();
    let canonical = FleetRetirementStatus::from(decoded.unwrap());
    assert_eq!(canonical, original);
    assert_eq!(
        crate::protocol::authority::encode(&canonical).unwrap(),
        crate::protocol::authority::encode(&original).unwrap()
    );
    assert!(matches!(
        ic_wire::native::decode_response::<Result<retirement_control::RetirementStatus, Error>>(
            remove::METHOD_ID,
            &bytes,
            RESPONSE_LIMITS
        ),
        Err(ic_wire::Error::Identity)
    ));
    let mut trailing = bytes;
    trailing.push(0);
    assert!(
        ic_wire::native::decode_response::<Result<retirement_control::RetirementStatus, Error>>(
            retire::METHOD_ID,
            &trailing,
            RESPONSE_LIMITS
        )
        .is_err()
    );
}

#[cfg(feature = "wire-schema")]
#[test]
fn removal_and_retirement_have_complete_native_metadata() {
    ic_wire::typescript::native_schema::<retirement_control::RemovalRequest>().unwrap();
    ic_wire::typescript::native_schema::<retirement_control::RootRemovalRequest>().unwrap();
    ic_wire::typescript::native_schema::<retirement_control::RetirementRequest>().unwrap();
    ic_wire::typescript::native_schema::<Result<crate::dto::role::OperationReceipt, Error>>()
        .unwrap();
    ic_wire::typescript::native_schema::<Result<retirement_control::RetirementStatus, Error>>()
        .unwrap();
}

#[test]
fn operation_read_preserves_exact_id_and_rejects_another_method() {
    let operation_id = [0x42_u8; 32];
    let bytes = ic_wire::native::encode_request(
        super::status::METHOD_ID,
        &operation_id,
        super::REQUEST_LIMITS,
    )
    .unwrap();
    assert_eq!(
        ic_wire::native::decode_request::<[u8; 32]>(
            super::status::METHOD_ID,
            &bytes,
            super::REQUEST_LIMITS
        )
        .unwrap(),
        operation_id
    );
    assert!(matches!(
        ic_wire::native::decode_request::<[u8; 32]>(
            crate::protocol::public_overview::METHOD_ID,
            &bytes,
            super::REQUEST_LIMITS
        ),
        Err(ic_wire::Error::Identity)
    ));
}

#[cfg(feature = "wire-schema")]
#[test]
fn operation_read_registers_complete_native_evidence() {
    ic_wire::typescript::native_schema::<
        Result<crate::dto::retirement_control::RetirementStatus, crate::dto::error::Error>,
    >()
    .unwrap();
}

#[test]
fn removal_observation_cannot_alias_retirement() {
    let operation_id = [0x43_u8; 32];
    let bytes = ic_wire::native::encode_request(
        super::removal_status::METHOD_ID,
        &operation_id,
        super::REQUEST_LIMITS,
    )
    .unwrap();
    assert_eq!(
        ic_wire::native::decode_request::<[u8; 32]>(
            super::removal_status::METHOD_ID,
            &bytes,
            super::REQUEST_LIMITS
        )
        .unwrap(),
        operation_id
    );
    assert!(matches!(
        ic_wire::native::decode_request::<[u8; 32]>(
            super::status::METHOD_ID,
            &bytes,
            super::REQUEST_LIMITS
        ),
        Err(ic_wire::Error::Identity)
    ));
}

#[cfg(feature = "wire-schema")]
#[test]
fn removal_schema_contains_complete_retained_receipts() {
    ic_wire::typescript::native_schema::<
        Result<crate::dto::retirement_control::RemovalStatus, crate::dto::error::Error>,
    >()
    .unwrap();
}

#[test]
fn deletion_readiness_preserves_original_ledger_and_cycle_authority() {
    use crate::dto::retirement_control::{
        LedgerTransferIntent, LedgerTransferReceipt, Readiness, ReadinessRequest,
    };
    let root = ic_platform::Principal::from_slice(&[1]);
    let coordinator = ic_platform::Principal::from_slice(&[2]);
    let evidence = Readiness {
        request: ReadinessRequest {
            ledger_receipt: LedgerTransferReceipt {
                intent: LedgerTransferIntent {
                    source: root,
                    destination: coordinator,
                    balance_before: u128::MAX - 5,
                    fee: 4,
                    created_at_time: 3,
                    memo: [2; 32],
                },
                block_index: Some(u128::MAX - 1),
            },
            operation_id: [1; 32],
            fleet_subnet_root: root,
            expected_intent_hash: [3; 32],
            observed_cycles_after_reclamation: 8,
            cycles_reclaimed_at_ns: 9,
        },
        coordinator,
        final_inventory_hash: [4; 32],
        store_deletion_hash: [5; 32],
        observed_cycles_before_reclamation: u128::MAX,
        retained_cycles_target: 10,
        observed_reserved_cycles: 11,
        observed_idle_cycles_burned_per_day: 12,
        observed_freezing_threshold_seconds: 13,
        prepared_at_ns: 14,
        recorded_at_ns: 15,
        readiness_hash: [6; 32],
    };
    let canonical: crate::dto::fleet_registry::FleetSubnetRootDeletionReadinessResponse =
        evidence.clone().into();
    assert_eq!(
        canonical.request.ledger_receipt.intent.source,
        ic_platform::principal::ExternalPrincipal::from(root)
    );
    assert_eq!(
        canonical.request.ledger_receipt.intent.destination,
        ic_platform::principal::ExternalPrincipal::from(coordinator)
    );
    assert_eq!(
        canonical.request.ledger_receipt.block_index,
        Some(u128::MAX - 1)
    );
    assert_eq!(Readiness::from(canonical), evidence);
}
