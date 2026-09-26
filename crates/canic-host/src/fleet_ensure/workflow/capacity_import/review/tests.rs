//! Command admission rejects missing current authority before any ICP execution.

use super::*;

#[test]
fn review_requires_current_completed_records_before_icp() {
    let directory = crate::test_support::temp_dir("capacity-import-review-admission");
    let request = CapacityImportReviewRequest {
        environment: "staging".into(),
        fleet: "fleet".into(),
        canisters: vec![],
        root: None,
        declarations: "missing.toml".into(),
        policy: "policy.toml".into(),
        seed: "seed.toml".into(),
        maximum_source_debit_cycles: 1,
        maximum_root_debit_cycles: 1,
        maximum_root_paid_calls: 1,
    };
    let icp = IcpCli::new("/no/such/icp", Some("staging".into()));
    assert!(matches!(
        plan(&directory, &request, &icp),
        Err(CapacityImportJournalError::InfrastructureRequired)
    ));
    assert!(matches!(
        apply(&directory, "staging", "fleet", [1; 32], &icp),
        Err(CapacityImportJournalError::Integrity)
    ));
    assert!(matches!(
        apply(&directory, "../elsewhere", "fleet", [1; 32], &icp),
        Err(CapacityImportJournalError::Integrity)
    ));
    std::fs::remove_dir_all(directory).unwrap();
}
