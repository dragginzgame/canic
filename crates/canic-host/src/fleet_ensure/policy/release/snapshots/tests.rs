use super::*;

fn fixture() -> (EnsureAction, LiveCanister) {
    (
        EnsureAction::DeleteSnapshot {
            name: "root".into(),
            principal: "source".into(),
            snapshot_id: "ab01".into(),
            expected_snapshots: vec!["ab01".into(), "cd02".into()],
            expected_module_sha256: Some("module".into()),
        },
        LiveCanister {
            principal: "source".into(),
            controllers: vec!["operator".into()],
            module_sha256: Some("module".into()),
            canister_version: Some(5),
            reinstall_required: false,
            root_owned_lifecycle: None,
            status: CanisterRuntimeStatus::Stopped,
            cycles: 100,
        },
    )
}

#[test]
fn snapshot_reconciliation_requires_exact_remaining_inventory() {
    let (action, live) = fixture();
    let before = vec!["ab01".into(), "cd02".into()];
    let after = vec!["cd02".into()];
    assert_eq!(
        reconcile(&action, "operator", Some(&live), &before),
        Ok(false)
    );
    assert_eq!(
        reconcile(&action, "operator", Some(&live), &after),
        Ok(true)
    );
    for inventory in [
        vec![],
        vec!["ab01".into()],
        vec!["cd02".into(), "ee03".into()],
        vec!["cd02".into(), "cd02".into()],
    ] {
        assert_eq!(
            reconcile(&action, "operator", Some(&live), &inventory),
            Err(SnapshotRemovalError::Inventory)
        );
    }
}

#[test]
fn snapshot_deletion_never_uses_running_shared_or_changed_custody() {
    let (action, live) = fixture();
    let snapshots = vec!["ab01".into(), "cd02".into()];
    let mut wrong = live.clone();
    wrong.controllers.push("old-root".into());
    let mut running = live.clone();
    running.status = CanisterRuntimeStatus::Running;
    let mut replaced = live.clone();
    replaced.module_sha256 = None;
    let mut substituted = live;
    substituted.principal = "other".into();
    for changed in [
        None,
        Some(wrong),
        Some(running),
        Some(replaced),
        Some(substituted),
    ] {
        assert_eq!(
            reconcile(&action, "operator", changed.as_ref(), &snapshots),
            Err(SnapshotRemovalError::Custody)
        );
    }
}

#[test]
fn malformed_snapshot_review_cannot_authorize_deletion() {
    let (mut action, live) = fixture();
    for inventory in [
        vec![],
        vec!["AB01".into()],
        vec!["ab01".into(), "ab01".into()],
        vec!["abc".into()],
        vec!["cd02".into(), "ab01".into()],
    ] {
        let EnsureAction::DeleteSnapshot {
            expected_snapshots, ..
        } = &mut action
        else {
            unreachable!()
        };
        *expected_snapshots = inventory.clone();
        assert_eq!(
            reconcile(&action, "operator", Some(&live), &inventory),
            Err(SnapshotRemovalError::Inventory)
        );
    }
}
