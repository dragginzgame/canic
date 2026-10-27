//! Native cycle-conservation evidence and exact observation framing.
use super::*;
use crate::dto::{
    retirement_control::{LedgerTransferIntent, LedgerTransferReceipt},
    root_removal_read::*,
};
use ic_platform::Principal;

fn ledger_receipt() -> LedgerTransferReceipt {
    LedgerTransferReceipt {
        intent: LedgerTransferIntent {
            source: Principal::from_slice(&[1]),
            destination: Principal::from_slice(&[2]),
            balance_before: u128::MAX,
            fee: 3,
            created_at_time: 4,
            memo: [5; 32],
        },
        block_index: Some(u128::MAX),
    }
}

#[test]
fn store_reclamation_preserves_complete_evidence() {
    let value = StoreReclamation {
        operation_id: [10; 32],
        fleet_subnet_root: Principal::from_slice(&[11]),
        wasm_store: Principal::from_slice(&[12]),
        final_inventory_hash: [13; 32],
        reclaimed_store_bytes: u64::MAX - 14,
        reclaimed_catalog_entries: u32::MAX - 15,
        reclaimed_template_count: u32::MAX - 16,
        reclaimed_release_count: u32::MAX - 17,
        gc_prepared_at_secs: u64::MAX - 18,
        gc_started_at_secs: u64::MAX - 19,
        gc_completed_at_secs: u64::MAX - 20,
        gc_runs_completed: u32::MAX - 21,
        completed_at_ns: u64::MAX - 22,
        reclamation_hash: [23; 32],
    };
    let canonical =
        crate::dto::fleet_subnet_root::FleetSubnetRootStoreReclamationResponse::from(value.clone());
    let bytes = ic_wire::native::encode_response(METHOD_ID, &value, RESPONSE_LIMITS).unwrap();
    let decoded: StoreReclamation =
        ic_wire::native::decode_response(METHOD_ID, &bytes, RESPONSE_LIMITS).unwrap();
    assert_eq!(decoded, value);
    assert_eq!(
        crate::dto::fleet_subnet_root::FleetSubnetRootStoreReclamationResponse::from(decoded),
        canonical
    );
    assert_eq!(StoreReclamation::from(canonical), value);
}

#[test]
fn store_finalization_preserves_complete_evidence() {
    let value = StoreFinalization {
        operation_id: [10; 32],
        fleet_subnet_root: Principal::from_slice(&[11]),
        wasm_store: Principal::from_slice(&[12]),
        final_inventory_hash: [13; 32],
        reclamation_hash: [14; 32],
        source_generation: u64::MAX - 15,
        finalized_generation: u64::MAX - 16,
        finalized_at_secs: u64::MAX - 17,
        completed_at_ns: u64::MAX - 18,
        finalization_hash: [19; 32],
    };
    let canonical =
        crate::dto::fleet_subnet_root::FleetSubnetRootStoreBindingFinalizationResponse::from(
            value.clone(),
        );
    let bytes = ic_wire::native::encode_response(METHOD_ID, &value, RESPONSE_LIMITS).unwrap();
    let decoded: StoreFinalization =
        ic_wire::native::decode_response(METHOD_ID, &bytes, RESPONSE_LIMITS).unwrap();
    assert_eq!(decoded, value);
    assert_eq!(
        crate::dto::fleet_subnet_root::FleetSubnetRootStoreBindingFinalizationResponse::from(
            decoded
        ),
        canonical
    );
    assert_eq!(StoreFinalization::from(canonical), value);
}

#[test]
fn store_deletion_preserves_complete_evidence() {
    let value = StoreDeletion {
        operation_id: [10; 32],
        fleet_subnet_root: Principal::from_slice(&[11]),
        wasm_store: Principal::from_slice(&[12]),
        binding_finalization_hash: [13; 32],
        observed_module_hash: [14; 32],
        observed_controllers: vec![Principal::from_slice(&[15]), Principal::from_slice(&[16])],
        observed_cycles_before_reclamation: u128::MAX - 16,
        retained_cycles_target: u128::MAX - 17,
        observed_cycles_after_reclamation: u128::MAX - 18,
        cycles_reclaimed_at_ns: u64::MAX - 19,
        prepared_at_ns: u64::MAX - 20,
        observed_absent_at_ns: u64::MAX - 21,
        completed_at_ns: u64::MAX - 22,
        deletion_hash: [23; 32],
    };
    let canonical =
        crate::dto::fleet_subnet_root::FleetSubnetRootStoreDeletionResponse::from(value.clone());
    let bytes = ic_wire::native::encode_response(METHOD_ID, &value, RESPONSE_LIMITS).unwrap();
    let decoded: StoreDeletion =
        ic_wire::native::decode_response(METHOD_ID, &bytes, RESPONSE_LIMITS).unwrap();
    assert_eq!(decoded, value);
    assert_eq!(
        crate::dto::fleet_subnet_root::FleetSubnetRootStoreDeletionResponse::from(decoded),
        canonical
    );
    assert_eq!(StoreDeletion::from(canonical), value);
}

#[test]
fn preparation_preserves_complete_evidence() {
    let value = Preparation {
        ledger_receipt: ledger_receipt(),
        operation_id: [11; 32],
        fleet_subnet_root: Principal::from_slice(&[12]),
        coordinator: Principal::from_slice(&[13]),
        final_inventory_hash: [14; 32],
        store_deletion_hash: [15; 32],
        observed_cycles_before_reclamation: u128::MAX - 16,
        retained_cycles_target: u128::MAX - 17,
        observed_reserved_cycles: u128::MAX - 18,
        observed_idle_cycles_burned_per_day: u128::MAX - 19,
        observed_freezing_threshold_seconds: u128::MAX - 20,
        observed_cycles_after_reclamation: u128::MAX - 21,
        cycles_reclaimed_at_ns: u64::MAX - 22,
        coordinator_intent_hash: [23; 32],
        coordinator_readiness_hash: [24; 32],
        prepared_at_ns: u64::MAX - 25,
        completed_at_ns: u64::MAX - 26,
    };
    let canonical = crate::dto::fleet_subnet_root::FleetSubnetRootDeletionPreparationResponse::from(
        value.clone(),
    );
    let bytes = ic_wire::native::encode_response(METHOD_ID, &value, RESPONSE_LIMITS).unwrap();
    let decoded: Preparation =
        ic_wire::native::decode_response(METHOD_ID, &bytes, RESPONSE_LIMITS).unwrap();
    assert_eq!(decoded, value);
    assert_eq!(
        crate::dto::fleet_subnet_root::FleetSubnetRootDeletionPreparationResponse::from(decoded),
        canonical
    );
    assert_eq!(Preparation::from(canonical), value);
}

#[test]
fn root_removal_requires_its_exact_method_frame() {
    let operation = [31_u8; 32];
    let bytes = ic_wire::native::encode_request(METHOD_ID, &operation, REQUEST_LIMITS).unwrap();
    assert_eq!(
        ic_wire::native::decode_request::<[u8; 32]>(METHOD_ID, &bytes, REQUEST_LIMITS).unwrap(),
        operation
    );
    assert!(matches!(
        ic_wire::native::decode_request::<[u8; 32]>(
            crate::protocol::component_removal_read::METHOD_ID,
            &bytes,
            REQUEST_LIMITS
        ),
        Err(ic_wire::Error::Identity)
    ));
}

#[cfg(feature = "wire-schema")]
#[test]
fn root_removal_schema_covers_all_receipts() {
    ic_wire::typescript::native_schema::<Result<Status, crate::dto::error::Error>>().unwrap();
}
