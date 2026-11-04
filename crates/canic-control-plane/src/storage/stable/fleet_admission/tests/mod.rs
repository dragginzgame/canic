#![cfg(test)]

//! Coordinator admission encoding and allocation qualification.

use super::*;
use canic_contracts::ids::{
    AppId, CanonicalNetworkId, FleetAdmissionRule, FleetBinding, FleetId, FleetKey, SubnetId,
};
use canic_core::{
    cdk::structures::{Memory, StableCell, VectorMemory, storable::Storable},
    shared_support::fleet_admission_policy::compile_installed_fleet_admission_policy,
};

#[test]
fn maximum_b3_current_plus_last_authority_fits_its_allocation() {
    let fleet = FleetBinding {
        fleet: FleetKey {
            canonical_network_id: CanonicalNetworkId::ic_mainnet(),
            fleet_id: FleetId::from_generated_bytes([u8::MAX; 32]),
        },
        app: AppId::from("a".repeat(40)),
    };
    let fleet_principals = (1..=256).map(principal).collect::<Vec<_>>();
    let rules = (0..32)
        .map(|index| FleetAdmissionRule {
            selector: FleetAdmissionSelector::ComponentSpec(
                format!("s{index:02}").parse().expect("Component Spec ID"),
            ),
            principals: fleet_principals[(index * 4)..(index * 4 + 4)].to_vec(),
        })
        .collect::<Vec<_>>();
    let active = compile_installed_fleet_admission_policy(
        fleet.clone(),
        u64::MAX - 1,
        fleet_principals.clone(),
        rules.clone(),
    )
    .expect("maximum active policy");
    let successor =
        compile_installed_fleet_admission_policy(fleet.clone(), u64::MAX, fleet_principals, rules)
            .expect("maximum successor policy");
    let authority = FleetCoordinatorBinding {
        fleet,
        coordinator_subnet: SubnetId::from_principal(principal(300)),
        coordinator: principal(301),
        recovery_controllers: Vec::new(),
    };
    let roots = (1..=4_096)
        .map(|index| FleetAdmissionCoordinatorRootProgressRecord {
            fleet_subnet_root: principal(index),
            placement_subnet: SubnetId::from_principal(principal(index + 4_096)),
            phase: FleetAdmissionCoordinatorRootPhaseRecord::Open,
            participant_catalog_digest: Some([0xf7; 32]),
            participant_count: Some(1),
            last_receipt_hash: Some([0xf9; 32]),
        })
        .collect::<Vec<_>>();
    let request = FleetAdmissionMutationRequestRecord {
        authority,
        expected_generation: u64::MAX - 1,
        expected_policy_digest: active.policy_digest,
        action: FleetAdmissionMutationActionRecord::Add,
        selector: FleetAdmissionSelector::Fleet,
        principal: principal(302),
        operation_id: [0xfe; 32],
        successor_policy_digest: successor.policy_digest,
        participant_catalog_digest: [0xf6; 32],
        participant_count: 4_096,
    };
    let record = FleetAdmissionAuthorityRecord {
        schema_version: 1,
        active_policy: active,
        current_transition: Some(FleetAdmissionTransitionRecord {
            request: request.clone(),
            request_hash: [0xfd; 32],
            successor,
            phase: FleetAdmissionCoordinatorTransitionPhaseRecord::Opening,
            roots: roots.clone(),
        }),
        last_result: Some(FleetAdmissionRetainedResultRecord {
            request,
            request_hash: [0xfc; 32],
            response: FleetAdmissionMutationResponseRecord {
                outcome: FleetAdmissionMutationOutcomeRecord::Converged,
                operation_id: [0xfe; 32],
                generation: u64::MAX - 1,
                policy_digest: [0xfb; 32],
            },
            roots,
        }),
    };

    let bytes = record.to_bytes().expect("encode current record");
    eprintln!(
        "maximum Coordinator admission current-plus-last encoded bytes: {}",
        bytes.len()
    );
    assert!(bytes.len() <= MAX_FLEET_ADMISSION_AUTHORITY_RECORD_BYTES as usize);
    assert_cell_allocation(&record, bytes.len() as u64);
}

fn assert_cell_allocation(record: &FleetAdmissionAuthorityRecord, encoded_bytes: u64) {
    let physical = VectorMemory::default();
    let runtime = ic_storage::testing::fixture_runtime(
        physical.clone(),
        "test",
        "test.",
        &["test.admission.v1"],
        ic_storage::StorageConfig::default(),
    )
    .unwrap();
    let mut cell = {
        let allocation = runtime
            .open_memory(&ic_storage::StableKey::parse("test.admission.v1").unwrap())
            .unwrap();
        if canic_core::cdk::structures::Memory::size(&allocation) == 0 {
            StableCell::create(allocation, None).expect("Storage operation failed")
        } else {
            StableCell::load(
                allocation,
                canic_contracts::serialization::STORAGE_LIMITS.bytes,
            )
            .expect("retained stable cell")
        }
    };
    cell.set(Some(record.clone()))
        .expect("Storage operation failed");
    let pages = runtime
        .memory_allocations()
        .unwrap()
        .iter()
        .find(|entry| entry.stable_key.as_str() == "test.admission.v1")
        .unwrap()
        .pages;
    assert_eq!(
        pages,
        (encoded_bytes + ic_storage::STABLE_CELL_HEADER_SIZE as u64).div_ceil(65_536)
    );
    eprintln!(
        "Coordinator admission cell: virtual_bytes={} physical_bytes={}",
        pages * 65_536,
        physical.size() * 65_536
    );
    drop(cell);
    assert_eq!(
        {
            let allocation = runtime
                .open_memory(&ic_storage::StableKey::parse("test.admission.v1").unwrap())
                .unwrap();
            if canic_core::cdk::structures::Memory::size(&allocation) == 0 {
                StableCell::create(allocation, None).expect("Storage operation failed")
            } else {
                StableCell::load(
                    allocation,
                    canic_contracts::serialization::STORAGE_LIMITS.bytes,
                )
                .expect("retained stable cell")
            }
        }
        .get(),
        &Some(record.clone())
    );
}

fn principal(index: u16) -> Principal {
    Principal::from_slice(&index.to_be_bytes())
}
