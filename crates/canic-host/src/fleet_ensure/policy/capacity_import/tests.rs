//! Admission failures preserve exact identities and original cycle allowances.

use super::*;
use crate::fleet_ensure::{
    model::capacity_import::{CapacityImportSourceBinding, CapacityImportSourceRecord},
    ops::capacity_import::prepare_review,
};
use canic_contracts::ids::{AppId, CanonicalNetworkId, FleetBinding, FleetId, FleetKey};

pub fn principal(byte: u8) -> Principal {
    Principal::from_slice(&[byte; 29])
}

pub fn plan() -> CapacityImportPlanRecord {
    let authority = CapacityImportAuthority {
        fleet: FleetBinding {
            app: AppId::from("toko"),
            fleet: FleetKey {
                canonical_network_id: CanonicalNetworkId::ic_mainnet(),
                fleet_id: FleetId::from_generated_bytes([1; 32]),
            },
        },
        network_root_key_sha256: [2; 32],
        operator: principal(3),
        coordinator: principal(4),
        root: principal(5),
        subnet: SubnetId::from_principal(principal(6)),
        root_authority_sha256: [7; 32],
        import_sequence: 0,
        recovery_controllers: vec![principal(8)],
    };
    let source = CapacityImportSourceRecord {
        binding: CapacityImportSourceBinding {
            canister_id: principal(9),
            subnet: authority.subnet,
            controllers: vec![principal(3), principal(10)],
            module_sha256: Some([11; 32]),
            canister_version: 42,
            stopped: true,
            snapshots_size_bytes: 0,
        },
        disposition: CapacityImportDisposition::Retired {
            evidence_sha256: [12; 32],
        },
        observed_cycles: 1_000,
        observed_reserved_cycles: 100,
        minimum_ready_cycles: 800,
        maximum_debit_cycles: 200,
    };
    prepare_review(authority, vec![source], root_budget()).unwrap()
}

pub fn destination(plan: &CapacityImportPlanRecord) -> CapacityImportDestinationView {
    CapacityImportDestinationView {
        authority: plan.authority.clone(),
        ready: true,
        draining: false,
        competing_operation: false,
        occupied_capacity: 3,
        maximum_capacity: 4,
        controlled_cycles: 1_000,
        reserved_cycles: 100,
        minimum_retained_cycles: 900,
        assigned_canisters: [plan.authority.root, plan.authority.coordinator].into(),
    }
}

#[test]
fn current_root_custody_accepts_running_sources_without_an_operator_handoff() {
    let mut plan = plan();
    plan.sources[0].binding.controllers = vec![plan.authority.root];
    plan.sources[0].binding.stopped = false;
    validate_plan(&plan).unwrap();
    assert!(!requires_handoff(&plan, &plan.sources[0]));
    assert_eq!(controller_version_delta(&plan, &plan.sources[0]), 1);
    let mut observed = sources(&plan);
    observed[0].binding.canister_version += 10;
    admit_handoffs(&plan, &destination(&plan), &observed).unwrap();
    observed[0].binding.controllers = vec![principal(99)];
    assert!(matches!(
        admit_handoffs(&plan, &destination(&plan), &observed),
        Err(CapacityImportPolicyError::SourceChanged { .. })
    ));
    plan.sources[0].binding.controllers = vec![principal(99)];
    plan.sources[0].binding.stopped = true;
    assert!(matches!(
        validate_plan(&plan),
        Err(CapacityImportPolicyError::OperatorNotController { .. })
    ));
}

pub fn sources(plan: &CapacityImportPlanRecord) -> Vec<CapacityImportSourceView> {
    plan.sources
        .iter()
        .map(|source| CapacityImportSourceView {
            binding: source.binding.clone(),
            cycles: source.observed_cycles,
            reserved_cycles: source.observed_reserved_cycles,
            ownership: CapacityImportOwnershipView::Unassigned,
            disposition_evidence_sha256: Some(disposition_digest(source)),
        })
        .collect()
}

#[test]
fn exact_import_drops_previous_owner_and_retains_only_reviewed_recovery_controllers() {
    let plan = plan();
    assert_eq!(
        plan.transitional_controllers,
        vec![principal(3), principal(5), principal(8)]
    );
    assert_eq!(plan.final_controllers, vec![principal(5), principal(8)]);
    assert_eq!(
        admit_handoffs(&plan, &destination(&plan), &sources(&plan)),
        Ok(())
    );
}

#[test]
fn missing_authority_and_duplicate_sources_reject() {
    let mut plan = plan();
    plan.sources[0].binding.controllers = vec![principal(10)];
    assert_eq!(
        validate_plan(&plan),
        Err(CapacityImportPolicyError::OperatorNotController {
            canister: principal(9)
        })
    );
    plan.sources[0].binding.controllers.insert(0, principal(3));
    plan.sources.push(plan.sources[0].clone());
    assert_eq!(
        validate_plan(&plan),
        Err(CapacityImportPolicyError::InvalidSources)
    );
}

#[test]
fn controller_limits_apply_to_the_transitional_set_too() {
    let plan = plan();
    let mut authority = plan.authority.clone();
    authority.recovery_controllers = (20..29).map(principal).collect();
    assert!(matches!(
        prepare_review(authority, plan.sources.clone(), root_budget()),
        Err(
            crate::fleet_ensure::ops::capacity_import::CapacityImportReviewError::Policy(
                CapacityImportPolicyError::InvalidAuthority
            )
        )
    ));
    let mut authority = plan.authority.clone();
    authority.recovery_controllers = vec![authority.operator];
    let reviewed = prepare_review(authority, plan.sources, root_budget()).unwrap();
    assert_eq!(reviewed.final_controllers, vec![principal(3), principal(5)]);
    assert_eq!(
        reviewed.transitional_controllers,
        reviewed.final_controllers
    );
}

#[test]
fn every_management_binding_change_rejects_before_handoff() {
    let plan = plan();
    let changes: [fn(&mut CapacityImportSourceBinding); 5] = [
        |binding| binding.canister_id = principal(20),
        |binding| binding.subnet = SubnetId::from_principal(principal(20)),
        |binding| binding.controllers.push(principal(20)),
        |binding| binding.module_sha256 = None,
        |binding| binding.canister_version += 1,
    ];
    for change in changes {
        let mut observed = sources(&plan);
        change(&mut observed[0].binding);
        assert_eq!(
            admit_handoffs(&plan, &destination(&plan), &observed),
            Err(CapacityImportPolicyError::SourceChanged {
                canister: observed[0].binding.canister_id
            })
        );
    }
}

#[test]
fn missing_and_duplicate_observations_do_not_establish_complete_coverage() {
    let plan = plan();
    assert_eq!(
        admit_handoffs(&plan, &destination(&plan), &[]),
        Err(CapacityImportPolicyError::InvalidSources)
    );
    let observed = sources(&plan);
    assert_eq!(
        admit_handoffs(
            &plan,
            &destination(&plan),
            &[observed[0].clone(), observed[0].clone()]
        ),
        Err(CapacityImportPolicyError::InvalidSources)
    );
}

#[test]
fn wrong_subnet_and_known_infrastructure_reject_without_replacement() {
    let mut plan = plan();
    plan.sources[0].binding.subnet = SubnetId::from_principal(principal(20));
    assert_eq!(
        validate_plan(&plan),
        Err(CapacityImportPolicyError::WrongSubnet {
            canister: principal(9)
        })
    );
    plan.sources[0].binding.subnet = plan.authority.subnet;
    plan.sources[0].binding.canister_id = plan.authority.coordinator;
    assert_eq!(
        validate_plan(&plan),
        Err(CapacityImportPolicyError::SourceAssigned {
            canister: principal(4)
        })
    );
}

#[test]
fn assigned_unknown_and_unretired_candidates_stay_blocked() {
    let plan = plan();
    for ownership in [
        CapacityImportOwnershipView::Assigned,
        CapacityImportOwnershipView::Unknown,
    ] {
        let mut observed = sources(&plan);
        observed[0].ownership = ownership;
        assert_eq!(
            admit_handoffs(&plan, &destination(&plan), &observed),
            Err(CapacityImportPolicyError::SourceAssigned {
                canister: principal(9)
            })
        );
    }
    for evidence in [None, Some([0; 32]), Some([99; 32])] {
        let mut observed = sources(&plan);
        observed[0].disposition_evidence_sha256 = evidence;
        assert_eq!(
            admit_handoffs(&plan, &destination(&plan), &observed),
            Err(CapacityImportPolicyError::DispositionUnresolved {
                canister: principal(9)
            })
        );
    }
    let mut root = destination(&plan);
    root.assigned_canisters.insert(principal(9));
    assert_eq!(
        admit_handoffs(&plan, &root, &sources(&plan)),
        Err(CapacityImportPolicyError::SourceAssigned {
            canister: principal(9)
        })
    );
}

#[test]
fn destination_must_remain_ready_uncontested_and_with_exact_authority() {
    let plan = plan();
    for change in [
        |root: &mut CapacityImportDestinationView| root.ready = false,
        |root: &mut CapacityImportDestinationView| root.draining = true,
        |root: &mut CapacityImportDestinationView| root.competing_operation = true,
    ] {
        let mut root = destination(&plan);
        change(&mut root);
        assert_eq!(
            admit_handoffs(&plan, &root, &sources(&plan)),
            Err(CapacityImportPolicyError::DestinationUnavailable)
        );
    }
    let mut root = destination(&plan);
    root.authority.root_authority_sha256 = [99; 32];
    assert_eq!(
        admit_handoffs(&plan, &root, &sources(&plan)),
        Err(CapacityImportPolicyError::DestinationChanged)
    );
}

#[test]
fn physical_capacity_includes_pending_creation_reservations() {
    let plan = plan();
    for occupied in [4, u32::MAX] {
        let mut root = destination(&plan);
        root.occupied_capacity = occupied;
        assert_eq!(
            admit_handoffs(&plan, &root, &sources(&plan)),
            Err(CapacityImportPolicyError::CapacityExceeded)
        );
    }
}

#[test]
fn both_source_and_root_require_full_reviewed_headroom() {
    let mut plan = plan();
    plan.sources[0].observed_cycles -= 1;
    assert_eq!(
        validate_plan(&plan),
        Err(CapacityImportPolicyError::InsufficientCycles {
            canister: principal(9),
            required_cycles: 1_000,
            available_cycles: 999,
            shortfall_cycles: 1,
        })
    );
    plan.sources[0].observed_cycles += 1;
    let mut root = destination(&plan);
    root.controlled_cycles -= 1;
    assert!(admit_handoffs(&plan, &root, &sources(&plan)).is_ok());
    root.controlled_cycles = 899;
    assert_eq!(
        admit_handoffs(&plan, &root, &sources(&plan)),
        Err(CapacityImportPolicyError::InvalidCycleBounds)
    );
    root.controlled_cycles = 1_001;
    admit_handoffs(&plan, &root, &sources(&plan)).unwrap();
    plan.sources[0].maximum_debit_cycles = u128::MAX;
    assert_eq!(
        validate_plan(&plan),
        Err(CapacityImportPolicyError::InvalidCycleBounds)
    );
}

#[test]
fn retries_keep_original_cycle_baseline_with_native_credits() {
    let plan = plan();
    let source = &plan.sources[0];
    assert_eq!(retained_source_debit(source, 800, 100), Ok(200));
    assert_eq!(retained_source_debit(source, 1_001, 100), Ok(0));
    assert_eq!(
        retained_source_debit(source, 799, 100),
        Err(CapacityImportPolicyError::ConservationUnproven {
            canister: principal(9)
        })
    );
    let encoded = serde_json::to_vec(&plan).unwrap();
    let resumed: CapacityImportPlanRecord = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(
        retained_source_debit(&resumed.sources[0], 799, 100),
        Err(CapacityImportPolicyError::ConservationUnproven {
            canister: principal(9)
        })
    );
}

#[test]
fn ambiguous_or_unavailable_root_never_selects_a_replacement() {
    let plan = plan();
    let first = CapacityImportRootView {
        root: plan.authority.root,
        subnet: plan.authority.subnet,
    };
    let mut second = first;
    second.root = principal(20);
    let roots = [first, second];
    assert_eq!(
        select_destination(&roots, plan.authority.subnet, None),
        Err(CapacityImportPolicyError::DestinationAmbiguous {
            subnet: plan.authority.subnet
        })
    );
    assert_eq!(
        select_destination(&roots, plan.authority.subnet, Some(principal(5)))
            .unwrap()
            .root,
        principal(5)
    );
    assert_eq!(
        select_destination(&roots, plan.authority.subnet, Some(principal(99))),
        Err(CapacityImportPolicyError::DestinationMissing {
            subnet: plan.authority.subnet
        })
    );
}

pub fn root_budget() -> crate::fleet_ensure::model::capacity_import::CapacityImportRootBudget {
    crate::fleet_ensure::model::capacity_import::CapacityImportRootBudget {
        observed_reserved_cycles: 100,
        observed_cycles: 1_000,
        minimum_retained_cycles: 900,
        maximum_debit_cycles: 100,
        maximum_paid_calls: 100,
    }
}

#[test]
fn capacity_import_requires_installed_sources_to_be_quiescent() {
    let mut plan = plan();
    plan.sources[0].binding.stopped = false;
    assert_eq!(
        validate_plan(&plan),
        Err(CapacityImportPolicyError::SourceRunning {
            canister: principal(9)
        })
    );
    plan.sources[0].binding.module_sha256 = None;
    assert!(validate_plan(&plan).is_ok());
}

#[test]
fn capacity_import_accounts_reserved_cycles_without_spending_the_liquid_floor() {
    let plan = plan();
    let source = &plan.sources[0];
    assert_eq!(retained_source_debit(source, 950, 150), Ok(0));
    assert_eq!(retained_source_debit(source, 950, 0), Ok(150));
    assert_eq!(retained_source_debit(source, 1_000, 101), Ok(0));
    assert!(matches!(
        retained_source_debit(source, 799, 301),
        Err(CapacityImportPolicyError::InsufficientCycles { .. })
    ));
    assert_eq!(
        source_total(u128::MAX, 1),
        Err(CapacityImportPolicyError::InvalidCycleBounds)
    );
}

#[test]
fn capacity_import_requires_snapshot_retirement_before_review() {
    let mut plan = plan();
    plan.sources[0].binding.snapshots_size_bytes = 1;
    assert_eq!(
        validate_plan(&plan),
        Err(CapacityImportPolicyError::SourceSnapshots {
            canister: plan.sources[0].binding.canister_id,
        })
    );
}
