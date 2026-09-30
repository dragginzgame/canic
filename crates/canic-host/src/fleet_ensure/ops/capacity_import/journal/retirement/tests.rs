//! Rejected ingress recovery keeps original cycle baselines and append-only request history.

use super::*;
use crate::fleet_ensure::{
    ops::{
        EnsurePaths,
        capacity_import::{
            journal::CapacityImportJournalStore,
            transport::tests::{completion, rejected, request_with_expiry},
        },
    },
    policy::capacity_import::{
        CapacityImportPolicyError,
        tests::{principal, sources},
    },
};

fn issued() -> (CapacityImportJournalRecord, CapacityImportSourceView) {
    let record = journal::tests::reserved();
    let source = sources(&record.plan).remove(0);
    let request = request_with_expiry(&record.plan, source.binding.canister_id, 1);
    let intent = journal::prepare_handoff(&record, &source, request).unwrap();
    (
        journal::issue_handoff(&intent, source.binding.canister_id).unwrap(),
        source,
    )
}

fn reject(record: &CapacityImportJournalRecord) -> CapacityImportJournalRecord {
    let handoff = &record.handoffs[0];
    let witness = rejected(
        &record.plan,
        handoff.canister_id,
        handoff.request.as_ref().unwrap(),
    );
    retain(record, handoff.canister_id, &witness).unwrap()
}

#[test]
fn capacity_import_rejected_handoff_resumes_after_restart_without_rebasing() {
    let (issued, mut source) = issued();
    let root = crate::test_support::temp_dir("capacity-import-rejection");
    let paths = EnsurePaths::under(&root, "local", "staging");
    let store = CapacityImportJournalStore::open(&paths).unwrap();
    store.stage(issued.plan.clone()).unwrap();
    let reserved = journal::tests::reserved();
    store.save(&reserved).unwrap();
    let request = issued.handoffs[0].request.clone().unwrap();
    store
        .save(&journal::prepare_handoff(&reserved, &source, request).unwrap())
        .unwrap();
    store.save(&issued).unwrap();
    let failed = reject(&issued);
    store.save(&failed).unwrap();
    assert_eq!(reject(&failed), failed);
    drop(store);
    let store = CapacityImportJournalStore::open(&paths).unwrap();
    let failed = store.read().unwrap().unwrap();
    source.cycles -= 25;
    let request = request_with_expiry(&failed.plan, source.binding.canister_id, 2);
    let renewed = renew(&failed, &source, request.clone()).unwrap();
    store.save(&renewed).unwrap();
    assert_eq!(
        renewed.handoffs[0].effect.as_ref().unwrap().pre_cycles,
        Some(1000)
    );
    assert_eq!(renewed.handoffs[0].before_reserved_cycles, Some(100));
    assert_eq!(
        renewed.handoffs[0].retirements,
        failed.handoffs[0].retirements
    );
    assert!(matches!(
        store.save(&failed),
        Err(CapacityImportJournalError::Conflict)
    ));
    drop(store);
    let store = CapacityImportJournalStore::open(&paths).unwrap();
    let renewed = store.read().unwrap().unwrap();
    let sent = journal::issue_handoff(&renewed, source.binding.canister_id).unwrap();
    store.save(&sent).unwrap();
    source.binding.controllers = sent.plan.transitional_controllers.clone();
    source.binding.canister_version += 1;
    source.cycles -= 25;
    let done = journal::observe_handoff(
        &sent,
        &source,
        &completion(&sent.plan, source.binding.canister_id, &request),
    )
    .unwrap();
    store.save(&done).unwrap();
    assert_eq!(done.handoffs[0].retirements.len(), 1);
    assert_eq!(store.read().unwrap().unwrap(), done);
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn capacity_import_renewal_requires_exact_certified_rejection_and_original_source() {
    let (issued, source) = issued();
    let id = source.binding.canister_id;
    let new_request = request_with_expiry(&issued.plan, id, 2);
    assert!(matches!(
        renew(&issued, &source, new_request.clone()),
        Err(CapacityImportJournalError::Unresolved)
    ));
    let wrong = rejected(
        &issued.plan,
        principal(90),
        issued.handoffs[0].request.as_ref().unwrap(),
    );
    assert!(matches!(
        retain(&issued, id, &wrong),
        Err(CapacityImportJournalError::Integrity)
    ));
    let wrong = rejected(&issued.plan, id, &new_request);
    assert!(matches!(
        retain(&issued, id, &wrong),
        Err(CapacityImportJournalError::Integrity)
    ));
    let failed = reject(&issued);
    let mut changed = source.clone();
    changed.binding.canister_version += 1;
    assert!(matches!(
        renew(&failed, &changed, new_request.clone()),
        Err(CapacityImportJournalError::Policy(
            CapacityImportPolicyError::SourceChanged { .. }
        ))
    ));
    changed = source.clone();
    changed.binding.controllers = failed.plan.transitional_controllers.clone();
    assert!(matches!(
        renew(&failed, &changed, new_request.clone()),
        Err(CapacityImportJournalError::Policy(
            CapacityImportPolicyError::SourceChanged { .. }
        ))
    ));
    changed = source.clone();
    changed.cycles += 1;
    assert!(renew(&failed, &changed, new_request.clone()).is_err());
    changed = source.clone();
    changed.cycles -= 201;
    assert!(matches!(
        renew(&failed, &changed, new_request),
        Err(CapacityImportJournalError::Policy(
            CapacityImportPolicyError::ConservationUnproven { .. }
        ))
    ));
    assert!(matches!(
        renew(
            &failed,
            &source,
            failed.handoffs[0].request.clone().unwrap()
        ),
        Err(CapacityImportJournalError::RequestInvalid)
    ));
}

#[test]
fn capacity_import_second_rejection_is_terminal_and_history_cannot_be_erased() {
    let (issued, source) = issued();
    let failed = reject(&issued);
    let request = request_with_expiry(&failed.plan, source.binding.canister_id, 2);
    let renewed = renew(&failed, &source, request).unwrap();
    assert!(monotonic(&failed.handoffs[0], &renewed.handoffs[0]));
    assert!(!monotonic(&issued.handoffs[0], &renewed.handoffs[0]));
    let sent = journal::issue_handoff(&renewed, source.binding.canister_id).unwrap();
    let terminal = reject(&sent);
    assert_eq!(terminal.handoffs[0].retirements.len(), 2);
    let third = request_with_expiry(&terminal.plan, source.binding.canister_id, 3);
    assert!(matches!(
        renew(&terminal, &source, third),
        Err(CapacityImportJournalError::BudgetExhausted { .. })
    ));
    assert!(!monotonic(&terminal.handoffs[0], &sent.handoffs[0]));
    let mut corrupt = terminal;
    corrupt.handoffs[0].retirements[0].certificate_sha256 = [0; 32];
    assert!(matches!(
        journal::validate(&corrupt),
        Err(CapacityImportJournalError::Integrity)
    ));
}

#[test]
fn retired_pruned_handoff_reconciles_exact_custody_or_renews_original() {
    use crate::fleet_ensure::ops::capacity_import::transport::tests::retired;
    use ic_agent::agent::RequestStatusResponse;
    for status in [RequestStatusResponse::Done, RequestStatusResponse::Unknown] {
        let (issued, source) = issued();
        let id = source.binding.canister_id;
        let witness = retired(
            &issued.plan,
            id,
            issued.handoffs[0].request.as_ref().unwrap(),
            status,
            2,
        );
        let retired = retain(&issued, id, &witness).unwrap();
        let bytes = serde_json::to_vec(&retired).unwrap();
        let reopened: CapacityImportJournalRecord = serde_json::from_slice(&bytes).unwrap();
        let replacement = request_with_expiry(&issued.plan, id, 3);
        let renewed = reconcile(&reopened, &source, replacement.clone()).unwrap();
        assert_eq!(
            renewed.handoffs[0].effect.as_ref().unwrap().state,
            EffectState::Intent
        );
        assert_eq!(
            renewed.handoffs[0].effect.as_ref().unwrap().pre_cycles,
            issued.handoffs[0].effect.as_ref().unwrap().pre_cycles
        );
        assert!(monotonic(&reopened.handoffs[0], &renewed.handoffs[0]));

        let mut completed_source = source.clone();
        completed_source.binding.controllers = issued.plan.transitional_controllers.clone();
        completed_source.binding.canister_version += 1;
        completed_source.cycles -= 25;
        let completed = reconcile(&reopened, &completed_source, replacement.clone()).unwrap();
        journal::validate(&completed).unwrap();
        assert!(journal::custody_ready(&completed, id));
        assert_eq!(completed.handoffs[0].request, issued.handoffs[0].request);
        assert_eq!(
            completed.handoffs[0].retirements,
            reopened.handoffs[0].retirements
        );
        assert_eq!(
            completed.handoffs[0].effect.as_ref().unwrap().post_cycles,
            Some(completed_source.cycles)
        );
        assert!(monotonic(&reopened.handoffs[0], &completed.handoffs[0]));
        assert!(!monotonic(&completed.handoffs[0], &issued.handoffs[0]));

        completed_source.binding.canister_version += 1;
        assert!(matches!(
            reconcile(&reopened, &completed_source, replacement.clone()),
            Err(CapacityImportJournalError::Unresolved)
        ));
        completed_source.binding.canister_version = source.binding.canister_version;
        assert!(matches!(
            reconcile(&reopened, &completed_source, replacement),
            Err(CapacityImportJournalError::Unresolved)
        ));
    }
}

#[test]
fn refresh_never_issued_ingress_preserves_effect_authority_and_rejects_issued_state() {
    let reserved = journal::tests::reserved();
    let source = sources(&reserved.plan).remove(0);
    let id = source.binding.canister_id;
    let first = request_with_expiry(&reserved.plan, id, 1);
    let original = journal::prepare_handoff(&reserved, &source, first).unwrap();
    let replacement = request_with_expiry(&reserved.plan, id, 2);
    let refreshed = refresh_unissued(&original, id, replacement.clone()).unwrap();
    assert_eq!(original.handoffs[0].effect, refreshed.handoffs[0].effect);
    assert_eq!(
        original.handoffs[0].before_reserved_cycles,
        refreshed.handoffs[0].before_reserved_cycles
    );
    assert!(refreshed.handoffs[0].retirements.is_empty());
    let issued = journal::issue_handoff(&refreshed, id).unwrap();
    assert!(monotonic(&original.handoffs[0], &issued.handoffs[0]));
    assert!(matches!(
        refresh_unissued(&issued, id, replacement),
        Err(CapacityImportJournalError::Unresolved)
    ));
}
