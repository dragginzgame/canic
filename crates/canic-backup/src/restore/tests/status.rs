use super::*;

#[test]
fn restore_status_starts_all_members_as_planned() {
    let manifest = valid_manifest(IdentityMode::Relocatable);

    let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
    let status = RestoreStatus::from_plan(&plan);

    assert_eq!(status.status_version, 1);
    assert_eq!(status.backup_id.as_str(), plan.backup_id.as_str());
    assert_eq!(
        status.source_environment.as_str(),
        plan.source_environment.as_str()
    );
    assert_eq!(
        status.source_root_canister.as_str(),
        plan.source_root_canister.as_str()
    );
    assert_eq!(status.topology_hash.as_str(), plan.topology_hash.as_str());
    assert!(status.ready);
    assert!(status.readiness_reasons.is_empty());
    assert!(status.verification_required);
    assert_eq!(status.member_count, 2);
    assert_eq!(status.phase_count, 1);
    assert_eq!(status.planned_snapshot_uploads, 2);
    assert_eq!(status.planned_snapshot_loads, 2);
    assert_eq!(status.planned_code_reinstalls, 0);
    assert_eq!(status.planned_verification_checks, 2);
    assert_eq!(status.planned_operations, 6);
    assert_eq!(status.phases.len(), 1);
    assert_eq!(status.phases[0].restore_group, 1);
    assert_eq!(status.phases[0].members.len(), 2);
    assert_eq!(
        status.phases[0].members[0].state,
        RestoreMemberState::Planned
    );
    assert_eq!(status.phases[0].members[0].source_canister, ROOT);
    assert_eq!(status.phases[0].members[0].target_canister, ROOT);
    assert_eq!(status.phases[0].members[0].snapshot_id, "snap-root");
    assert_eq!(status.phases[0].members[0].artifact_path, "artifacts/root");
    assert_eq!(
        status.phases[0].members[1].state,
        RestoreMemberState::Planned
    );
    assert_eq!(status.phases[0].members[1].source_canister, CHILD);
}
