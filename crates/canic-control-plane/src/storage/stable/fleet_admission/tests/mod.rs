#![cfg(test)]

//! Coordinator admission encoding and allocation qualification.

use super::*;
use canic_core::{
    cdk::{
        bounded_cell::BoundedCell,
        structures::{
            Memory, VectorMemory,
            memory::{MemoryId, MemoryManager},
            storable::Storable,
        },
    },
    ids::{
        AppId, CanonicalNetworkId, FleetAdmissionRule, FleetBinding, FleetId, FleetKey, SubnetId,
    },
    shared_support::fleet_admission_policy::compile_installed_fleet_admission_policy,
};

#[test]
fn maximum_b3_current_plus_last_authority_fits_memory_id_64() {
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

    let bytes = record.to_bytes();
    eprintln!(
        "maximum Coordinator admission current-plus-last encoded bytes: {}",
        bytes.len()
    );
    assert!(bytes.len() <= MAX_FLEET_ADMISSION_AUTHORITY_RECORD_BYTES as usize);
    assert_cell_allocation(&record, bytes.len() as u64);
}

fn assert_cell_allocation(record: &FleetAdmissionAuthorityRecord, encoded_bytes: u64) {
    let physical = VectorMemory::default();
    let manager = MemoryManager::init_with_bucket_size(physical.clone(), 16);
    let memory = manager.get(MemoryId::new(64));
    let mut cell = BoundedCell::init(memory.clone(), None);
    cell.set(Some(record.clone()));
    assert_eq!(memory.size(), (encoded_bytes + 9).div_ceil(65_536));
    eprintln!(
        "Coordinator admission cell: virtual_bytes={} physical_bytes={}",
        memory.size() * 65_536,
        physical.size() * 65_536
    );
    drop(cell);
    assert_eq!(BoundedCell::init(memory, None).get(), &Some(record.clone()));
}

fn principal(index: u16) -> Principal {
    Principal::from_slice(&index.to_be_bytes())
}
