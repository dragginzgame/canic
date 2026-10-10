//! Review digests bind authority, destructive dispositions and debit allowances.

use super::*;
use crate::fleet_ensure::policy::capacity_import::tests::{plan, principal, root_budget};

#[test]
fn review_encoding_preserves_large_cycle_amounts_and_exact_approval() {
    let initial = plan();
    let mut sources = initial.sources;
    sources[0].observed_cycles = u128::from(u64::MAX) + 1_000;
    let plan = prepare_review(initial.authority, sources, root_budget()).unwrap();
    verify_review(&plan, plan.plan_sha256).unwrap();
    let encoded = serde_json::to_vec(&plan).unwrap();
    assert_eq!(
        serde_json::to_value(&plan).unwrap()["funding_credits"],
        serde_json::json!([])
    );
    let restored: CapacityImportPlanRecord = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(restored, plan);
    let projection = serde_json::to_value(&plan).unwrap();
    for field in projection.as_object().unwrap().keys() {
        let mut incomplete = projection.clone();
        incomplete.as_object_mut().unwrap().remove(field);
        let error = serde_json::from_value::<CapacityImportPlanRecord>(incomplete).unwrap_err();
        assert_eq!(error.classify(), serde_json::error::Category::Data);
    }
    verify_review(&restored, plan.plan_sha256).unwrap();
    assert!(matches!(
        verify_review(&restored, [0; 32]),
        Err(CapacityImportReviewError::DigestMismatch)
    ));
}

#[test]
fn content_substitution_cannot_reuse_approval() {
    let plan = plan();
    let changes: [fn(&mut CapacityImportPlanRecord); 8] = [
        |p| p.authority.operator = principal(20),
        |p| p.authority.network_root_key_sha256 = [20; 32],
        |p| p.authority.root_authority_sha256 = [20; 32],
        |p| p.sources[0].binding.canister_version += 1,
        |p| p.sources[0].maximum_debit_cycles += 1,
        |p| p.sources[0].observed_cycles += 1,
        |p| p.final_controllers.push(principal(20)),
        |p| p.root_budget.maximum_debit_cycles += 1,
    ];
    for change in changes {
        let mut substituted = plan.clone();
        change(&mut substituted);
        assert!(matches!(
            verify_review(&substituted, plan.plan_sha256),
            Err(CapacityImportReviewError::DigestMismatch)
        ));
    }
}

#[test]
fn review_canonicalizes_order_without_hiding_duplicate_authority() {
    let plan = plan();
    let mut sources = plan.sources.clone();
    sources[0].binding.controllers.reverse();
    assert_eq!(
        prepare_review(plan.authority.clone(), sources, plan.root_budget).unwrap(),
        plan
    );
    let mut authority = plan.authority.clone();
    authority
        .recovery_controllers
        .push(authority.recovery_controllers[0]);
    assert!(matches!(
        prepare_review(authority, plan.sources, plan.root_budget),
        Err(CapacityImportReviewError::Policy(
            CapacityImportPolicyError::InvalidAuthority
        ))
    ));
}

#[test]
fn capacity_import_does_not_infer_missing_module_authority_or_wrap_versions() {
    let plan = plan();
    let mut value = serde_json::to_value(&plan).unwrap();
    value["sources"][0]["binding"]
        .as_object_mut()
        .unwrap()
        .remove("module_sha256");
    assert!(serde_json::from_value::<CapacityImportPlanRecord>(value).is_err());
    let mut sources = plan.sources;
    sources[0].binding.canister_version = u64::MAX;
    assert!(matches!(
        prepare_review(plan.authority, sources, root_budget()),
        Err(CapacityImportReviewError::Policy(
            CapacityImportPolicyError::VersionExhausted { .. }
        ))
    ));
}

#[test]
fn capacity_import_root_reservation_preserves_reviewed_authority_and_budget() {
    use canic_contracts::dto::pool_import::{
        PoolImportPhase, PoolImportSourceProgress, PoolImportStatus,
    };
    let plan = plan();
    let request = root_reservation(&plan).unwrap();
    assert_eq!(request.sequence, plan.authority.import_sequence);
    assert_eq!(
        request.sources[0].observed_cycles,
        plan.sources[0].observed_cycles
    );
    assert_eq!(
        request.maximum_root_debit_cycles,
        plan.root_budget.maximum_debit_cycles
    );
    assert_eq!(
        request.maximum_paid_calls,
        plan.root_budget.maximum_paid_calls
    );
    let mut status = PoolImportStatus {
        root_receipt: None,
        reserved_at_ns: 1,
        reservation: request,
        progress: vec![PoolImportSourceProgress::AwaitingHandoff],
        phase: PoolImportPhase::Reserved,
        paid_calls: 0,
        reserved_debit_cycles: 0,
        last_root_cycles: plan.root_budget.observed_cycles,
    };
    assert_eq!(
        reservation_evidence(&plan, &status).unwrap().plan_sha256,
        plan.plan_sha256
    );
    status.reservation.sequence += 1;
    assert!(matches!(
        reservation_evidence(&plan, &status),
        Err(CapacityImportReviewError::ReservationMismatch)
    ));
    status.reservation.sequence -= 1;
    status.reservation.maximum_root_debit_cycles += 1;
    assert!(matches!(
        reservation_evidence(&plan, &status),
        Err(CapacityImportReviewError::ReservationMismatch)
    ));
}
