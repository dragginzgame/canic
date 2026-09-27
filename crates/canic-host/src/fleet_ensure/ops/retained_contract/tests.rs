//! Current-source selection leaves ordinary review and recovery admission to their owners.

use super::*;
use crate::fleet_ensure::{
    model::FleetEnsureCompletion,
    ops::{read_journal, write_journal},
};
use std::fs;

#[test]
fn incomplete_work_remains_with_its_existing_recovery_owner() {
    let (fixture, paths, _) = crate::fleet_ensure::tests::terminal_retirement_fixture();
    let mut journal = read_journal(&paths).unwrap().unwrap();
    journal.completion = FleetEnsureCompletion::InProgress;
    write_journal(&paths, &journal).unwrap();
    let before = fs::read(&paths.journal).unwrap();
    check(&fixture.root, "local", "test-fleet").unwrap();
    assert!(!completed_source_available(&fixture.root, "local", "test-fleet").unwrap());
    assert_eq!(before, fs::read(&paths.journal).unwrap());
    fs::remove_dir_all(fixture.root).unwrap();
}

#[test]
fn pending_current_review_is_not_a_completed_source() {
    let (fixture, paths, _) = crate::fleet_ensure::tests::terminal_retirement_fixture();
    let mut plan = read_plan(&paths).unwrap().unwrap();
    plan.operation_id = "ef".repeat(32);
    plan.plan_sha256 = "12".repeat(32);
    crate::fleet_ensure::ops::write_plan(&paths, &plan).unwrap();
    let before = fs::read(&paths.journal).unwrap();
    check(&fixture.root, "local", "test-fleet").unwrap();
    assert!(!completed_source_available(&fixture.root, "local", "test-fleet").unwrap());
    assert_eq!(before, fs::read(&paths.journal).unwrap());
    fs::remove_dir_all(fixture.root).unwrap();
}
