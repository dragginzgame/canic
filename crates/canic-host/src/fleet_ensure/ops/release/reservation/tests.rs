//! Disk recovery, finite read authority and uncertain persistence under the Fleet lock.

use super::*;
use crate::fleet_ensure::{
    ops::{lock_operation, release::prepare_review},
    policy::release::tests::fixture,
};
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

pub(in crate::fleet_ensure) fn seed(paths: &EnsurePaths, review: &FleetReleaseReviewRecord) {
    let journal = FleetEnsureJournalRecord {
        release: None,
        bootstrap_registration_recovery: None,
        funding_observations: BTreeMap::new(),
        funding_reviews: vec![],
        successor_phases: vec![],
        completion: FleetEnsureCompletion::InProgress,
        estate_funding_required: None,
        effects: vec![],
        fleet: "release".into(),
        initial_controlled_cycles: review
            .sources
            .iter()
            .map(|source| source.observed_cycles)
            .sum(),
        initial_estate_funding_cycles_by_root: BTreeMap::new(),
        initial_operator_cycles: 0,
        operation_id: hex_bytes(review.authority.operation_id),
        plan_sha256: hex_bytes([19; 32]),
        schema_version: 1,
        stalled_observations: 0,
    };
    write_journal(paths, &journal).unwrap();
}

#[test]
fn lost_read_results_remain_spent_after_reopening_the_same_operation() {
    let directory = crate::test_support::temp_dir("release-reservation-recovery");
    let paths = EnsurePaths::under(&directory, "local", "release");
    let (review, observed) = fixture();
    let review = prepare_review(review, &observed).unwrap();
    let id = review.sources[0].binding.canister_id;
    seed(&paths, &review);
    let mut owner = ReleaseObservationJournal::attach(&paths, &review, &observed).unwrap();
    {
        let reservation = owner.reserve(id).unwrap();
        let subnet = review.sources[0].binding.subnet;
        assert!(reservation.authorizes(&review.authority, id, subnet));
        assert!(!reservation.authorizes(&review.authority, id, review.sources[1].binding.subnet));
        assert!(!reservation.authorizes(
            &review.authority,
            review.sources[1].binding.canister_id,
            subnet
        ));
        let wrong = FleetReleaseAuthority {
            operation_id: [9; 32],
            ..review.authority.clone()
        };
        assert!(!reservation.authorizes(&wrong, id, subnet));
    }
    assert_eq!(
        read_journal(&paths)
            .unwrap()
            .unwrap()
            .release
            .unwrap()
            .reserved_paid_calls[&id.to_text()],
        4
    );
    drop(owner);
    let mut resumed = ReleaseObservationJournal::resume(&paths, &review).unwrap();
    for _ in 0..4 {
        let _ = resumed.reserve(id).unwrap();
    }
    let retained = fs::read(&paths.journal).unwrap();
    assert!(matches!(
        resumed.reserve(id),
        Err(ReleaseReservationError::Evidence(
            FleetReleaseError::ObservationBudget {
                requested_calls: 4,
                remaining_calls: 0,
                required_debit_cycles: 40,
                remaining_debit_cycles: 0,
                ..
            }
        ))
    ));
    assert_eq!(fs::read(&paths.journal).unwrap(), retained);
    drop(resumed);
    assert!(matches!(
        lock_operation(&paths),
        Err(EnsureStateError::ReleaseInProgress { .. })
    ));
    assert!(matches!(
        crate::fleet_ensure::ops::capacity_import::journal::CapacityImportJournalStore::open(
            &paths
        ),
        Err(
            crate::fleet_ensure::ops::capacity_import::journal::CapacityImportJournalError::State(
                EnsureStateError::ReleaseInProgress { .. }
            )
        )
    ));
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn changed_review_inventory_and_terminal_state_cannot_reserve() {
    let directory = crate::test_support::temp_dir("release-reservation-authority");
    let paths = EnsurePaths::under(&directory, "local", "release");
    let (review, observed) = fixture();
    let review = prepare_review(review, &observed).unwrap();
    seed(&paths, &review);
    drop(ReleaseObservationJournal::attach(&paths, &review, &observed).unwrap());
    let mut wrong = review.clone();
    wrong.sources[0].maximum_paid_calls += 1;
    assert!(matches!(
        ReleaseObservationJournal::resume(&paths, &wrong),
        Err(ReleaseReservationError::Evidence(
            FleetReleaseError::Authority
        ))
    ));
    let original = read_journal(&paths).unwrap().unwrap();
    let mut changed = original.clone();
    changed.plan_sha256 = hex_bytes([20; 32]);
    write_journal(&paths, &changed).unwrap();
    assert!(matches!(
        ReleaseObservationJournal::resume(&paths, &review),
        Err(ReleaseReservationError::Integrity)
    ));
    changed = original.clone();
    changed
        .release
        .as_mut()
        .unwrap()
        .reserved_paid_calls
        .remove(&review.sources[0].binding.canister_id.to_text());
    write_journal(&paths, &changed).unwrap();
    assert!(matches!(
        ReleaseObservationJournal::resume(&paths, &review),
        Err(ReleaseReservationError::Integrity)
    ));
    changed = original.clone();
    changed
        .release
        .as_mut()
        .unwrap()
        .reserved_paid_calls
        .insert(review.sources[0].binding.canister_id.to_text(), u32::MAX);
    write_journal(&paths, &changed).unwrap();
    assert!(matches!(
        ReleaseObservationJournal::resume(&paths, &review),
        Err(ReleaseReservationError::Evidence(
            FleetReleaseError::Budget { .. }
        ))
    ));
    changed = original;
    changed.completion = FleetEnsureCompletion::Converged;
    write_journal(&paths, &changed).unwrap();
    assert!(matches!(
        ReleaseObservationJournal::resume(&paths, &review),
        Err(ReleaseReservationError::Integrity)
    ));
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn release_quote_is_part_of_review_and_both_allowances_are_checked() {
    let (mut review, observed) = fixture();
    review.sources[0].maximum_call_debit_cycles += 1;
    assert!(matches!(
        prepare_review(review, &observed),
        Err(FleetReleaseError::Budget { .. })
    ));
    let (review, _) = fixture();
    let mut source = review.sources[0].clone();
    source.maximum_debit_cycles = 39;
    assert!(matches!(
        reserve_observation_calls(&source, 0, 4),
        Err(FleetReleaseError::ObservationBudget {
            remaining_calls: 20,
            required_debit_cycles: 40,
            remaining_debit_cycles: 39,
            ..
        })
    ));
    source.maximum_call_debit_cycles = u128::MAX;
    assert!(matches!(
        reserve_observation_calls(&source, 0, 4),
        Err(FleetReleaseError::Budget { .. })
    ));
}

#[cfg(unix)]
#[test]
fn persistence_refusal_issues_no_token_and_requires_reopening() {
    let directory = crate::test_support::temp_dir("release-reservation-persistence");
    let paths = EnsurePaths::under(&directory, "local", "release");
    let (review, observed) = fixture();
    let review = prepare_review(review, &observed).unwrap();
    let id = review.sources[0].binding.canister_id;
    seed(&paths, &review);
    let mut owner = ReleaseObservationJournal::attach(&paths, &review, &observed).unwrap();
    let parent = paths.journal.parent().unwrap();
    let permissions = fs::metadata(parent).unwrap().permissions();
    fs::set_permissions(parent, fs::Permissions::from_mode(0o500)).unwrap();
    let result = owner.reserve(id).map(|_| ());
    fs::set_permissions(parent, permissions).unwrap();
    assert!(matches!(result, Err(ReleaseReservationError::State(_))));
    assert!(matches!(
        owner.reserve(id),
        Err(ReleaseReservationError::PersistenceUncertain)
    ));
    drop(owner);
    let mut resumed = ReleaseObservationJournal::resume(&paths, &review).unwrap();
    let _ = resumed.reserve(id).unwrap();
    assert_eq!(
        read_journal(&paths)
            .unwrap()
            .unwrap()
            .release
            .unwrap()
            .reserved_paid_calls[&id.to_text()],
        4
    );
    drop(resumed);
    fs::remove_dir_all(directory).unwrap();
}
