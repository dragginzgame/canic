//! Same-operation recovery preserves exact handoff intent and source cycle baselines.

use super::*;
use crate::fleet_ensure::policy::capacity_import::tests::root_budget;
use crate::fleet_ensure::{
    ops::capacity_import::prepare_review,
    policy::capacity_import::tests::{destination, plan, principal, sources},
    view::capacity_import::CapacityImportOwnershipView,
};

pub(super) fn reserved() -> CapacityImportJournalRecord {
    let plan = plan();
    let record = reviewed(plan.clone()).unwrap();
    let record = approve(
        &record,
        plan.plan_sha256,
        &destination(&plan),
        &sources(&plan),
    )
    .unwrap();
    reserve(
        &record,
        CapacityImportReservationRecord {
            plan_sha256: plan.plan_sha256,
            authority: plan.authority.clone(),
            sources: plan
                .sources
                .iter()
                .map(|source| source.binding.canister_id)
                .collect(),
        },
    )
    .unwrap()
}

fn restart(record: &CapacityImportJournalRecord) -> CapacityImportJournalRecord {
    let restored = serde_json::from_slice(&serde_json::to_vec(record).unwrap()).unwrap();
    validate(&restored).unwrap();
    restored
}

#[test]
fn reviewed_root_custody_restarts_without_fabricated_host_effects() {
    let mut plan = plan();
    plan.sources[0].binding.controllers = vec![plan.authority.root];
    plan.sources[0].binding.stopped = false;
    let plan = prepare_review(plan.authority, plan.sources, plan.root_budget).unwrap();
    let record = reviewed(plan.clone()).unwrap();
    let record = approve(
        &record,
        plan.plan_sha256,
        &destination(&plan),
        &sources(&plan),
    )
    .unwrap();
    let record = reserve(
        &record,
        CapacityImportReservationRecord {
            plan_sha256: plan.plan_sha256,
            authority: plan.authority.clone(),
            sources: vec![plan.sources[0].binding.canister_id],
        },
    )
    .unwrap();
    let restored = restart(&record);
    assert!(all_custody_ready(&restored));
    assert!(restored.handoffs[0].effect.is_none());
    assert!(restored.handoffs[0].request.is_none());
    assert!(matches!(
        prepare_handoff(&restored, &sources(&plan)[0]),
        Err(CapacityImportJournalError::Integrity)
    ));
}

#[test]
fn capacity_import_requires_approval_and_reservation_before_handoff_intent() {
    let plan = plan();
    let record = reviewed(plan.clone()).unwrap();
    let source = sources(&plan).remove(0);
    assert!(matches!(
        prepare_handoff(&record, &source),
        Err(CapacityImportJournalError::ReservationRequired)
    ));
    let approved = approve(
        &record,
        plan.plan_sha256,
        &destination(&plan),
        &sources(&plan),
    )
    .unwrap();
    assert!(matches!(
        prepare_handoff(&approved, &source),
        Err(CapacityImportJournalError::ReservationRequired)
    ));
    let mut reservation = reserved().reservation.unwrap();
    reservation.sources.clear();
    assert!(matches!(
        reserve(&approved, reservation),
        Err(CapacityImportJournalError::ReservationRequired)
    ));
}

#[test]
fn capacity_import_lost_handoff_reply_recovers_exact_effect_after_restart() {
    let record = reserved();
    let source = sources(&record.plan).remove(0);
    let intent = prepare_handoff(&record, &source).unwrap();
    assert_eq!(prepare_handoff(&restart(&intent), &source).unwrap(), intent);
    let issued = issue_handoff(&intent, source.binding.canister_id).unwrap();
    assert!(matches!(
        issue_handoff(&restart(&issued), source.binding.canister_id),
        Err(CapacityImportJournalError::Unresolved)
    ));
    assert!(matches!(
        observe_handoff(&restart(&issued), &source),
        Err(CapacityImportJournalError::Unresolved)
    ));
    let mut after = source.clone();
    after.binding.controllers = issued.plan.transitional_controllers.clone();
    after.binding.canister_version += 1;
    after.cycles -= 100;
    let applied = observe_handoff(&restart(&issued), &after).unwrap();
    let restored = restart(&applied);
    assert_eq!(
        restored.handoffs[0].effect.as_ref().unwrap().post_cycles,
        Some(900)
    );
    assert_eq!(observe_handoff(&restored, &source).unwrap(), restored);
    assert_eq!(prepare_handoff(&restored, &source).unwrap(), restored);
}

#[test]
fn capacity_import_code_or_version_substitution_does_not_confirm_handoff() {
    let record = reserved();
    let source = sources(&record.plan).remove(0);
    let intent = prepare_handoff(&record, &source).unwrap();
    let issued = issue_handoff(&intent, source.binding.canister_id).unwrap();
    let mut after = source;
    after.binding.controllers = issued.plan.transitional_controllers.clone();
    after.binding.canister_version += 1;
    let mut replaced_code = after.clone();
    replaced_code.binding.module_sha256 = None;
    assert!(matches!(
        observe_handoff(&issued, &replaced_code),
        Err(CapacityImportJournalError::Unresolved)
    ));
    after.binding.canister_version += 1;
    assert!(matches!(
        observe_handoff(&issued, &after),
        Err(CapacityImportJournalError::Unresolved)
    ));
}

#[test]
fn capacity_import_late_ownership_or_retirement_change_blocks_new_intent() {
    let record = reserved();
    let mut source = sources(&record.plan).remove(0);
    source.ownership = CapacityImportOwnershipView::Assigned;
    assert!(matches!(
        prepare_handoff(&record, &source),
        Err(CapacityImportJournalError::Policy(
            CapacityImportPolicyError::SourceAssigned { .. }
        ))
    ));
    source.ownership = CapacityImportOwnershipView::Unassigned;
    source.disposition_evidence_sha256 = None;
    assert!(matches!(
        prepare_handoff(&record, &source),
        Err(CapacityImportJournalError::Policy(
            CapacityImportPolicyError::DispositionUnresolved { .. }
        ))
    ));
    assert!(record.handoffs[0].effect.is_none());
}

#[test]
fn capacity_import_never_rebases_cycle_allowance_on_restart() {
    let record = reserved();
    let mut source = sources(&record.plan).remove(0);
    source.cycles -= 100;
    let intent = prepare_handoff(&record, &source).unwrap();
    let issued = issue_handoff(&intent, source.binding.canister_id).unwrap();
    source.binding.controllers = issued.plan.transitional_controllers.clone();
    source.binding.canister_version += 1;
    source.cycles = 799;
    assert!(matches!(
        observe_handoff(&restart(&issued), &source),
        Err(CapacityImportJournalError::Policy(
            CapacityImportPolicyError::ConservationUnproven { .. }
        ))
    ));
    source.cycles = 1_501;
    let credited = restart(&observe_handoff(&restart(&issued), &source).unwrap());
    assert_eq!(credited.plan, issued.plan);
    assert_eq!(
        credited.handoffs[0].effect.as_ref().unwrap().pre_cycles,
        Some(900)
    );
    assert_eq!(
        credited.handoffs[0].effect.as_ref().unwrap().post_cycles,
        Some(1_501)
    );
}

#[test]
fn capacity_import_partial_completion_preserves_each_original_source() {
    let initial = plan();
    let mut entries = initial.sources.clone();
    let mut second = entries[0].clone();
    second.binding.canister_id = principal(20);
    entries.push(second);
    let plan = prepare_review(initial.authority, entries, root_budget()).unwrap();
    let mut root = destination(&plan);
    root.maximum_capacity += 1;
    let record = approve(
        &reviewed(plan.clone()).unwrap(),
        plan.plan_sha256,
        &root,
        &sources(&plan),
    )
    .unwrap();
    let record = reserve(
        &record,
        CapacityImportReservationRecord {
            plan_sha256: plan.plan_sha256,
            authority: plan.authority.clone(),
            sources: plan
                .sources
                .iter()
                .map(|source| source.binding.canister_id)
                .collect(),
        },
    )
    .unwrap();
    let mut first = sources(&plan).remove(0);
    let intent = prepare_handoff(&record, &first).unwrap();
    let issued = issue_handoff(&intent, first.binding.canister_id).unwrap();
    first.binding.controllers = plan.transitional_controllers.clone();
    first.binding.canister_version += 1;
    first.cycles -= 10;
    let applied = observe_handoff(&issued, &first).unwrap();
    let second = sources(&plan).remove(1);
    let resumed = prepare_handoff(&restart(&applied), &second).unwrap();
    assert_eq!(resumed.handoffs[0], applied.handoffs[0]);
    assert_eq!(
        resumed.handoffs[1].effect.as_ref().unwrap().state,
        EffectState::Intent
    );
}

#[test]
fn capacity_import_corrupt_effect_authority_and_omitted_fields_reject() {
    let record = reserved();
    let intent = prepare_handoff(&record, &sources(&record.plan).remove(0)).unwrap();
    let mut corrupt = intent.clone();
    corrupt.handoffs[0]
        .effect
        .as_mut()
        .unwrap()
        .pre_canister_version = None;
    assert!(matches!(
        validate(&corrupt),
        Err(CapacityImportJournalError::Integrity)
    ));
    let mut corrupt = intent.clone();
    corrupt.handoffs[0].effect.as_mut().unwrap().action_sha256 = "00".repeat(32);
    assert!(matches!(
        validate(&corrupt),
        Err(CapacityImportJournalError::Integrity)
    ));
    let mut missing = serde_json::to_value(&intent).unwrap();
    missing.as_object_mut().unwrap().remove("reservation");
    assert!(serde_json::from_value::<CapacityImportJournalRecord>(missing).is_err());
}

#[test]
fn capacity_import_durable_handoff_reopens_without_rebasing_or_losing_intent() {
    use crate::fleet_ensure::ops::EnsurePaths;
    let root = crate::test_support::temp_dir("capacity-import-journal");
    let paths = EnsurePaths::under(&root, "local", "staging");
    let store = CapacityImportJournalStore::open(&paths).unwrap();
    let plan = plan();
    let initial = store.stage(plan.clone()).unwrap();
    let approved = approve(
        &initial,
        plan.plan_sha256,
        &destination(&plan),
        &sources(&plan),
    )
    .unwrap();
    store.save(&approved).unwrap();
    let record = reserve(&approved, reserved().reservation.unwrap()).unwrap();
    store.save(&record).unwrap();
    let intent = prepare_handoff(&record, &sources(&plan).remove(0)).unwrap();
    store.save(&intent).unwrap();
    let issued = issue_handoff(&intent, principal(9)).unwrap();
    store.save(&issued).unwrap();
    drop(store);

    let store = CapacityImportJournalStore::open(&paths).unwrap();
    assert_eq!(store.read().unwrap(), Some(issued.clone()));
    assert!(matches!(
        store.save(&intent),
        Err(CapacityImportJournalError::Conflict)
    ));
    let mut rebased = issued.clone();
    rebased.handoffs[0].effect.as_mut().unwrap().pre_cycles = Some(999);
    assert!(matches!(
        store.save(&rebased),
        Err(CapacityImportJournalError::Conflict)
    ));
    let mut changed_plan = plan;
    changed_plan.sources[0].binding.canister_id = principal(20);
    let changed_plan =
        prepare_review(changed_plan.authority, changed_plan.sources, root_budget()).unwrap();
    assert!(matches!(
        store.stage(changed_plan),
        Err(CapacityImportJournalError::Conflict)
    ));
    assert_eq!(store.read().unwrap(), Some(issued));
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn capacity_import_journal_rejects_symlinked_evidence() {
    use crate::fleet_ensure::ops::EnsurePaths;
    let root = crate::test_support::temp_dir("capacity-import-link");
    let paths = EnsurePaths::under(&root, "local", "staging");
    let store = CapacityImportJournalStore::open(&paths).unwrap();
    let target = root.join("foreign.json");
    std::fs::write(&target, serde_json::to_vec(&reserved()).unwrap()).unwrap();
    std::os::unix::fs::symlink(&target, paths.plan.with_file_name("capacity-import.json")).unwrap();
    assert!(matches!(
        store.read(),
        Err(CapacityImportJournalError::Io(_))
    ));
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn capacity_import_approval_fences_other_fleet_operations_across_restart() {
    use crate::fleet_ensure::ops::{EnsurePaths, EnsureStateError, lock_operation};
    let root = crate::test_support::temp_dir("capacity-import-fence");
    let paths = EnsurePaths::under(&root, "local", "staging");
    let plan = plan();
    let store = CapacityImportJournalStore::open(&paths).unwrap();
    let record = store.stage(plan.clone()).unwrap();
    drop(store);
    drop(lock_operation(&paths).unwrap());
    let store = CapacityImportJournalStore::open(&paths).unwrap();
    let approved = approve(
        &record,
        plan.plan_sha256,
        &destination(&plan),
        &sources(&plan),
    )
    .unwrap();
    store.save(&approved).unwrap();
    drop(store);
    assert!(matches!(
        lock_operation(&paths),
        Err(EnsureStateError::CapacityImportInProgress { .. })
    ));
    assert!(matches!(
        crate::fleet_ensure::ops::retained_contract::check(&root, "local", "staging"),
        Err(
            crate::fleet_ensure::ops::retained_contract::RetainedContractError::State(
                EnsureStateError::CapacityImportInProgress { .. }
            )
        )
    ));
    let store = CapacityImportJournalStore::open(&paths).unwrap();
    assert_eq!(store.read().unwrap(), Some(approved));
    drop(store);
    std::fs::write(paths.plan.with_file_name("capacity-import.json"), b"{}").unwrap();
    assert!(matches!(
        lock_operation(&paths),
        Err(EnsureStateError::CapacityImportJournal { .. })
    ));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn capacity_import_reserves_journal_space_before_approving_any_handoff() {
    use crate::fleet_ensure::ops::EnsurePaths;
    use canic_core::ids::AppId;
    let root = crate::test_support::temp_dir("capacity-import-size");
    let paths = EnsurePaths::under(&root, "local", "staging");
    let store = CapacityImportJournalStore::open(&paths).unwrap();
    let mut plan = plan();
    plan.authority.fleet.app = AppId::from("a".repeat(5 * 1024 * 1024));
    let plan = prepare_review(plan.authority, plan.sources, root_budget()).unwrap();
    assert!(matches!(
        store.stage(plan),
        Err(CapacityImportJournalError::Integrity)
    ));
    assert!(store.read().unwrap().is_none());
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn capacity_import_handoff_retains_both_cycle_balances_across_restart() {
    let record = reserved();
    let mut source = sources(&record.plan).remove(0);
    let intent = prepare_handoff(&record, &source).unwrap();
    let issued = issue_handoff(&intent, source.binding.canister_id).unwrap();
    source.binding.controllers = issued.plan.transitional_controllers.clone();
    source.binding.canister_version += 1;
    source.cycles -= 50;
    source.reserved_cycles += 50;
    let applied = observe_handoff(&restart(&issued), &source).unwrap();
    let restored = restart(&applied);
    assert_eq!(restored.handoffs[0].before_reserved_cycles, Some(100));
    assert_eq!(restored.handoffs[0].after_reserved_cycles, Some(150));
    let mut json = serde_json::to_value(restored).unwrap();
    json["handoffs"][0]
        .as_object_mut()
        .unwrap()
        .remove("before_reserved_cycles");
    assert!(serde_json::from_value::<CapacityImportJournalRecord>(json).is_err());
}

// Pure journal tests supply classified wire evidence; transport authentication
// and real management effects have separate tests.
fn prepare_handoff(
    journal: &CapacityImportJournalRecord,
    observed: &CapacityImportSourceView,
) -> Result<CapacityImportJournalRecord, CapacityImportJournalError> {
    let request = crate::fleet_ensure::ops::capacity_import::transport::tests::request(
        &journal.plan,
        observed.binding.canister_id,
    );
    super::prepare_handoff(journal, observed, request)
}

fn observe_handoff(
    journal: &CapacityImportJournalRecord,
    observed: &CapacityImportSourceView,
) -> Result<CapacityImportJournalRecord, CapacityImportJournalError> {
    let handoff = journal
        .handoffs
        .iter()
        .find(|handoff| handoff.canister_id == observed.binding.canister_id)
        .unwrap();
    let completion = crate::fleet_ensure::ops::capacity_import::transport::tests::completion(
        &journal.plan,
        handoff.canister_id,
        handoff.request.as_ref().unwrap(),
    );
    super::observe_handoff(journal, observed, &completion)
}
