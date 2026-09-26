//! Completed-record preflight preserves source evidence and never grants execution authority.

use super::*;
use crate::test_support::temp_dir;
use std::fs;

fn completed(root: &Path) -> EnsurePaths {
    let paths = EnsurePaths::under(root, "staging", "fleet");
    fs::create_dir_all(paths.plan.parent().unwrap()).unwrap();
    let identity = serde_json::json!({
        "schema_version": 1,
        "operation_id": "ab".repeat(32),
        "plan_sha256": "cd".repeat(32),
        "fleet": "fleet",
        "environment": "staging",
    });
    let mut plan = identity.clone();
    plan["reviewed_desired"] = serde_json::json!({
        "desired": {"bootstrap": {"coordinator": "coordinator"}}
    });
    let mut journal = identity;
    journal["completion"] = "converged".into();
    fs::write(&paths.plan, serde_json::to_vec(&plan).unwrap()).unwrap();
    fs::write(&paths.journal, serde_json::to_vec(&journal).unwrap()).unwrap();
    paths
}

#[test]
fn incomplete_receipt_evidence_rejects_before_current_state_decoding() {
    let root = temp_dir("completed-contract-preflight");
    let paths = completed(&root);
    // Neither current state nor phase decoding may mask the early source decision.
    fs::write(&paths.state, b"not a current state contract").unwrap();
    let before = [
        fs::read(&paths.plan).unwrap(),
        fs::read(&paths.journal).unwrap(),
        fs::read(&paths.state).unwrap(),
    ];
    assert!(matches!(
        check(&root, "staging", "fleet"),
        Err(RetainedContractError::ReceiptAudit(_))
    ));
    assert_eq!(before[0], fs::read(&paths.plan).unwrap());
    assert_eq!(before[1], fs::read(&paths.journal).unwrap());
    assert_eq!(before[2], fs::read(&paths.state).unwrap());
    assert!(!paths.lock.exists());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn mismatched_completion_claim_is_not_presented_as_a_completed_estate() {
    let root = temp_dir("completed-contract-identity");
    let paths = completed(&root);
    let mut plan: Value = serde_json::from_slice(&fs::read(&paths.plan).unwrap()).unwrap();
    plan["operation_id"] = "ef".repeat(32).into();
    fs::write(&paths.plan, serde_json::to_vec(&plan).unwrap()).unwrap();
    assert!(matches!(
        check(&root, "staging", "fleet"),
        Err(RetainedContractError::IdentityMismatch)
    ));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn current_controller_declaration_does_not_authorize_or_block_source_execution() {
    let root = temp_dir("completed-contract-current");
    let paths = completed(&root);
    let mut plan: Value = serde_json::from_slice(&fs::read(&paths.plan).unwrap()).unwrap();
    plan["reviewed_desired"]["desired"]["bootstrap"]["recovery_controllers"] =
        serde_json::json!([]);
    fs::write(&paths.plan, serde_json::to_vec(&plan).unwrap()).unwrap();
    check(&root, "staging", "fleet").unwrap();
    // Invalid present values remain the strict current decoder's responsibility.
    plan["reviewed_desired"]["desired"]["bootstrap"]["recovery_controllers"] =
        serde_json::json!("invalid");
    fs::write(&paths.plan, serde_json::to_vec(&plan).unwrap()).unwrap();
    check(&root, "staging", "fleet").unwrap();
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn incomplete_work_remains_with_its_existing_recovery_owner() {
    let root = temp_dir("completed-contract-incomplete");
    let paths = completed(&root);
    let mut journal: Value = serde_json::from_slice(&fs::read(&paths.journal).unwrap()).unwrap();
    journal["completion"] = "in_progress".into();
    fs::write(&paths.journal, serde_json::to_vec(&journal).unwrap()).unwrap();
    check(&root, "staging", "fleet").unwrap();
    fs::remove_dir_all(root).unwrap();
}

#[test]
#[ignore = "requires the explicit read-only external evidence path"]
fn inspect_supplied_completed_source_without_external_mutation() {
    let root = std::env::var_os("CANIC_COMPLETED_SOURCE_WORKSPACE").unwrap();
    let environment = std::env::var("CANIC_COMPLETED_SOURCE_ENVIRONMENT").unwrap();
    let fleet = std::env::var("CANIC_COMPLETED_SOURCE_FLEET").unwrap();
    assert!(matches!(
        check(Path::new(&root), &environment, &fleet),
        Err(RetainedContractError::CompletedAuthorityContract { .. })
    ));
}

#[test]
fn reinstall_checks_completed_source_before_state_or_release_artifacts() {
    let mut fixture = crate::fleet_ensure::tests::protocol_tranche_fixture(Vec::new());
    fixture.desired.environment = "staging".into();
    fixture.desired.fleet = "fleet".into();
    let paths = completed(&fixture.root);
    fs::write(&paths.state, b"not a current state contract").unwrap();
    let result = crate::fleet_ensure::workflow::plan_reinstall(
        &fixture.root,
        &fixture.desired,
        "unbuilt-target",
        "fleet",
        1,
        &mut fixture.platform,
    );
    assert!(matches!(
        result,
        Err(
            crate::fleet_ensure::workflow::EnsureWorkflowError::RetainedContract(
                RetainedContractError::ReceiptAudit(_)
            )
        )
    ));
    assert!(!paths.lock.exists());
    assert_eq!(
        fs::read(&paths.state).unwrap(),
        b"not a current state contract"
    );
}
