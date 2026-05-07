use super::*;

const ROOT: &str = "aaaaa-aa";
const CHILD: &str = "renrk-eyaaa-aaaaa-aaada-cai";
const HASH: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

// Build one valid manifest for validation tests.
fn valid_manifest() -> FleetBackupManifest {
    FleetBackupManifest {
        manifest_version: 1,
        backup_id: "fbk_test_001".to_string(),
        created_at: "2026-04-10T12:00:00Z".to_string(),
        tool: ToolMetadata {
            name: "canic".to_string(),
            version: "v1".to_string(),
        },
        source: SourceMetadata {
            environment: "local".to_string(),
            root_canister: ROOT.to_string(),
        },
        consistency: ConsistencySection {
            backup_units: vec![BackupUnit {
                unit_id: "core".to_string(),
                kind: BackupUnitKind::Subtree,
                roles: vec!["root".to_string(), "app".to_string()],
            }],
        },
        fleet: FleetSection {
            topology_hash_algorithm: "sha256".to_string(),
            topology_hash_input: "sorted(pid,parent_pid,role,module_hash)".to_string(),
            discovery_topology_hash: HASH.to_string(),
            pre_snapshot_topology_hash: HASH.to_string(),
            topology_hash: HASH.to_string(),
            members: vec![
                fleet_member("root", ROOT, None, IdentityMode::Fixed),
                fleet_member("app", CHILD, Some(ROOT), IdentityMode::Relocatable),
            ],
        },
        verification: VerificationPlan {
            fleet_checks: vec![VerificationCheck {
                kind: "status".to_string(),
                roles: Vec::new(),
            }],
            member_checks: Vec::new(),
        },
    }
}

#[test]
fn valid_manifest_passes_validation() {
    let manifest = valid_manifest();

    manifest.validate().expect("manifest should validate");
}

// Ensure snapshot checksum provenance stays canonical when present.
#[test]
fn invalid_snapshot_checksum_fails_validation() {
    let mut manifest = valid_manifest();
    manifest.fleet.members[0].source_snapshot.checksum = Some("not-a-sha".to_string());

    let err = manifest
        .validate()
        .expect_err("invalid snapshot checksum should fail");

    assert!(matches!(
        err,
        ManifestValidationError::InvalidHash("fleet.members[].source_snapshot.checksum")
    ));
}

// Build one valid fleet member for manifest validation tests.
fn fleet_member(
    role: &str,
    canister_id: &str,
    parent_canister_id: Option<&str>,
    identity_mode: IdentityMode,
) -> FleetMember {
    FleetMember {
        role: role.to_string(),
        canister_id: canister_id.to_string(),
        parent_canister_id: parent_canister_id.map(str::to_string),
        subnet_canister_id: Some(CHILD.to_string()),
        controller_hint: Some(ROOT.to_string()),
        identity_mode,
        verification_checks: vec![VerificationCheck {
            kind: "status".to_string(),
            roles: Vec::new(),
        }],
        source_snapshot: SourceSnapshot {
            snapshot_id: format!("snap-{role}"),
            module_hash: Some(HASH.to_string()),
            wasm_hash: Some(HASH.to_string()),
            code_version: Some("v0.30.0".to_string()),
            artifact_path: format!("artifacts/{role}"),
            checksum_algorithm: "sha256".to_string(),
            checksum: Some(HASH.to_string()),
        },
    }
}

#[test]
fn topology_hash_mismatch_fails_validation() {
    let mut manifest = valid_manifest();
    manifest.fleet.pre_snapshot_topology_hash =
        "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff".to_string();

    let err = manifest.validate().expect_err("mismatch should fail");

    assert!(matches!(
        err,
        ManifestValidationError::TopologyHashMismatch { .. }
    ));
}

#[test]
fn missing_member_verification_checks_fail_validation() {
    let mut manifest = valid_manifest();
    manifest.fleet.members[0].verification_checks.clear();

    let err = manifest
        .validate()
        .expect_err("missing member checks should fail");

    assert!(matches!(
        err,
        ManifestValidationError::MissingMemberVerificationChecks(_)
    ));
}

#[test]
fn backup_unit_roles_must_exist_in_fleet() {
    let mut manifest = valid_manifest();
    manifest.consistency.backup_units[0]
        .roles
        .push("missing-role".to_string());

    let err = manifest
        .validate()
        .expect_err("unknown backup unit role should fail");

    assert!(matches!(
        err,
        ManifestValidationError::UnknownBackupUnitRole { .. }
    ));
}

#[test]
fn backup_unit_ids_must_be_unique() {
    let mut manifest = valid_manifest();
    manifest
        .consistency
        .backup_units
        .push(manifest.consistency.backup_units[0].clone());

    let err = manifest
        .validate()
        .expect_err("duplicate unit IDs should fail");

    assert!(matches!(
        err,
        ManifestValidationError::DuplicateBackupUnitId(_)
    ));
}

#[test]
fn backup_unit_roles_must_be_unique() {
    let mut manifest = valid_manifest();
    manifest.consistency.backup_units[0]
        .roles
        .push("root".to_string());

    let err = manifest
        .validate()
        .expect_err("duplicate backup unit role should fail");

    assert!(matches!(
        err,
        ManifestValidationError::DuplicateBackupUnitRole { .. }
    ));
}

#[test]
fn every_fleet_role_must_be_covered_by_a_backup_unit() {
    let mut manifest = valid_manifest();
    manifest.consistency.backup_units[0].kind = BackupUnitKind::Single;
    manifest.consistency.backup_units[0].roles = vec!["root".to_string()];

    let err = manifest
        .validate()
        .expect_err("uncovered app role should fail");

    assert!(matches!(
        err,
        ManifestValidationError::BackupUnitCoverageMissingRole { .. }
    ));
}

#[test]
fn fleet_verification_roles_must_exist_in_fleet() {
    let mut manifest = valid_manifest();
    manifest.verification.fleet_checks[0]
        .roles
        .push("missing-role".to_string());

    let err = manifest
        .validate()
        .expect_err("unknown fleet verification role should fail");

    assert!(matches!(
        err,
        ManifestValidationError::UnknownVerificationRole { .. }
    ));
}

#[test]
fn member_verification_check_roles_must_exist_in_fleet() {
    let mut manifest = valid_manifest();
    manifest.fleet.members[0].verification_checks[0]
        .roles
        .push("missing-role".to_string());

    let err = manifest
        .validate()
        .expect_err("unknown member verification check role should fail");

    assert!(matches!(
        err,
        ManifestValidationError::UnknownVerificationRole { .. }
    ));
}

#[test]
fn verification_check_roles_must_be_unique() {
    let mut manifest = valid_manifest();
    manifest.verification.fleet_checks[0]
        .roles
        .push("root".to_string());
    manifest.verification.fleet_checks[0]
        .roles
        .push("root".to_string());

    let err = manifest
        .validate()
        .expect_err("duplicate verification role filter should fail");

    assert!(matches!(
        err,
        ManifestValidationError::DuplicateVerificationCheckRole { .. }
    ));
}

#[test]
fn member_verification_group_roles_must_exist_in_fleet() {
    let mut manifest = valid_manifest();
    manifest
        .verification
        .member_checks
        .push(MemberVerificationChecks {
            role: "missing-role".to_string(),
            checks: vec![VerificationCheck {
                kind: "status".to_string(),
                roles: Vec::new(),
            }],
        });

    let err = manifest
        .validate()
        .expect_err("unknown member verification role should fail");

    assert!(matches!(
        err,
        ManifestValidationError::UnknownVerificationRole { .. }
    ));
}

#[test]
fn member_verification_group_roles_must_be_unique() {
    let mut manifest = valid_manifest();
    manifest
        .verification
        .member_checks
        .push(member_verification_checks("root"));
    manifest
        .verification
        .member_checks
        .push(member_verification_checks("root"));

    let err = manifest
        .validate()
        .expect_err("duplicate member verification role should fail");

    assert!(matches!(
        err,
        ManifestValidationError::DuplicateMemberVerificationRole(_)
    ));
}

#[test]
fn nested_member_verification_roles_must_exist_in_fleet() {
    let mut manifest = valid_manifest();
    let mut checks = member_verification_checks("root");
    checks.checks[0].roles.push("missing-role".to_string());
    manifest.verification.member_checks.push(checks);

    let err = manifest
        .validate()
        .expect_err("unknown nested verification role should fail");

    assert!(matches!(
        err,
        ManifestValidationError::UnknownVerificationRole { .. }
    ));
}

#[test]
fn subtree_unit_must_be_closed_under_descendants() {
    let mut manifest = valid_manifest();
    manifest.consistency.backup_units[0].kind = BackupUnitKind::Subtree;
    manifest.consistency.backup_units[0].roles = vec!["root".to_string()];

    let err = manifest
        .validate()
        .expect_err("subtree unit omitting app child should fail");

    assert!(matches!(
        err,
        ManifestValidationError::SubtreeBackupUnitMissingDescendant { .. }
    ));
}

#[test]
fn subtree_unit_must_be_connected() {
    let mut manifest = valid_manifest();
    manifest.fleet.members.push(fleet_member(
        "worker",
        "r7inp-6aaaa-aaaaa-aaabq-cai",
        None,
        IdentityMode::Relocatable,
    ));
    manifest.consistency.backup_units[0].kind = BackupUnitKind::Subtree;
    manifest.consistency.backup_units[0].roles = vec!["app".to_string(), "worker".to_string()];

    let err = manifest
        .validate()
        .expect_err("disconnected subtree unit should fail");

    assert!(matches!(
        err,
        ManifestValidationError::SubtreeBackupUnitNotConnected { .. }
    ));
}

#[test]
fn manifest_round_trips_through_json() {
    let manifest = valid_manifest();

    let encoded = serde_json::to_string(&manifest).expect("serialize manifest");
    let decoded: FleetBackupManifest =
        serde_json::from_str(&encoded).expect("deserialize manifest");

    decoded
        .validate()
        .expect("decoded manifest should validate");
}

// Build one role-scoped verification group for validation tests.
fn member_verification_checks(role: &str) -> MemberVerificationChecks {
    MemberVerificationChecks {
        role: role.to_string(),
        checks: vec![VerificationCheck {
            kind: "status".to_string(),
            roles: Vec::new(),
        }],
    }
}
