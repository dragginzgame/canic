//! Interrupted initial observation must preserve finite attempts and the first balance baseline.

use super::*;
use crate::fleet_ensure::policy::capacity_import::tests::{plan, principal};

#[test]
fn restart_keeps_lost_observation_attempts_spent() {
    let directory = crate::test_support::temp_dir("capacity-import-survey-budget");
    let paths = EnsurePaths::under(&directory, "local", "survey");
    let owner = CapacityImportJournalStore::open(&paths).unwrap();
    let mut survey =
        CapacityImportSurveyStore::open(&owner, &paths, [1; 32], &[principal(9)]).unwrap();
    survey.reserve(principal(9)).unwrap();
    drop(survey);
    drop(owner);
    let owner = CapacityImportJournalStore::open(&paths).unwrap();
    let mut survey =
        CapacityImportSurveyStore::open(&owner, &paths, [1; 32], &[principal(9)]).unwrap();
    survey.reserve(principal(9)).unwrap();
    assert!(matches!(
        survey.reserve(principal(9)),
        Err(CapacityImportJournalError::BudgetExhausted { .. })
    ));
    drop(survey);
    drop(owner);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn successful_sample_cannot_be_rebased_or_reassigned_after_restart() {
    let directory = crate::test_support::temp_dir("capacity-import-survey-sample");
    let paths = EnsurePaths::under(&directory, "local", "survey");
    let owner = CapacityImportJournalStore::open(&paths).unwrap();
    let mut survey =
        CapacityImportSurveyStore::open(&owner, &paths, [2; 32], &[principal(9)]).unwrap();
    let source = plan().sources.remove(0);
    let sample = CapacityImportSampleRecord {
        binding: source.binding,
        cycles: source.observed_cycles,
        reserved_cycles: source.observed_reserved_cycles,
    };
    assert!(matches!(
        survey.retain(sample.clone()),
        Err(CapacityImportJournalError::Integrity)
    ));
    survey.reserve(principal(9)).unwrap();
    survey.retain(sample.clone()).unwrap();
    let retained = std::fs::read(&survey.path).unwrap();
    assert_eq!(
        CapacityImportSurveyStore::original_sample(
            &owner,
            &paths,
            [2; 32],
            &[principal(9)],
            principal(9)
        )
        .unwrap(),
        Some(sample.clone())
    );
    assert!(matches!(
        CapacityImportSurveyStore::original_sample(
            &owner,
            &paths,
            [2; 32],
            &[principal(9), principal(10)],
            principal(9)
        ),
        Err(CapacityImportJournalError::Integrity)
    ));
    assert_eq!(std::fs::read(&survey.path).unwrap(), retained);
    drop(survey);
    let mut survey =
        CapacityImportSurveyStore::open(&owner, &paths, [2; 32], &[principal(9)]).unwrap();
    assert_eq!(survey.sample(principal(9)), Some(&sample));
    let mut changed = sample;
    changed.cycles += 1;
    assert!(matches!(
        survey.retain(changed),
        Err(CapacityImportJournalError::Integrity)
    ));
    assert!(CapacityImportSurveyStore::open(&owner, &paths, [2; 32], &[principal(10)]).is_err());
    drop(survey);
    drop(owner);
    std::fs::remove_dir_all(directory).unwrap();
}
