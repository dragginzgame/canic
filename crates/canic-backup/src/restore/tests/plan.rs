use super::*;

// Ensure in-place restore planning sorts parent before child.
#[test]
fn in_place_plan_orders_parent_before_child() {
    let manifest = valid_manifest(IdentityMode::Relocatable);

    let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
    let ordered = plan.ordered_members();

    assert_eq!(plan.backup_id, "fbk_test_001");
    assert_eq!(plan.source_environment, "local");
    assert_eq!(plan.source_root_canister, ROOT);
    assert_eq!(plan.topology_hash, HASH);
    assert_eq!(plan.member_count, 2);
    assert_eq!(plan.identity_summary.fixed_members, 1);
    assert_eq!(plan.identity_summary.relocatable_members, 1);
    assert_eq!(plan.identity_summary.in_place_members, 2);
    assert_eq!(plan.identity_summary.mapped_members, 0);
    assert_eq!(plan.identity_summary.remapped_members, 0);
    assert!(plan.verification_summary.verification_required);
    assert!(plan.verification_summary.all_members_have_checks);
    assert!(plan.readiness_summary.ready);
    assert!(plan.readiness_summary.reasons.is_empty());
    assert_eq!(plan.verification_summary.fleet_checks, 0);
    assert_eq!(plan.verification_summary.member_check_groups, 0);
    assert_eq!(plan.verification_summary.member_checks, 2);
    assert_eq!(plan.verification_summary.members_with_checks, 2);
    assert_eq!(plan.verification_summary.total_checks, 2);
    assert_eq!(plan.ordering_summary.ordered_members, 2);
    assert_eq!(plan.ordering_summary.dependency_free_members, 1);
    assert_eq!(plan.ordering_summary.parent_edges, 1);
    assert_eq!(ordered[0].member_order, 0);
    assert_eq!(ordered[1].member_order, 1);
    assert_eq!(ordered[0].source_canister, ROOT);
    assert_eq!(ordered[1].source_canister, CHILD);
    assert_eq!(
        ordered[1].ordering_dependency,
        Some(RestoreOrderingDependency {
            source_canister: ROOT.to_string(),
            target_canister: ROOT.to_string(),
            relationship: RestoreOrderingRelationship::ParentBeforeChild,
        })
    );
}

// Ensure fixed identities cannot be remapped.
#[test]
fn fixed_identity_member_cannot_be_remapped() {
    let manifest = valid_manifest(IdentityMode::Fixed);
    let mapping = RestoreMapping {
        members: vec![
            RestoreMappingEntry {
                source_canister: ROOT.to_string(),
                target_canister: ROOT.to_string(),
            },
            RestoreMappingEntry {
                source_canister: CHILD.to_string(),
                target_canister: TARGET.to_string(),
            },
        ],
    };

    let err = RestorePlanner::plan(&manifest, Some(&mapping))
        .expect_err("fixed member remap should fail");

    assert!(matches!(err, RestorePlanError::FixedIdentityRemap { .. }));
}

// Ensure relocatable identities may be mapped when all members are covered.
#[test]
fn relocatable_member_can_be_mapped() {
    let manifest = valid_manifest(IdentityMode::Relocatable);
    let mapping = RestoreMapping {
        members: vec![
            RestoreMappingEntry {
                source_canister: ROOT.to_string(),
                target_canister: ROOT.to_string(),
            },
            RestoreMappingEntry {
                source_canister: CHILD.to_string(),
                target_canister: TARGET.to_string(),
            },
        ],
    };

    let plan = RestorePlanner::plan(&manifest, Some(&mapping)).expect("plan should build");
    let child = plan
        .ordered_members()
        .into_iter()
        .find(|member| member.source_canister == CHILD)
        .expect("child member should be planned");

    assert_eq!(plan.identity_summary.fixed_members, 1);
    assert_eq!(plan.identity_summary.relocatable_members, 1);
    assert_eq!(plan.identity_summary.in_place_members, 1);
    assert_eq!(plan.identity_summary.mapped_members, 2);
    assert_eq!(plan.identity_summary.remapped_members, 1);
    assert_eq!(child.target_canister, TARGET);
    assert_eq!(child.parent_target_canister, Some(ROOT.to_string()));
}

// Ensure restore plans carry enough metadata for operator dry-runs.
#[test]
fn plan_members_include_snapshot_and_verification_metadata() {
    let manifest = valid_manifest(IdentityMode::Relocatable);

    let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
    let root = plan
        .ordered_members()
        .into_iter()
        .find(|member| member.source_canister == ROOT)
        .expect("root member should be planned");

    assert_eq!(root.identity_mode, IdentityMode::Fixed);
    assert_eq!(root.verification_checks[0].kind, "status");
    assert_eq!(root.source_snapshot.snapshot_id, "snap-root");
    assert_eq!(root.source_snapshot.artifact_path, "artifacts/root");
}

// Ensure restore plans make mapping mode explicit.
#[test]
fn plan_includes_mapping_summary() {
    let manifest = valid_manifest(IdentityMode::Relocatable);
    let in_place = RestorePlanner::plan(&manifest, None).expect("plan should build");

    assert!(!in_place.identity_summary.mapping_supplied);
    assert!(!in_place.identity_summary.all_sources_mapped);
    assert_eq!(in_place.identity_summary.mapped_members, 0);

    let mapping = RestoreMapping {
        members: vec![
            RestoreMappingEntry {
                source_canister: ROOT.to_string(),
                target_canister: ROOT.to_string(),
            },
            RestoreMappingEntry {
                source_canister: CHILD.to_string(),
                target_canister: TARGET.to_string(),
            },
        ],
    };
    let mapped = RestorePlanner::plan(&manifest, Some(&mapping)).expect("plan should build");

    assert!(mapped.identity_summary.mapping_supplied);
    assert!(mapped.identity_summary.all_sources_mapped);
    assert_eq!(mapped.identity_summary.mapped_members, 2);
    assert_eq!(mapped.identity_summary.remapped_members, 1);
}

// Ensure restore plans summarize snapshot provenance completeness.
#[test]
fn plan_includes_snapshot_summary() {
    let mut manifest = valid_manifest(IdentityMode::Relocatable);
    manifest.fleet.members[1].source_snapshot.module_hash = None;
    manifest.fleet.members[1].source_snapshot.wasm_hash = None;
    manifest.fleet.members[1].source_snapshot.checksum = None;

    let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");

    assert!(!plan.snapshot_summary.all_members_have_module_hash);
    assert!(!plan.snapshot_summary.all_members_have_wasm_hash);
    assert!(plan.snapshot_summary.all_members_have_code_version);
    assert!(!plan.snapshot_summary.all_members_have_checksum);
    assert_eq!(plan.snapshot_summary.members_with_module_hash, 1);
    assert_eq!(plan.snapshot_summary.members_with_wasm_hash, 1);
    assert_eq!(plan.snapshot_summary.members_with_code_version, 2);
    assert_eq!(plan.snapshot_summary.members_with_checksum, 1);
    assert!(!plan.readiness_summary.ready);
    assert_eq!(
        plan.readiness_summary.reasons,
        ["missing-snapshot-checksum"]
    );
}

// Ensure restore plans summarize manifest-level verification work.
#[test]
fn plan_includes_verification_summary() {
    let mut manifest = valid_manifest(IdentityMode::Relocatable);
    manifest.verification.fleet_checks.push(VerificationCheck {
        kind: "status".to_string(),
        roles: Vec::new(),
    });
    manifest
        .verification
        .member_checks
        .push(MemberVerificationChecks {
            role: "app".to_string(),
            checks: vec![VerificationCheck {
                kind: "status".to_string(),
                roles: Vec::new(),
            }],
        });

    let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");

    assert!(plan.verification_summary.verification_required);
    assert!(plan.verification_summary.all_members_have_checks);
    let app = plan
        .ordered_members()
        .into_iter()
        .find(|member| member.role == "app")
        .expect("app member should be planned");
    assert_eq!(app.verification_checks.len(), 2);
    assert_eq!(plan.fleet_verification_checks.len(), 1);
    assert_eq!(plan.fleet_verification_checks[0].kind, "status");
    assert_eq!(plan.verification_summary.fleet_checks, 1);
    assert_eq!(plan.verification_summary.member_check_groups, 1);
    assert_eq!(plan.verification_summary.member_checks, 3);
    assert_eq!(plan.verification_summary.members_with_checks, 2);
    assert_eq!(plan.verification_summary.total_checks, 4);
}

// Ensure restore plans summarize the concrete operation counts automation will schedule.
#[test]
fn plan_includes_operation_summary() {
    let manifest = valid_manifest(IdentityMode::Relocatable);

    let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");

    assert_eq!(plan.operation_summary.planned_snapshot_uploads, 2);
    assert_eq!(plan.operation_summary.planned_snapshot_loads, 2);
    assert_eq!(plan.operation_summary.planned_verification_checks, 2);
    assert_eq!(plan.operation_summary.planned_operations, 6);
}

// Ensure role-level verification checks are counted once per matching member.
#[test]
fn plan_expands_role_verification_checks_per_matching_member() {
    let mut manifest = valid_manifest(IdentityMode::Relocatable);
    manifest.fleet.members.push(fleet_member(
        "app",
        CHILD_TWO,
        Some(ROOT),
        IdentityMode::Relocatable,
    ));
    manifest
        .verification
        .member_checks
        .push(MemberVerificationChecks {
            role: "app".to_string(),
            checks: vec![VerificationCheck {
                kind: "status".to_string(),
                roles: Vec::new(),
            }],
        });

    let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");

    assert_eq!(plan.verification_summary.fleet_checks, 0);
    assert_eq!(plan.verification_summary.member_check_groups, 1);
    assert_eq!(plan.verification_summary.member_checks, 5);
    assert_eq!(plan.verification_summary.members_with_checks, 3);
    assert_eq!(plan.verification_summary.total_checks, 5);
}

// Ensure member verification role filters control concrete restore checks.
#[test]
fn plan_applies_member_verification_role_filters() {
    let mut manifest = valid_manifest(IdentityMode::Relocatable);
    manifest.fleet.members[0]
        .verification_checks
        .push(VerificationCheck {
            kind: "status".to_string(),
            roles: vec!["root".to_string()],
        });
    manifest
        .verification
        .member_checks
        .push(MemberVerificationChecks {
            role: "app".to_string(),
            checks: vec![
                VerificationCheck {
                    kind: "status".to_string(),
                    roles: vec!["app".to_string()],
                },
                VerificationCheck {
                    kind: "status".to_string(),
                    roles: vec!["root".to_string()],
                },
            ],
        });

    let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
    let app = plan
        .ordered_members()
        .into_iter()
        .find(|member| member.role == "app")
        .expect("app member should be planned");
    let dry_run = RestoreApplyDryRun::from_plan(&plan);
    let app_verification_kinds = dry_run
        .operations
        .iter()
        .filter(|operation| {
            operation.source_canister == CHILD
                && operation.operation == RestoreApplyOperationKind::VerifyMember
        })
        .filter_map(|operation| operation.verification_kind.as_deref())
        .collect::<Vec<_>>();

    assert_eq!(app.verification_checks.len(), 2);
    assert_eq!(
        app.verification_checks
            .iter()
            .map(|check| check.kind.as_str())
            .collect::<Vec<_>>(),
        ["status", "status"]
    );
    assert_eq!(plan.verification_summary.member_checks, 3);
    assert_eq!(plan.verification_summary.total_checks, 3);
    assert_eq!(dry_run.rendered_operations, 7);
    assert_eq!(app_verification_kinds, ["status", "status"]);
}

// Ensure mapped restores must cover every source member.
#[test]
fn mapped_restore_requires_complete_mapping() {
    let manifest = valid_manifest(IdentityMode::Relocatable);
    let mapping = RestoreMapping {
        members: vec![RestoreMappingEntry {
            source_canister: ROOT.to_string(),
            target_canister: ROOT.to_string(),
        }],
    };

    let err = RestorePlanner::plan(&manifest, Some(&mapping))
        .expect_err("incomplete mapping should fail");

    assert!(matches!(err, RestorePlanError::MissingMappingSource(_)));
}

// Ensure mappings cannot silently include canisters outside the manifest.
#[test]
fn mapped_restore_rejects_unknown_mapping_sources() {
    let manifest = valid_manifest(IdentityMode::Relocatable);
    let unknown = "rdmx6-jaaaa-aaaaa-aaadq-cai";
    let mapping = RestoreMapping {
        members: vec![
            RestoreMappingEntry {
                source_canister: ROOT.to_string(),
                target_canister: ROOT.to_string(),
            },
            RestoreMappingEntry {
                source_canister: CHILD.to_string(),
                target_canister: TARGET.to_string(),
            },
            RestoreMappingEntry {
                source_canister: unknown.to_string(),
                target_canister: unknown.to_string(),
            },
        ],
    };

    let err = RestorePlanner::plan(&manifest, Some(&mapping))
        .expect_err("unknown mapping source should fail");

    assert!(matches!(err, RestorePlanError::UnknownMappingSource(_)));
}

// Ensure duplicate target mappings fail before a plan is produced.
#[test]
fn duplicate_mapping_targets_fail_validation() {
    let manifest = valid_manifest(IdentityMode::Relocatable);
    let mapping = RestoreMapping {
        members: vec![
            RestoreMappingEntry {
                source_canister: ROOT.to_string(),
                target_canister: ROOT.to_string(),
            },
            RestoreMappingEntry {
                source_canister: CHILD.to_string(),
                target_canister: ROOT.to_string(),
            },
        ],
    };

    let err =
        RestorePlanner::plan(&manifest, Some(&mapping)).expect_err("duplicate targets should fail");

    assert!(matches!(err, RestorePlanError::DuplicateMappingTarget(_)));
}
