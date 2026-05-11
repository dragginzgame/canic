use super::*;

const ROOT: &str = "aaaaa-aa";
const APP: &str = "renrk-eyaaa-aaaaa-aaada-cai";
const WORKER: &str = "rno2w-sqaaa-aaaaa-aaacq-cai";
const OTHER_WORKER: &str = "rdmx6-jaaaa-aaaaa-aaadq-cai";
const PREFLIGHT_ID: &str = "preflight-001";
const VALIDATED_AT: &str = "unix:100";
const EXPIRES_AT: &str = "unix:200";
const AS_OF: &str = "unix:150";

// Ensure a normal subtree plan can prove authority before mutation.
#[test]
fn validates_subtree_plan_with_authority_preflights() {
    let plan = subtree_plan();

    plan.validate().expect("valid subtree plan");
    plan.validate_for_execution()
        .expect("executable subtree plan");
}

// Ensure normal backups cannot silently include root.
#[test]
fn rejects_root_in_normal_scope() {
    let mut plan = subtree_plan();
    plan.root_included = true;
    plan.targets.push(BackupTarget {
        canister_id: ROOT.to_string(),
        role: Some("root".to_string()),
        parent_canister_id: None,
        depth: 0,
        control_authority: proven_root_control(),
        snapshot_read_authority: proven_root_read(),
        identity_mode: IdentityMode::Fixed,
        expected_module_hash: None,
    });

    let err = plan
        .validate()
        .expect_err("root should require maintenance");

    assert!(matches!(
        err,
        BackupPlanError::RootIncludedWithoutMaintenance
    ));
}

// Ensure declared authority is still a valid planning/dry-run artifact.
#[test]
fn planning_allows_declared_authority() {
    let mut plan = subtree_plan();
    plan.snapshot_read_authority =
        SnapshotReadAuthority::root_configured_read(AuthorityEvidence::Declared);
    plan.targets[0].control_authority =
        ControlAuthority::root_controller(AuthorityEvidence::Declared);
    plan.targets[0].snapshot_read_authority =
        SnapshotReadAuthority::root_configured_read(AuthorityEvidence::Declared);

    plan.validate().expect("declared authority can plan");
    let err = plan
        .validate_for_execution()
        .expect_err("declared authority cannot execute");

    assert!(matches!(
        err,
        BackupPlanError::UnprovenControlAuthority(canister) if canister == APP
    ));
}

// Ensure unknown authority can be represented for dry-run output.
#[test]
fn planning_allows_unknown_authority() {
    let mut plan = subtree_plan();
    plan.snapshot_read_authority = SnapshotReadAuthority::unknown();
    plan.targets[0].control_authority = ControlAuthority::unknown();
    plan.targets[0].snapshot_read_authority = SnapshotReadAuthority::unknown();

    plan.validate().expect("unknown authority can plan");
    let err = plan
        .validate_for_execution()
        .expect_err("unknown authority cannot execute");

    assert!(matches!(
        err,
        BackupPlanError::UnprovenControlAuthority(canister) if canister == APP
    ));
}

// Ensure unproven control authority fails before any execution can happen.
#[test]
fn rejects_unproven_control_authority() {
    let mut plan = subtree_plan();
    plan.targets[0].control_authority =
        ControlAuthority::root_controller(AuthorityEvidence::Declared);

    let err = plan
        .validate_for_execution()
        .expect_err("control authority should be proven");

    assert!(matches!(
        err,
        BackupPlanError::UnprovenControlAuthority(canister) if canister == APP
    ));
}

// Ensure snapshot read authority is a first-class preflight, not a late download error.
#[test]
fn rejects_unproven_snapshot_read_authority() {
    let mut plan = subtree_plan();
    plan.targets[0].snapshot_read_authority =
        SnapshotReadAuthority::root_configured_read(AuthorityEvidence::Declared);

    let err = plan
        .validate_for_execution()
        .expect_err("snapshot read authority should be proven");

    assert!(matches!(
        err,
        BackupPlanError::UnprovenTargetSnapshotReadAuthority(canister) if canister == APP
    ));
}

// Ensure mutations cannot be planned before authority and quiescence checks.
#[test]
fn rejects_mutation_before_preflights() {
    let mut plan = subtree_plan();
    let stop = plan.phases.remove(4);
    plan.phases.insert(0, stop);
    reset_phase_order(&mut plan.phases);

    let err = plan
        .validate()
        .expect_err("mutation before preflight should reject");

    assert!(matches!(
        err,
        BackupPlanError::MutationBeforePreflight { operation_id }
            if operation_id == "stop-app"
    ));
}

// Ensure whole non-root-fleet plans do not pretend to have a unique subtree root.
#[test]
fn rejects_non_root_fleet_with_selected_root() {
    let mut plan = subtree_plan();
    plan.selected_scope_kind = BackupScopeKind::NonRootFleet;

    let err = plan
        .validate()
        .expect_err("non-root fleet scope should not name one root");

    assert!(matches!(err, BackupPlanError::NonRootFleetHasSelectedRoot));
}

// Ensure journals can rely on stable contiguous operation ordering.
#[test]
fn rejects_operation_order_mismatch() {
    let mut plan = subtree_plan();
    plan.phases[1].order = 42;

    let err = plan
        .validate()
        .expect_err("operation order mismatch should reject");

    assert!(matches!(
        err,
        BackupPlanError::OperationOrderMismatch { operation_id, order, expected }
            if operation_id == "validate-control" && order == 42 && expected == 1
    ));
}

// Ensure registry planning produces root-stays-running subtree phases.
#[test]
fn builds_subtree_plan_from_registry() {
    let plan = build_backup_plan(BackupPlanBuildInput {
        selected_canister_id: Some(APP.to_string()),
        selected_scope_kind: BackupScopeKind::Subtree,
        registry: &registry(),
        ..plan_input()
    })
    .expect("build subtree plan");

    assert_eq!(plan.selected_subtree_root.as_deref(), Some(APP));
    assert!(!plan.root_included);
    assert_eq!(
        plan.targets
            .iter()
            .map(|target| target.canister_id.as_str())
            .collect::<Vec<_>>(),
        vec![APP, WORKER]
    );
    assert!(
        plan.phases
            .iter()
            .all(|phase| phase.target_canister_id.as_deref() != Some(ROOT))
    );
    assert_operation_order(
        &plan,
        &[
            ("validate-topology", None),
            ("validate-control-authority", None),
            ("validate-snapshot-read-authority", None),
            ("validate-quiescence-policy", None),
            ("stop-renrk-eyaaa-aaaaa-aaada-cai", Some(APP)),
            ("stop-rno2w-sqaaa-aaaaa-aaacq-cai", Some(WORKER)),
            ("snapshot-renrk-eyaaa-aaaaa-aaada-cai", Some(APP)),
            ("snapshot-rno2w-sqaaa-aaaaa-aaacq-cai", Some(WORKER)),
            ("start-rno2w-sqaaa-aaaaa-aaacq-cai", Some(WORKER)),
            ("start-renrk-eyaaa-aaaaa-aaada-cai", Some(APP)),
        ],
    );
}

// Ensure non-root fleet scope expands every managed member while leaving root running.
#[test]
fn builds_non_root_fleet_plan_without_root_target() {
    let plan = build_backup_plan(BackupPlanBuildInput {
        selected_canister_id: None,
        selected_scope_kind: BackupScopeKind::NonRootFleet,
        registry: &registry(),
        ..plan_input()
    })
    .expect("build non-root fleet plan");

    assert_eq!(plan.selected_subtree_root, None);
    assert!(!plan.root_included);
    assert_eq!(
        plan.targets
            .iter()
            .map(|target| target.canister_id.as_str())
            .collect::<Vec<_>>(),
        vec![APP, WORKER]
    );
}

// Ensure normal planning rejects a root subtree before generating mutating phases.
#[test]
fn builder_rejects_root_subtree_without_maintenance() {
    let err = build_backup_plan(BackupPlanBuildInput {
        selected_canister_id: Some(ROOT.to_string()),
        selected_scope_kind: BackupScopeKind::Subtree,
        registry: &registry(),
        ..plan_input()
    })
    .expect_err("normal root subtree should reject");

    assert!(matches!(
        err,
        BackupPlanError::RootIncludedWithoutMaintenance
    ));
}

// Ensure selectors can target explicit principals or unambiguous roles.
#[test]
fn resolves_principal_and_role_selectors() {
    let registry = registry();

    assert_eq!(
        resolve_backup_selector(&registry, APP).expect("resolve principal"),
        APP
    );
    assert_eq!(
        resolve_backup_selector(&registry, "app").expect("resolve role"),
        APP
    );
}

// Ensure role selectors fail closed when a role is not unique.
#[test]
fn rejects_ambiguous_role_selector() {
    let mut registry = registry();
    registry.push(RegistryEntry {
        pid: OTHER_WORKER.to_string(),
        role: Some("worker".to_string()),
        kind: Some("replica".to_string()),
        parent_pid: Some(APP.to_string()),
        module_hash: None,
    });

    let err =
        resolve_backup_selector(&registry, "worker").expect_err("ambiguous role should reject");

    assert!(matches!(
        err,
        BackupPlanError::AmbiguousSelector { selector, matches }
            if selector == "worker" && matches == vec![WORKER.to_string(), OTHER_WORKER.to_string()]
    ));
}

// Ensure selectors never silently target missing topology nodes.
#[test]
fn rejects_unknown_selector() {
    let err =
        resolve_backup_selector(&registry(), "missing-role").expect_err("missing selector rejects");

    assert!(
        matches!(err, BackupPlanError::UnknownSelector(selector) if selector == "missing-role")
    );
}

// Ensure authority receipts are the bridge from dry-run planning to execution.
#[test]
fn authority_receipts_upgrade_declared_plan_for_execution() {
    let mut plan = subtree_plan();
    plan.targets[0].control_authority =
        ControlAuthority::root_controller(AuthorityEvidence::Declared);
    plan.targets[0].snapshot_read_authority =
        SnapshotReadAuthority::root_configured_read(AuthorityEvidence::Declared);

    plan.apply_authority_preflight_receipts(
        PREFLIGHT_ID,
        &[control_receipt(APP, proven_root_control())],
        &[snapshot_read_receipt(APP, proven_root_read())],
        AS_OF,
    )
    .expect("apply authority receipts");

    assert_eq!(plan.targets[0].control_authority, proven_root_control());
    assert_eq!(plan.targets[0].snapshot_read_authority, proven_root_read());
    plan.validate_for_execution()
        .expect("receipts make plan executable");
}

// Ensure control authority preflight must cover every selected target.
#[test]
fn control_authority_receipts_must_cover_all_targets() {
    let mut plan = build_backup_plan(BackupPlanBuildInput {
        selected_canister_id: Some(APP.to_string()),
        selected_scope_kind: BackupScopeKind::Subtree,
        registry: &registry(),
        ..plan_input()
    })
    .expect("build subtree plan");

    let err = plan
        .apply_control_authority_receipts(
            PREFLIGHT_ID,
            &[control_receipt(APP, proven_root_control())],
            AS_OF,
        )
        .expect_err("missing worker receipt rejects");

    assert!(matches!(
        err,
        BackupPlanError::MissingControlAuthorityReceipt(canister) if canister == WORKER
    ));
}

// Ensure receipts cannot prove authority for canisters outside the plan.
#[test]
fn authority_receipts_reject_unknown_targets() {
    let mut plan = subtree_plan();

    let err = plan
        .apply_control_authority_receipts(
            PREFLIGHT_ID,
            &[control_receipt(WORKER, proven_root_control())],
            AS_OF,
        )
        .expect_err("unknown target receipt rejects");

    assert!(matches!(
        err,
        BackupPlanError::UnknownAuthorityReceiptTarget(canister) if canister == WORKER
    ));
}

// Ensure receipt application does not treat declarations as execution proof.
#[test]
fn authority_receipts_reject_unproven_authority() {
    let mut plan = subtree_plan();

    let err = plan
        .apply_control_authority_receipts(
            PREFLIGHT_ID,
            &[control_receipt(
                APP,
                ControlAuthority::root_controller(AuthorityEvidence::Declared),
            )],
            AS_OF,
        )
        .expect_err("declared receipt rejects");

    assert!(matches!(
        err,
        BackupPlanError::UnprovenControlAuthority(canister) if canister == APP
    ));
}

// Ensure root-coordinated plans cannot be upgraded by operator-only proof.
#[test]
fn root_controller_plans_require_root_controller_receipts() {
    let mut plan = subtree_plan();

    let err = plan
        .apply_control_authority_receipts(
            PREFLIGHT_ID,
            &[control_receipt(
                APP,
                ControlAuthority::operator_controller(AuthorityEvidence::Proven),
            )],
            AS_OF,
        )
        .expect_err("operator controller does not satisfy root controller plan");

    assert!(matches!(
        err,
        BackupPlanError::MissingRootController(canister) if canister == APP
    ));
}

// Ensure control authority preflights have a stable typed request shape.
#[test]
fn builds_control_authority_preflight_request() {
    let plan = subtree_plan();
    let request = plan.control_authority_preflight_request();

    assert_eq!(request.plan_id, "plan-001");
    assert_eq!(request.run_id, "run-001");
    assert_eq!(request.root_canister_id, ROOT);
    assert!(request.requires_root_controller);
    assert_eq!(request.targets.len(), 1);
    assert_eq!(request.targets[0].canister_id, APP);
    assert_eq!(request.targets[0].role.as_deref(), Some("app"));
    assert_eq!(request.targets[0].declared_authority, proven_root_control());
}

// Ensure snapshot read preflights have a stable typed request shape.
#[test]
fn builds_snapshot_read_authority_preflight_request() {
    let plan = subtree_plan();
    let request = plan.snapshot_read_authority_preflight_request();

    assert_eq!(request.plan_id, "plan-001");
    assert_eq!(request.run_id, "run-001");
    assert_eq!(request.root_canister_id, ROOT);
    assert_eq!(request.targets.len(), 1);
    assert_eq!(request.targets[0].canister_id, APP);
    assert_eq!(request.targets[0].role.as_deref(), Some("app"));
    assert_eq!(request.targets[0].declared_authority, proven_root_read());
}

// Ensure topology preflights have a stable typed request shape.
#[test]
fn builds_topology_preflight_request() {
    let plan = subtree_plan();
    let request = plan.topology_preflight_request();

    assert_eq!(request.plan_id, "plan-001");
    assert_eq!(request.run_id, "run-001");
    assert_eq!(request.selected_subtree_root.as_deref(), Some(APP));
    assert_eq!(request.selected_scope_kind, BackupScopeKind::Subtree);
    assert_eq!(
        request.topology_hash_before_quiesce,
        plan.topology_hash_before_quiesce
    );
    assert_eq!(request.targets.len(), 1);
    assert_eq!(request.targets[0].canister_id, APP);
    assert_eq!(request.targets[0].parent_canister_id.as_deref(), Some(ROOT));
    assert_eq!(request.targets[0].depth, 1);
}

// Ensure quiescence preflights have a stable typed request shape.
#[test]
fn builds_quiescence_preflight_request() {
    let plan = subtree_plan();
    let request = plan.quiescence_preflight_request();

    assert_eq!(request.plan_id, "plan-001");
    assert_eq!(request.run_id, "run-001");
    assert_eq!(request.selected_subtree_root.as_deref(), Some(APP));
    assert_eq!(request.quiescence_policy, QuiescencePolicy::RootCoordinated);
    assert_eq!(request.targets.len(), 1);
    assert_eq!(request.targets[0].canister_id, APP);
    assert_eq!(request.targets[0].role.as_deref(), Some("app"));
}

// Ensure topology and quiescence receipts gate mutating execution preflights.
#[test]
fn validates_execution_preflight_receipts() {
    let plan = subtree_plan();

    plan.validate_execution_preflight_receipts(
        &topology_receipt(&plan),
        &quiescence_receipt(&plan),
        PREFLIGHT_ID,
        AS_OF,
    )
    .expect("valid execution preflights");
}

// Ensure the full preflight bundle upgrades authority and validates execution gates.
#[test]
fn applies_execution_preflight_receipt_bundle() {
    let mut plan = subtree_plan();
    plan.targets[0].control_authority =
        ControlAuthority::root_controller(AuthorityEvidence::Declared);
    plan.targets[0].snapshot_read_authority =
        SnapshotReadAuthority::root_configured_read(AuthorityEvidence::Declared);
    let receipts = execution_preflight_receipts(&subtree_plan());

    plan.apply_execution_preflight_receipts(&receipts, AS_OF)
        .expect("apply execution preflight bundle");

    assert_eq!(plan.targets[0].control_authority, proven_root_control());
    assert_eq!(plan.targets[0].snapshot_read_authority, proven_root_read());
    plan.validate_for_execution()
        .expect("bundle makes plan executable");
}

// Ensure stale preflight bundles cannot authorize later mutation.
#[test]
fn rejects_expired_execution_preflight_bundle() {
    let mut plan = subtree_plan();
    let receipts = execution_preflight_receipts(&plan);

    let err = plan
        .apply_execution_preflight_receipts(&receipts, "unix:250")
        .expect_err("expired preflight bundle rejects");

    assert!(matches!(
        err,
        BackupPlanError::PreflightReceiptExpired { preflight_id, expires_at, as_of }
            if preflight_id == PREFLIGHT_ID && expires_at == EXPIRES_AT && as_of == "unix:250"
    ));
}

// Ensure receipt bundles cannot mix proofs from a different preflight run.
#[test]
fn rejects_mismatched_preflight_id_in_bundle_receipts() {
    let mut plan = subtree_plan();
    let mut receipts = execution_preflight_receipts(&plan);
    receipts.topology.preflight_id = "preflight-other".to_string();

    let err = plan
        .apply_execution_preflight_receipts(&receipts, AS_OF)
        .expect_err("mismatched preflight receipt rejects");

    assert!(matches!(
        err,
        BackupPlanError::PreflightReceiptIdMismatch { expected, actual }
            if expected == PREFLIGHT_ID && actual == "preflight-other"
    ));
}

// Ensure standalone authority proofs also expire.
#[test]
fn rejects_expired_authority_receipt() {
    let mut plan = subtree_plan();

    let err = plan
        .apply_control_authority_receipts(
            PREFLIGHT_ID,
            &[control_receipt(APP, proven_root_control())],
            "unix:250",
        )
        .expect_err("expired authority receipt rejects");

    assert!(matches!(
        err,
        BackupPlanError::PreflightReceiptExpired { preflight_id, expires_at, as_of }
            if preflight_id == PREFLIGHT_ID && expires_at == EXPIRES_AT && as_of == "unix:250"
    ));
}

// Ensure topology drift fails before mutation.
#[test]
fn rejects_topology_preflight_hash_drift() {
    let plan = subtree_plan();
    let mut receipt = topology_receipt(&plan);
    receipt.topology_hash_at_preflight =
        "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff".to_string();

    let err = plan
        .validate_execution_preflight_receipts(
            &receipt,
            &quiescence_receipt(&plan),
            PREFLIGHT_ID,
            AS_OF,
        )
        .expect_err("topology drift rejects");

    assert!(matches!(
        err,
        BackupPlanError::TopologyPreflightHashMismatch { expected, actual }
            if expected == plan.topology_hash_before_quiesce
                && actual == "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"
    ));
}

// Ensure quiescence rejection fails before mutation.
#[test]
fn rejects_unaccepted_quiescence_preflight() {
    let plan = subtree_plan();
    let mut receipt = quiescence_receipt(&plan);
    receipt.accepted = false;

    let err = plan
        .validate_execution_preflight_receipts(
            &topology_receipt(&plan),
            &receipt,
            PREFLIGHT_ID,
            AS_OF,
        )
        .expect_err("quiescence rejection rejects");

    assert!(matches!(err, BackupPlanError::QuiescencePreflightRejected));
}

// Ensure quiescence receipts cannot silently cover a different target set.
#[test]
fn rejects_quiescence_target_mismatch() {
    let plan = subtree_plan();
    let mut receipt = quiescence_receipt(&plan);
    receipt.targets.clear();

    let err = plan
        .validate_execution_preflight_receipts(
            &topology_receipt(&plan),
            &receipt,
            PREFLIGHT_ID,
            AS_OF,
        )
        .expect_err("quiescence target mismatch rejects");

    assert!(matches!(
        err,
        BackupPlanError::QuiescencePreflightTargetsMismatch
    ));
}

fn subtree_plan() -> BackupPlan {
    BackupPlan {
        plan_id: "plan-001".to_string(),
        run_id: "run-001".to_string(),
        fleet: "demo".to_string(),
        network: "local".to_string(),
        root_canister_id: ROOT.to_string(),
        selected_subtree_root: Some(APP.to_string()),
        selected_scope_kind: BackupScopeKind::Subtree,
        include_descendants: true,
        root_included: false,
        requires_root_controller: true,
        snapshot_read_authority: proven_root_read(),
        quiescence_policy: QuiescencePolicy::RootCoordinated,
        topology_hash_before_quiesce:
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string(),
        targets: vec![BackupTarget {
            canister_id: APP.to_string(),
            role: Some("app".to_string()),
            parent_canister_id: Some(ROOT.to_string()),
            depth: 1,
            control_authority: proven_root_control(),
            snapshot_read_authority: proven_root_read(),
            identity_mode: IdentityMode::Relocatable,
            expected_module_hash: None,
        }],
        phases: vec![
            phase(
                "validate-topology",
                0,
                BackupOperationKind::ValidateTopology,
                None,
            ),
            phase(
                "validate-control",
                1,
                BackupOperationKind::ValidateControlAuthority,
                None,
            ),
            phase(
                "validate-read",
                2,
                BackupOperationKind::ValidateSnapshotReadAuthority,
                None,
            ),
            phase(
                "validate-quiescence",
                3,
                BackupOperationKind::ValidateQuiescencePolicy,
                None,
            ),
            phase("stop-app", 4, BackupOperationKind::Stop, Some(APP)),
            phase(
                "snapshot-app",
                5,
                BackupOperationKind::CreateSnapshot,
                Some(APP),
            ),
            phase("start-app", 6, BackupOperationKind::Start, Some(APP)),
            phase(
                "download-app",
                7,
                BackupOperationKind::DownloadSnapshot,
                Some(APP),
            ),
            phase(
                "verify-app",
                8,
                BackupOperationKind::VerifyArtifact,
                Some(APP),
            ),
            phase("finalize", 9, BackupOperationKind::FinalizeManifest, None),
        ],
    }
}

fn plan_input<'a>() -> BackupPlanBuildInput<'a> {
    BackupPlanBuildInput {
        plan_id: "plan-001".to_string(),
        run_id: "run-001".to_string(),
        fleet: "demo".to_string(),
        network: "local".to_string(),
        root_canister_id: ROOT.to_string(),
        selected_canister_id: Some(APP.to_string()),
        selected_scope_kind: BackupScopeKind::Subtree,
        include_descendants: true,
        topology_hash_before_quiesce:
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string(),
        registry: &[],
        control_authority: proven_root_control(),
        snapshot_read_authority: proven_root_read(),
        quiescence_policy: QuiescencePolicy::RootCoordinated,
        identity_mode: IdentityMode::Relocatable,
    }
}

fn proven_root_control() -> ControlAuthority {
    ControlAuthority::root_controller(AuthorityEvidence::Proven)
}

fn proven_root_read() -> SnapshotReadAuthority {
    SnapshotReadAuthority::root_configured_read(AuthorityEvidence::Proven)
}

fn control_receipt(canister_id: &str, authority: ControlAuthority) -> ControlAuthorityReceipt {
    ControlAuthorityReceipt {
        plan_id: "plan-001".to_string(),
        preflight_id: PREFLIGHT_ID.to_string(),
        target_canister_id: canister_id.to_string(),
        authority,
        proof_source: AuthorityProofSource::RootCoordination,
        validated_at: VALIDATED_AT.to_string(),
        expires_at: EXPIRES_AT.to_string(),
        message: None,
    }
}

fn snapshot_read_receipt(
    canister_id: &str,
    authority: SnapshotReadAuthority,
) -> SnapshotReadAuthorityReceipt {
    SnapshotReadAuthorityReceipt {
        plan_id: "plan-001".to_string(),
        preflight_id: PREFLIGHT_ID.to_string(),
        target_canister_id: canister_id.to_string(),
        authority,
        proof_source: AuthorityProofSource::SnapshotReadCheck,
        validated_at: VALIDATED_AT.to_string(),
        expires_at: EXPIRES_AT.to_string(),
        message: None,
    }
}

fn topology_receipt(plan: &BackupPlan) -> TopologyPreflightReceipt {
    TopologyPreflightReceipt {
        plan_id: plan.plan_id.clone(),
        preflight_id: PREFLIGHT_ID.to_string(),
        topology_hash_before_quiesce: plan.topology_hash_before_quiesce.clone(),
        topology_hash_at_preflight: plan.topology_hash_before_quiesce.clone(),
        targets: plan.topology_preflight_request().targets,
        validated_at: VALIDATED_AT.to_string(),
        expires_at: EXPIRES_AT.to_string(),
        message: None,
    }
}

fn quiescence_receipt(plan: &BackupPlan) -> QuiescencePreflightReceipt {
    QuiescencePreflightReceipt {
        plan_id: plan.plan_id.clone(),
        preflight_id: PREFLIGHT_ID.to_string(),
        quiescence_policy: plan.quiescence_policy.clone(),
        accepted: true,
        targets: plan.quiescence_preflight_request().targets,
        validated_at: VALIDATED_AT.to_string(),
        expires_at: EXPIRES_AT.to_string(),
        message: None,
    }
}

fn execution_preflight_receipts(plan: &BackupPlan) -> BackupExecutionPreflightReceipts {
    BackupExecutionPreflightReceipts {
        plan_id: plan.plan_id.clone(),
        preflight_id: PREFLIGHT_ID.to_string(),
        validated_at: VALIDATED_AT.to_string(),
        expires_at: EXPIRES_AT.to_string(),
        topology: topology_receipt(plan),
        control_authority: vec![control_receipt(APP, proven_root_control())],
        snapshot_read_authority: vec![snapshot_read_receipt(APP, proven_root_read())],
        quiescence: quiescence_receipt(plan),
    }
}

fn registry() -> Vec<RegistryEntry> {
    vec![
        RegistryEntry {
            pid: ROOT.to_string(),
            role: Some("root".to_string()),
            kind: Some("root".to_string()),
            parent_pid: None,
            module_hash: None,
        },
        RegistryEntry {
            pid: APP.to_string(),
            role: Some("app".to_string()),
            kind: Some("singleton".to_string()),
            parent_pid: Some(ROOT.to_string()),
            module_hash: None,
        },
        RegistryEntry {
            pid: WORKER.to_string(),
            role: Some("worker".to_string()),
            kind: Some("replica".to_string()),
            parent_pid: Some(APP.to_string()),
            module_hash: None,
        },
    ]
}

fn assert_operation_order(plan: &BackupPlan, expected: &[(&str, Option<&str>)]) {
    let actual = plan
        .phases
        .iter()
        .take(expected.len())
        .map(|phase| {
            (
                phase.operation_id.as_str(),
                phase.target_canister_id.as_deref(),
            )
        })
        .collect::<Vec<_>>();

    assert_eq!(actual, expected);
}

fn reset_phase_order(phases: &mut [BackupOperation]) {
    for (index, phase) in phases.iter_mut().enumerate() {
        phase.order = u32::try_from(index).expect("test phase index fits u32");
    }
}

fn phase(
    operation_id: &str,
    order: u32,
    kind: BackupOperationKind,
    target_canister_id: Option<&str>,
) -> BackupOperation {
    BackupOperation {
        operation_id: operation_id.to_string(),
        order,
        kind,
        target_canister_id: target_canister_id.map(str::to_string),
    }
}
