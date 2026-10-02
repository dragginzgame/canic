//! Release review binds whole-Fleet custody without treating a digest as live authority.

use super::*;
use crate::fleet_ensure::{
    model::{
        capacity_import::{CapacityImportAuthority, CapacityImportSourceBinding},
        release::{FleetReleaseAccountRecord, FleetReleaseAuthority, FleetReleaseSourceRecord},
    },
    ops::release::{prepare_review, verify_held_capacity, verify_reset_custody, verify_review},
    view::{
        capacity_import::CapacityImportDestinationView,
        release::{FleetReleaseOwnerView, FleetReleaseSourceView},
    },
};
use canic_core::ids::{AppId, CanonicalNetworkId, FleetBinding, FleetId, FleetKey, SubnetId};

fn id(byte: u8) -> Principal {
    Principal::from_slice(&[byte; 29])
}

pub(in crate::fleet_ensure) fn fixture() -> (FleetReleaseReviewRecord, FleetReleaseObservation) {
    let authority = FleetReleaseAuthority {
        fleet: FleetBinding {
            app: AppId::from("release-fixture"),
            fleet: FleetKey {
                canonical_network_id: CanonicalNetworkId::ic_mainnet(),
                fleet_id: FleetId::from_generated_bytes([1; 32]),
            },
        },
        network_root_key_sha256: [2; 32],
        release_build_sha256: [3; 32],
        operation_id: [4; 32],
        operator: id(10),
        coordinator: id(1),
        cycles_ledger: id(30),
    };
    let sources = [
        (1, FleetReleaseRole::Coordinator, 20, id(10)),
        (2, FleetReleaseRole::Root, 21, id(10)),
        (3, FleetReleaseRole::Store { root: id(2) }, 21, id(2)),
        (4, FleetReleaseRole::Child { root: id(2) }, 21, id(2)),
    ]
    .into_iter()
    .map(
        |(canister, role, subnet, controller)| FleetReleaseSourceRecord {
            binding: CapacityImportSourceBinding {
                canister_id: id(canister),
                subnet: SubnetId::from_principal(id(subnet)),
                controllers: vec![controller],
                module_sha256: Some([5; 32]),
                canister_version: 8,
                stopped: true,
                snapshots_size_bytes: 100,
            },
            snapshots: vec![vec![1; 32]],
            role,
            destination: FleetReleaseDestination::OperatorHeld,
            disposition_sha256: [6; 32],
            observed_cycles: 1_000,
            observed_reserved_cycles: 500,
            minimum_retained_cycles: 800,
            maximum_debit_cycles: 200,
            maximum_call_debit_cycles: 10,
            maximum_paid_calls: 20,
        },
    )
    .collect::<Vec<_>>();
    let accounts = [id(1), id(2)]
        .into_iter()
        .map(|owner| FleetReleaseAccountRecord {
            ledger: id(30),
            owner,
            subaccount: None,
            recovery_artifact_sha256: [7; 32],
            observed_balance: 10_000,
        })
        .collect::<Vec<_>>();
    let observed = FleetReleaseObservation {
        authority: authority.clone(),
        sources: sources
            .iter()
            .map(|source| FleetReleaseSourceView {
                binding: source.binding.clone(),
                snapshots: source.snapshots.clone(),
                cycles: source.observed_cycles,
                reserved_cycles: source.observed_reserved_cycles,
                maximum_call_debit_cycles: 10,
                required_paid_calls: 10,
            })
            .collect(),
        owners: vec![
            FleetReleaseOwnerView {
                owner: id(1),
                children: vec![id(2)],
                producers_quiescent: true,
                unresolved_operations: vec![],
            },
            FleetReleaseOwnerView {
                owner: id(2),
                children: vec![id(3), id(4)],
                producers_quiescent: true,
                unresolved_operations: vec![],
            },
        ],
        accounts: accounts.clone(),
        destinations: vec![],
    };
    (
        FleetReleaseReviewRecord {
            schema_version: 1,
            authority,
            sources,
            accounts,
            review_sha256: [0; 32],
        },
        observed,
    )
}

fn destination(review: &FleetReleaseReviewRecord) -> CapacityImportDestinationView {
    let mut fleet = review.authority.fleet.clone();
    fleet.fleet.fleet_id = FleetId::from_generated_bytes([9; 32]);
    CapacityImportDestinationView {
        authority: CapacityImportAuthority {
            fleet,
            network_root_key_sha256: review.authority.network_root_key_sha256,
            operator: review.authority.operator,
            coordinator: id(40),
            root: id(41),
            subnet: SubnetId::from_principal(id(21)),
            root_authority_sha256: [8; 32],
            import_sequence: 0,
            recovery_controllers: vec![],
        },
        ready: true,
        draining: false,
        competing_operation: false,
        occupied_capacity: 3,
        maximum_capacity: 5,
        controlled_cycles: 10_000,
        reserved_cycles: 0,
        minimum_retained_cycles: 1_000,
        assigned_canisters: [id(40), id(41)].into(),
    }
}

#[test]
fn all_infrastructure_roles_and_children_bind_one_effect_free_review() {
    let (review, observed) = fixture();
    let sealed = prepare_review(review.clone(), &observed).unwrap();
    assert_ne!(sealed.review_sha256, [0; 32]);
    assert_eq!(prepare_review(review, &observed).unwrap(), sealed);
    verify_review(&sealed, &observed).unwrap();
    let bytes = serde_json::to_vec(&sealed).unwrap();
    let decoded = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(sealed, decoded);
}

#[test]
fn changed_bounds_or_current_authority_cannot_reuse_a_review() {
    let (review, observed) = fixture();
    let sealed = prepare_review(review, &observed).unwrap();
    let mut changed = sealed.clone();
    changed.sources[0].maximum_paid_calls += 1;
    assert_eq!(
        verify_review(&changed, &observed),
        Err(FleetReleaseError::Authority)
    );
    let mut changed = observed.clone();
    changed.authority.release_build_sha256 = [99; 32];
    assert_eq!(
        verify_review(&sealed, &changed),
        Err(FleetReleaseError::Authority)
    );
    changed = observed;
    changed.sources[2].binding.controllers = vec![id(99)];
    assert_eq!(
        verify_review(&sealed, &changed),
        Err(FleetReleaseError::Custody { canister: id(3) })
    );
}

#[test]
fn incomplete_duplicate_and_wrong_subnet_inventory_rejects() {
    let (review, observed) = fixture();
    let mut omitted = review.clone();
    omitted.sources.pop();
    let mut observations = observed.clone();
    observations.sources.pop();
    assert_eq!(
        validate_review(&omitted, &observations),
        Err(FleetReleaseError::Inventory)
    );
    let mut changed = review.clone();
    changed.sources.push(changed.sources[0].clone());
    assert_eq!(
        validate_review(&changed, &observed),
        Err(FleetReleaseError::Inventory)
    );
    changed = review;
    changed.sources[2].binding.subnet = SubnetId::from_principal(id(20));
    assert_eq!(
        validate_review(&changed, &observed),
        Err(FleetReleaseError::Inventory)
    );
    observations = observed;
    observations.owners[1].children.push(id(4));
    assert_eq!(
        validate_review(&fixture().0, &observations),
        Err(FleetReleaseError::Inventory)
    );
}

#[test]
fn unfinished_effects_and_active_producers_keep_their_owner() {
    let (review, observed) = fixture();
    for owner in 0..observed.owners.len() {
        let mut changed = observed.clone();
        changed.owners[owner].unresolved_operations.push([42; 32]);
        let expected = Err(FleetReleaseError::Unfinished {
            owner: changed.owners[owner].owner,
        });
        assert_eq!(validate_review(&review, &changed), expected);
        changed.owners[owner].unresolved_operations.clear();
        changed.owners[owner].producers_quiescent = false;
        assert_eq!(validate_review(&review, &changed), expected);
    }
}

#[test]
fn reserved_cycles_cannot_fund_calls_and_overflow_never_grants_headroom() {
    let (review, observed) = fixture();
    for debit in [201, u128::MAX] {
        let mut changed = review.clone();
        changed.sources[0].maximum_debit_cycles = debit;
        assert_eq!(
            validate_review(&changed, &observed),
            Err(FleetReleaseError::Budget { canister: id(1) })
        );
    }
    let mut changed = review.clone();
    changed.sources[0].maximum_paid_calls = 9;
    assert_eq!(
        validate_review(&changed, &observed),
        Err(FleetReleaseError::Budget { canister: id(1) })
    );
    let mut observations = observed;
    observations.sources[0].maximum_call_debit_cycles = u128::MAX;
    assert_eq!(
        validate_review(&review, &observations),
        Err(FleetReleaseError::Budget { canister: id(1) })
    );
}

#[test]
fn pool_admission_requires_independent_live_authority_and_aggregate_room() {
    let (mut review, mut observed) = fixture();
    for source in &mut review.sources[2..] {
        source.destination = FleetReleaseDestination::IndependentPool { root: id(41) };
    }
    observed.destinations.push(destination(&review));
    validate_review(&review, &observed).unwrap();
    observed.destinations[0].maximum_capacity = 4;
    assert_eq!(
        validate_review(&review, &observed),
        Err(FleetReleaseError::Capacity { root: id(41) })
    );
    observed.destinations[0].maximum_capacity = 5;
    observed.destinations[0].authority.coordinator = id(1);
    assert_eq!(
        validate_review(&review, &observed),
        Err(FleetReleaseError::Destination { canister: id(3) })
    );
    observed.destinations[0] = destination(&review);
    review.sources[0].destination = FleetReleaseDestination::IndependentPool { root: id(41) };
    assert_eq!(
        validate_review(&review, &observed),
        Err(FleetReleaseError::Accounts)
    );
    review.sources[0].destination = FleetReleaseDestination::OperatorHeld;
    observed.destinations[0].authority.subnet = SubnetId::from_principal(id(20));
    assert_eq!(
        validate_review(&review, &observed),
        Err(FleetReleaseError::Destination { canister: id(3) })
    );
}

#[test]
fn external_accounts_require_exact_inventory_and_recoverable_operator_custody() {
    let (review, observed) = fixture();
    let mut changed = review.clone();
    changed.accounts.pop();
    assert_eq!(
        validate_review(&changed, &observed),
        Err(FleetReleaseError::Accounts)
    );
    changed.accounts.clear();
    let mut missing = observed.clone();
    missing.accounts.clear();
    assert_eq!(
        validate_review(&changed, &missing),
        Err(FleetReleaseError::Accounts)
    );
    changed = review.clone();
    changed.accounts[0].recovery_artifact_sha256 = [0; 32];
    assert_eq!(
        validate_review(&changed, &observed),
        Err(FleetReleaseError::Accounts)
    );
    let mut observations = observed;
    observations.accounts[0].observed_balance -= 1;
    assert_eq!(
        validate_review(&review, &observations),
        Err(FleetReleaseError::Accounts)
    );
}

#[test]
fn default_account_representations_share_one_identity_and_review_digest() {
    let (review, observed) = fixture();
    let sealed = prepare_review(review.clone(), &observed).unwrap();
    let mut explicit = review;
    explicit.accounts[0].subaccount = Some([0; 32]);
    assert_eq!(prepare_review(explicit.clone(), &observed).unwrap(), sealed);
    let mut observed_explicit = observed.clone();
    observed_explicit.accounts[1].subaccount = Some([0; 32]);
    verify_review(&sealed, &observed_explicit).unwrap();

    explicit.accounts.push(sealed.accounts[0].clone());
    assert_eq!(
        validate_review(&explicit, &observed),
        Err(FleetReleaseError::Accounts)
    );
    observed_explicit
        .accounts
        .push(observed.accounts[1].clone());
    assert_eq!(
        validate_review(&sealed, &observed_explicit),
        Err(FleetReleaseError::Accounts)
    );
}

#[test]
fn empty_retained_ids_need_exact_controllers_snapshots_and_conservation() {
    let (review, mut observed) = fixture();
    let review = prepare_review(review, &observed).unwrap();
    for source in &mut observed.sources {
        source.binding.controllers = vec![review.authority.operator];
        source.binding.module_sha256 = None;
        source.binding.snapshots_size_bytes = 0;
        source.snapshots.clear();
        source.cycles -= 50;
    }
    verify_held_capacity(&review, &observed).unwrap();
    for index in 0..observed.sources.len() {
        let id = observed.sources[index].binding.canister_id;
        let mut changed = observed.clone();
        changed.sources[index]
            .binding
            .controllers
            .push(review.authority.coordinator);
        assert_eq!(
            validate_held_capacity(&review, &changed),
            Err(FleetReleaseError::Custody { canister: id })
        );
        changed = observed.clone();
        changed.sources[index].binding.snapshots_size_bytes = 1;
        assert_eq!(
            validate_held_capacity(&review, &changed),
            Err(FleetReleaseError::Custody { canister: id })
        );
        changed = observed.clone();
        changed.sources[index].snapshots.push(vec![1; 32]);
        assert_eq!(
            verify_held_capacity(&review, &changed),
            Err(FleetReleaseError::Custody { canister: id })
        );
        changed = observed.clone();
        changed.sources[index].reserved_cycles = 0;
        assert_eq!(
            validate_held_capacity(&review, &changed),
            Err(FleetReleaseError::Conservation { canister: id })
        );
    }
}

#[test]
fn child_custody_precedes_clearing_any_infrastructure_owner() {
    let (review, mut observed) = fixture();
    let review = prepare_review(review, &observed).unwrap();
    assert_eq!(
        verify_reset_custody(&review, &observed),
        Err(FleetReleaseError::Custody { canister: id(3) })
    );
    for source in &mut observed.sources {
        source.binding.controllers = vec![review.authority.operator];
        source.binding.canister_version += 1;
        source.cycles -= 10;
    }
    verify_reset_custody(&review, &observed).unwrap();
    observed.sources[0].binding.module_sha256 = None;
    assert_eq!(
        verify_reset_custody(&review, &observed),
        Err(FleetReleaseError::Custody { canister: id(1) })
    );
}

#[test]
fn snapshot_id_drift_and_forged_terminal_baselines_reject() {
    let (review, mut observed) = fixture();
    let mut review = prepare_review(review, &observed).unwrap();
    observed.sources[2].snapshots = vec![vec![2; 32]];
    assert_eq!(
        verify_review(&review, &observed),
        Err(FleetReleaseError::Custody { canister: id(3) })
    );
    review.sources[0].observed_cycles += 1;
    assert_eq!(
        verify_reset_custody(&review, &observed),
        Err(FleetReleaseError::Authority)
    );
    assert_eq!(
        verify_held_capacity(&review, &observed),
        Err(FleetReleaseError::Authority)
    );
}

#[test]
fn native_credits_preserve_reviewed_authority_at_both_capacity_boundaries() {
    let (review, observed) = fixture();
    let review = prepare_review(review, &observed).unwrap();
    let original = serde_json::to_vec(&review).unwrap();
    let reopened: FleetReleaseReviewRecord = serde_json::from_slice(&original).unwrap();
    for index in 0..observed.sources.len() {
        for credit in [1, review.sources[index].maximum_debit_cycles + 1] {
            for reserved_credit in [false, true] {
                let mut credited = observed.clone();
                for source in &mut credited.sources {
                    source.binding.controllers = vec![review.authority.operator];
                }
                let source = &mut credited.sources[index];
                if reserved_credit {
                    source.reserved_cycles += credit;
                } else {
                    source.cycles += credit;
                }
                verify_reset_custody(&reopened, &credited).unwrap();
                for source in &mut credited.sources {
                    source.binding.module_sha256 = None;
                    source.binding.snapshots_size_bytes = 0;
                    source.snapshots.clear();
                }
                verify_held_capacity(&reopened, &credited).unwrap();
                credited.sources[index]
                    .binding
                    .controllers
                    .push(Principal::anonymous());
                assert_eq!(
                    verify_held_capacity(&reopened, &credited),
                    Err(FleetReleaseError::Custody {
                        canister: review.sources[index].binding.canister_id
                    })
                );
            }
        }
    }
    assert_eq!(serde_json::to_vec(&reopened).unwrap(), original);
}

#[test]
fn capacity_credits_do_not_bypass_native_floors_debit_caps_or_checked_totals() {
    let (review, mut observed) = fixture();
    let review = prepare_review(review, &observed).unwrap();
    for source in &mut observed.sources {
        source.binding.controllers = vec![review.authority.operator];
    }
    for empty in [false, true] {
        let mut baseline = observed.clone();
        if empty {
            for source in &mut baseline.sources {
                source.binding.module_sha256 = None;
                source.binding.snapshots_size_bytes = 0;
                source.snapshots.clear();
            }
        }
        let verify = if empty {
            verify_held_capacity
        } else {
            verify_reset_custody
        };
        for (index, source) in review.sources.iter().enumerate() {
            let mut depleted = baseline.clone();
            depleted.sources[index].reserved_cycles -= source.maximum_debit_cycles;
            verify(&review, &depleted).unwrap();
            depleted.sources[index].reserved_cycles -= 1;
            assert_eq!(
                verify(&review, &depleted),
                Err(FleetReleaseError::Conservation {
                    canister: source.binding.canister_id
                })
            );
            let mut below_floor = baseline.clone();
            below_floor.sources[index].cycles = source.minimum_retained_cycles - 1;
            below_floor.sources[index].reserved_cycles += source.observed_cycles;
            assert_eq!(
                verify(&review, &below_floor),
                Err(FleetReleaseError::Conservation {
                    canister: source.binding.canister_id
                })
            );
            let mut overflow = baseline.clone();
            overflow.sources[index].cycles = u128::MAX;
            assert_eq!(
                verify(&review, &overflow),
                Err(FleetReleaseError::Conservation {
                    canister: source.binding.canister_id
                })
            );
        }
    }
}
