//! Exhaustion recovery retains spending authority and gives each reviewed counter a finite extension.

use super::*;
use crate::fleet_ensure::{
    model::capacity_import::survey::CapacityImportSurveyRecord,
    ops::capacity_import::{
        admission::survey::CapacityImportSurveyStore, journal::CapacityImportJournalStore,
    },
    policy::capacity_import::tests::principal,
};

fn exhausted_survey(paths: &EnsurePaths, digest: [u8; 32]) -> std::path::PathBuf {
    let owner = CapacityImportJournalStore::open(paths).unwrap();
    let mut survey =
        CapacityImportSurveyStore::open(&owner, paths, digest, &[principal(9)]).unwrap();
    survey.reserve(principal(9)).unwrap();
    survey.reserve(principal(9)).unwrap();
    paths
        .plan
        .with_file_name("capacity-import-surveys")
        .join(format!("{}.json", hex_bytes(digest)))
}

#[test]
fn reviewed_survey_continuation_keeps_spent_attempts_and_replays_without_another_grant() {
    let directory = crate::test_support::temp_dir("attempt-recovery-survey");
    let paths = EnsurePaths::under(&directory, "local", "survey");
    let file = exhausted_survey(&paths, [1; 32]);
    let before = std::fs::read(&file).unwrap();
    let review =
        crate::fleet_ensure::workflow::attempt_recovery::review(&directory, "local", "survey")
            .unwrap();
    assert_eq!(std::fs::read(&file).unwrap(), before);
    assert_eq!(review.owners[0].grants[0].spent_attempts, 2);
    crate::fleet_ensure::workflow::attempt_recovery::apply(
        &directory,
        "local",
        "survey",
        review.review_sha256,
    )
    .unwrap();
    let approved = std::fs::read(&file).unwrap();
    let record: CapacityImportSurveyRecord = serde_json::from_slice(&approved).unwrap();
    assert_eq!(record.canisters[&principal(9).to_text()].attempts, 2);
    assert_eq!(record.request_sha256, [1; 32]);
    assert_eq!(record.attempt_recoveries.len(), 1);
    crate::fleet_ensure::workflow::attempt_recovery::apply(
        &directory,
        "local",
        "survey",
        review.review_sha256,
    )
    .unwrap();
    assert_eq!(std::fs::read(&file).unwrap(), approved);
    let owner = CapacityImportJournalStore::open(&paths).unwrap();
    let mut survey =
        CapacityImportSurveyStore::open(&owner, &paths, [1; 32], &[principal(9)]).unwrap();
    survey.reserve(principal(9)).unwrap();
    survey.reserve(principal(9)).unwrap();
    assert!(matches!(
        survey.reserve(principal(9)),
        Err(CapacityImportJournalError::BudgetExhausted { .. })
    ));
    drop(survey);
    drop(owner);
    let second =
        crate::fleet_ensure::workflow::attempt_recovery::review(&directory, "local", "survey")
            .unwrap();
    assert_ne!(second.review_sha256, review.review_sha256);
    assert_eq!(second.owners[0].grants[0].spent_attempts, 4);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn changed_owner_and_wrong_digest_refuse_before_any_allowance_changes() {
    let directory = crate::test_support::temp_dir("attempt-recovery-changed");
    let paths = EnsurePaths::under(&directory, "local", "survey");
    let first = exhausted_survey(&paths, [1; 32]);
    let second = exhausted_survey(&paths, [2; 32]);
    let review =
        crate::fleet_ensure::workflow::attempt_recovery::review(&directory, "local", "survey")
            .unwrap();
    let original = std::fs::read(&first).unwrap();
    let mut changed: CapacityImportSurveyRecord =
        serde_json::from_slice(&std::fs::read(&second).unwrap()).unwrap();
    changed
        .canisters
        .get_mut(&principal(9).to_text())
        .unwrap()
        .attempts = 1;
    std::fs::write(&second, serde_json::to_vec(&changed).unwrap()).unwrap();
    assert!(matches!(
        crate::fleet_ensure::workflow::attempt_recovery::apply(
            &directory,
            "local",
            "survey",
            review.review_sha256
        ),
        Err(CapacityImportJournalError::Integrity)
    ));
    assert_eq!(std::fs::read(&first).unwrap(), original);
    assert!(
        crate::fleet_ensure::workflow::attempt_recovery::apply(
            &directory, "local", "survey", [3; 32]
        )
        .is_err()
    );
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn interrupted_local_grant_publication_resumes_exactly_once() {
    let directory = crate::test_support::temp_dir("attempt-recovery-partial");
    let paths = EnsurePaths::under(&directory, "local", "survey");
    exhausted_survey(&paths, [1; 32]);
    exhausted_survey(&paths, [2; 32]);
    let review =
        crate::fleet_ensure::workflow::attempt_recovery::review(&directory, "local", "survey")
            .unwrap();
    let (path, bytes) = owner::prepare(&paths, &review.owners[0]).unwrap().unwrap();
    ic_host_fs::durable::write_bytes(&path, &bytes).unwrap();
    crate::fleet_ensure::workflow::attempt_recovery::apply(
        &directory,
        "local",
        "survey",
        review.review_sha256,
    )
    .unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    for reviewed in &review.owners {
        assert!(owner::prepare(&paths, reviewed).unwrap().is_none());
    }
    std::fs::remove_dir_all(directory).unwrap();
}
