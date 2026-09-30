use super::*;
use crate::{
    cdk::types::Cycles,
    config::schema::{
        CanisterAuthConfig, CanisterConfig, CanisterKind, CyclesFundingPolicyConfig,
        DiagnosticsCanisterConfig, IndexConfig, IndexPool, MetricsCanisterConfig,
        StandardsCanisterConfig,
    },
    ids::{CanisterRole, ComponentSpecId},
    ops::{
        storage::children::CanisterChildrenOps,
        storage::intent::IntentStoreOps,
        storage::placement::index::{
            PlacementIndexClaimResult, PlacementIndexPendingClaim, PlacementIndexRegistryOps,
        },
    },
    test::{
        config::ConfigTestBuilder,
        seams::{lock, p},
        support::{direct_child, import_test_env},
    },
};
use futures::executor::block_on;

fn claim_id(id: u64) -> u64 {
    id
}

fn index_hub_config(instance_role: &CanisterRole) -> CanisterConfig {
    let mut index = IndexConfig::default();
    index.pools.insert(
        "projects".to_string(),
        IndexPool {
            canister_role: instance_role.clone(),
            key_name: "project".to_string(),
        },
    );

    CanisterConfig {
        kind: CanisterKind::Service,
        initial_cycles: Cycles::new(0),
        topup: None,
        cycles_funding: CyclesFundingPolicyConfig::default(),
        scaling: None,
        sharding: None,
        index: Some(index),
        auth: CanisterAuthConfig::default(),
        standards: StandardsCanisterConfig::default(),
        diagnostics: DiagnosticsCanisterConfig::default(),
        metrics: MetricsCanisterConfig::default(),
    }
}

fn install_index_test_context(child_role: &CanisterRole, child_pid: Principal) {
    let root_pid = p(1);
    let hub_pid = p(2);

    let _cfg = ConfigTestBuilder::new()
        .with_default_canister("project_hub", index_hub_config(child_role))
        .with_default_canister(
            "project_instance",
            ConfigTestBuilder::canister_config(CanisterKind::Instance),
        )
        .install();

    import_test_env(
        CanisterRole::new("project_hub"),
        ComponentSpecId::try_from(String::from("default")).expect("default Component Spec ID"),
        root_pid,
    );

    PlacementIndexRegistryOps::clear_for_test();
    IntentStoreOps::reset_for_tests();
    CanisterChildrenOps::import_direct_children(
        hub_pid,
        vec![direct_child(child_pid, child_role.clone(), [1; 32])],
    );
}

#[test]
fn late_created_child_completion_preserves_an_already_bound_result_after_receipt_cleanup() {
    let _guard = lock();
    let child_role = CanisterRole::new("project_instance");
    let child_pid = p(3);
    install_index_test_context(&child_role, child_pid);
    let PlacementIndexClaimResult::Claimed(claim) =
        PlacementIndexRegistryOps::claim_pending("projects", "alpha", p(2), claim_id(1), 1)
            .unwrap()
    else {
        panic!("expected a new pending claim");
    };
    let pool = IndexPool {
        canister_role: child_role.clone(),
        key_name: "project".into(),
    };
    let request =
        super::create::placement_index_allocation_request("projects", "alpha", &pool, claim);
    CanisterChildrenOps::import_direct_children(
        p(2),
        vec![direct_child(
            child_pid,
            child_role,
            request.identity.operation_id.into_bytes(),
        )],
    );
    let permit =
        crate::workflow::placement::allocation::PlacementAllocationWorkflow::resume_permit(
            &request,
        )
        .unwrap();
    PlacementIndexRegistryOps::bind_if_claim_matches(
        "projects",
        "alpha",
        claim.claim_id,
        child_pid,
        request.identity.operation_id.into_bytes(),
        10,
    )
    .unwrap();
    let expected = PlacementIndexStatusResponse::Bound {
        instance_pid: child_pid,
        bound_at: 10,
    };

    for _ in 0..2 {
        assert_eq!(
            block_on(PlacementIndexWorkflow::finalize_created_instance(
                "projects", "alpha", claim, child_pid, &permit
            ))
            .unwrap(),
            Some(expected.clone())
        );
        assert_eq!(
            PlacementIndexRegistryOps::lookup_entry("projects", "alpha"),
            Some(expected.clone())
        );
        let totals = IntentStoreOps::totals(&request.identity.resource_key);
        assert_eq!(totals.committed_qty, 0);
        assert_eq!(totals.pending_count, 0);
        assert_eq!(totals.reserved_qty, 0);
    }
}

#[test]
fn bind_instance_persists_assignment_for_matching_direct_child() {
    let _guard = lock();
    let child_role = CanisterRole::new("project_instance");
    let child_pid = p(3);
    install_index_test_context(&child_role, child_pid);

    PlacementIndexWorkflow::bind_instance("projects", "alpha", child_pid)
        .expect("bind should succeed");

    assert_eq!(
        query::PlacementIndexQuery::lookup_key("projects", "alpha"),
        Some(child_pid)
    );
}

#[test]
fn bind_instance_rejects_non_child_pid() {
    let _guard = lock();
    let child_role = CanisterRole::new("project_instance");
    let child_pid = p(3);
    install_index_test_context(&child_role, child_pid);
    CanisterChildrenOps::import_direct_children(p(2), vec![]);

    PlacementIndexWorkflow::bind_instance("projects", "alpha", child_pid)
        .expect_err("bind should reject non-child pid");
}

#[test]
fn bind_instance_rejects_role_mismatch() {
    let _guard = lock();
    let configured_role = CanisterRole::new("project_instance");
    let actual_role = CanisterRole::new("wrong_instance_role");
    let child_pid = p(3);
    install_index_test_context(&configured_role, child_pid);
    let hub_pid = p(2);
    CanisterChildrenOps::import_direct_children(
        hub_pid,
        vec![direct_child(child_pid, actual_role, [1; 32])],
    );

    PlacementIndexWorkflow::bind_instance("projects", "alpha", child_pid)
        .expect_err("bind should reject mismatched child role");
}

#[test]
fn resolve_or_create_returns_existing_bound_entry_without_create() {
    let _guard = lock();
    let child_role = CanisterRole::new("project_instance");
    let child_pid = p(3);
    install_index_test_context(&child_role, child_pid);
    PlacementIndexRegistryOps::bind("projects", "alpha", child_pid, [1; 32], 10)
        .expect("seed bound entry");

    let result = block_on(PlacementIndexWorkflow::resolve_or_create(
        "projects", "alpha",
    ))
    .expect("bound entry should resolve without create");

    assert_eq!(
        result,
        PlacementIndexStatusResponse::Bound {
            instance_pid: child_pid,
            bound_at: 10,
        }
    );
}

#[test]
fn resolve_or_create_returns_fresh_pending_entry_without_create() {
    let _guard = lock();
    let child_role = CanisterRole::new("project_instance");
    let child_pid = p(3);
    install_index_test_context(&child_role, child_pid);

    let owner_pid = p(7);
    let created_at = IcOps::now_secs();
    let claim = PlacementIndexRegistryOps::claim_pending(
        "projects",
        "alpha",
        owner_pid,
        claim_id(1),
        created_at,
    )
    .expect("seed pending entry");
    assert_eq!(
        claim,
        PlacementIndexClaimResult::Claimed(PlacementIndexPendingClaim {
            claim_id: claim_id(1),
            owner_pid,
            created_at,
        })
    );

    let result = block_on(PlacementIndexWorkflow::resolve_or_create(
        "projects", "alpha",
    ))
    .expect("fresh pending should be surfaced");

    assert_eq!(
        result,
        PlacementIndexStatusResponse::Pending {
            owner_pid,
            created_at,
            provisional_pid: None,
        }
    );
}

#[test]
fn resolve_or_create_repairs_stale_pending_with_valid_provisional_child() {
    let _guard = lock();
    let child_role = CanisterRole::new("project_instance");
    let child_pid = p(3);
    install_index_test_context(&child_role, child_pid);

    let claim = PlacementIndexRegistryOps::claim_pending("projects", "alpha", p(7), claim_id(1), 1)
        .expect("seed stale pending entry");
    let PlacementIndexClaimResult::Claimed(claim) = claim else {
        panic!("expected stale claim");
    };
    let operation_id = crate::model::placement::allocation::PlacementAllocationIdentity::index(
        p(7),
        "projects",
        "alpha",
        claim.claim_id,
        &child_role,
        None,
    )
    .operation_id
    .into_bytes();
    CanisterChildrenOps::import_direct_children(
        p(2),
        vec![direct_child(child_pid, child_role, operation_id)],
    );
    PlacementIndexRegistryOps::set_provisional_pid_if_claim_matches(
        "projects",
        "alpha",
        claim.claim_id,
        child_pid,
    )
    .expect("seed provisional child");

    let result = block_on(PlacementIndexWorkflow::resolve_or_create(
        "projects", "alpha",
    ))
    .expect("stale pending should repair to bound");

    match result {
        PlacementIndexStatusResponse::Bound { instance_pid, .. } => {
            assert_eq!(instance_pid, child_pid);
        }
        other @ PlacementIndexStatusResponse::Pending { .. } => {
            panic!("expected bound result, got {other:?}")
        }
    }
}

#[test]
fn classify_entry_returns_none_for_missing_key() {
    let _guard = lock();
    let child_role = CanisterRole::new("project_instance");
    let child_pid = p(3);
    install_index_test_context(&child_role, child_pid);

    let pool_cfg =
        PlacementIndexWorkflow::get_index_pool_cfg("projects").expect("pool config should exist");
    let classification =
        PlacementIndexWorkflow::classify_entry("projects", "alpha", &pool_cfg, IcOps::now_secs());

    assert_eq!(classification, None);
}

#[test]
fn classify_entry_marks_stale_pending_without_provisional_for_resume() {
    let _guard = lock();
    let child_role = CanisterRole::new("project_instance");
    let child_pid = p(3);
    install_index_test_context(&child_role, child_pid);
    PlacementIndexRegistryOps::claim_pending("projects", "alpha", p(7), claim_id(1), 1)
        .expect("seed stale pending entry");

    let pool_cfg =
        PlacementIndexWorkflow::get_index_pool_cfg("projects").expect("pool config should exist");
    let classification =
        PlacementIndexWorkflow::classify_entry("projects", "alpha", &pool_cfg, IcOps::now_secs());

    assert_eq!(
        classification,
        Some(PlacementIndexEntryClassification::Resumable {
            claim_id: claim_id(1),
            owner_pid: p(7),
            created_at: 1,
        })
    );
}

#[test]
fn classify_entry_marks_invalid_provisional_child_for_cleanup() {
    let _guard = lock();
    let child_role = CanisterRole::new("project_instance");
    let child_pid = p(3);
    install_index_test_context(&child_role, child_pid);
    let claim = PlacementIndexRegistryOps::claim_pending("projects", "alpha", p(7), claim_id(1), 1)
        .expect("seed stale pending entry");
    let PlacementIndexClaimResult::Claimed(claim) = claim else {
        panic!("expected stale claim");
    };
    PlacementIndexRegistryOps::set_provisional_pid_if_claim_matches(
        "projects",
        "alpha",
        claim.claim_id,
        p(8),
    )
    .expect("seed invalid provisional child");

    let pool_cfg =
        PlacementIndexWorkflow::get_index_pool_cfg("projects").expect("pool config should exist");
    let classification =
        PlacementIndexWorkflow::classify_entry("projects", "alpha", &pool_cfg, IcOps::now_secs());

    assert_eq!(
        classification,
        Some(PlacementIndexEntryClassification::NeedsCleanup {
            claim_id: claim_id(1),
            owner_pid: p(7),
            provisional_pid: p(8),
        })
    );
}

#[test]
fn stale_pending_without_provisional_child_remains_claimed_for_exact_resume() {
    let _guard = lock();
    let child_role = CanisterRole::new("project_instance");
    let child_pid = p(3);
    install_index_test_context(&child_role, child_pid);
    PlacementIndexRegistryOps::claim_pending("projects", "alpha", p(7), claim_id(1), 1)
        .expect("seed stale pending entry");

    let pool_cfg =
        PlacementIndexWorkflow::get_index_pool_cfg("projects").expect("pool config should exist");
    assert_eq!(
        PlacementIndexWorkflow::classify_entry("projects", "alpha", &pool_cfg, IcOps::now_secs(),),
        Some(PlacementIndexEntryClassification::Resumable {
            claim_id: claim_id(1),
            owner_pid: p(7),
            created_at: 1,
        })
    );
    let error = block_on(PlacementIndexWorkflow::recover_entry("projects", "alpha"))
        .expect_err("untracked stale claim must remain fail-closed");
    assert_eq!(
        error.public_error().code(),
        crate::diagnostics::codes::STATE_CONFLICT.raw_code()
    );
    assert!(PlacementIndexRegistryOps::lookup_entry("projects", "alpha").is_some());
}

#[test]
fn recover_entry_repairs_valid_stale_provisional_child() {
    let _guard = lock();
    let child_role = CanisterRole::new("project_instance");
    let child_pid = p(3);
    install_index_test_context(&child_role, child_pid);
    let claim = PlacementIndexRegistryOps::claim_pending("projects", "alpha", p(7), claim_id(1), 1)
        .expect("seed stale pending entry");
    let PlacementIndexClaimResult::Claimed(claim) = claim else {
        panic!("expected stale claim");
    };
    let operation_id = crate::model::placement::allocation::PlacementAllocationIdentity::index(
        p(7),
        "projects",
        "alpha",
        claim.claim_id,
        &child_role,
        None,
    )
    .operation_id
    .into_bytes();
    CanisterChildrenOps::import_direct_children(
        p(2),
        vec![direct_child(child_pid, child_role, operation_id)],
    );
    PlacementIndexRegistryOps::set_provisional_pid_if_claim_matches(
        "projects",
        "alpha",
        claim.claim_id,
        child_pid,
    )
    .expect("seed provisional child");

    let result = block_on(PlacementIndexWorkflow::recover_entry("projects", "alpha"))
        .expect("valid provisional child should be repaired");

    let PlacementIndexRecoveryResponse::RepairedToBound {
        instance_pid,
        bound_at,
    } = result
    else {
        panic!("valid provisional child must repair to a bound entry");
    };
    assert_eq!(instance_pid, child_pid);
    assert_eq!(
        PlacementIndexRegistryOps::lookup_entry("projects", "alpha"),
        Some(PlacementIndexStatusResponse::Bound {
            instance_pid: child_pid,
            bound_at,
        })
    );
}

#[test]
fn recover_entry_releases_stale_pending_when_provisional_child_is_missing() {
    let _guard = lock();
    let child_role = CanisterRole::new("project_instance");
    let child_pid = p(3);
    install_index_test_context(&child_role, child_pid);

    let claim = PlacementIndexRegistryOps::claim_pending("projects", "alpha", p(7), claim_id(1), 1)
        .expect("seed stale pending entry");
    let PlacementIndexClaimResult::Claimed(claim) = claim else {
        panic!("expected stale claim");
    };
    PlacementIndexRegistryOps::set_provisional_pid_if_claim_matches(
        "projects",
        "alpha",
        claim.claim_id,
        p(8),
    )
    .expect("seed missing provisional child");

    let recovery_started_at = IcOps::now_secs();
    let result = block_on(PlacementIndexWorkflow::recover_entry("projects", "alpha"))
        .expect("missing provisional child should still release stale key");
    let recovery_finished_at = IcOps::now_secs();

    let PlacementIndexRecoveryResponse::ReleasedStalePending {
        owner_pid,
        created_at,
        provisional_pid,
        released_at,
    } = result
    else {
        panic!("missing provisional child must release the stale pending entry");
    };
    assert_eq!(
        (owner_pid, created_at, provisional_pid),
        (p(7), 1, Some(p(8)))
    );
    assert!((recovery_started_at..=recovery_finished_at).contains(&released_at));
    assert_eq!(
        PlacementIndexRegistryOps::lookup_entry("projects", "alpha"),
        None
    );
}

#[test]
fn failed_local_admission_discards_only_the_new_unissued_claim() {
    use crate::model::placement::allocation::PlacementAllocationIdentity;
    use crate::ops::storage::intent::RECEIPT_BACKED_INTENT_RECORD_LIMIT;
    use crate::workflow::placement::allocation::{
        PlacementAllocationRequest, PlacementAllocationWorkflow,
    };

    let _guard = lock();
    let child_role = CanisterRole::new("project_instance");
    install_index_test_context(&child_role, p(3));
    let mut first = None;
    for sequence in 0..RECEIPT_BACKED_INTENT_RECORD_LIMIT {
        let request = PlacementAllocationRequest {
            identity: PlacementAllocationIdentity::scaling(
                p(2),
                "busy",
                sequence,
                &child_role,
                None,
            ),
            canister_role: child_role.clone(),
            extra_arg: None,
            reservation_limit: RECEIPT_BACKED_INTENT_RECORD_LIMIT,
        };
        let permit = PlacementAllocationWorkflow::resume_permit(&request).unwrap();
        first.get_or_insert(permit);
    }
    let PlacementIndexClaimResult::Claimed(claim) =
        PlacementIndexRegistryOps::claim_pending("projects", "alpha", p(2), 1, 1).unwrap()
    else {
        panic!("new pending claim");
    };
    let pool = IndexPool {
        canister_role: child_role,
        key_name: "project".into(),
    };
    let error = block_on(PlacementIndexWorkflow::create_and_finalize_claim(
        "projects", "alpha", &pool, claim,
    ))
    .unwrap_err();
    assert!(error.is_public_resource_exhausted());
    assert!(PlacementIndexRegistryOps::lookup_entry("projects", "alpha").is_none());

    PlacementAllocationWorkflow::finish_created_child(&first.unwrap(), p(4)).unwrap();
    let PlacementIndexClaimResult::Claimed(retry) =
        PlacementIndexRegistryOps::claim_pending("projects", "alpha", p(2), 2, 2).unwrap()
    else {
        panic!("retry can reclaim the key immediately");
    };
    let request =
        super::create::placement_index_allocation_request("projects", "alpha", &pool, retry);
    PlacementAllocationWorkflow::prepare_child(request).unwrap();
    assert!(
        !PlacementIndexRegistryOps::discard_unadmitted_claim("projects", "alpha", claim).unwrap()
    );
    assert!(PlacementIndexRegistryOps::lookup_entry("projects", "alpha").is_some());
}

#[test]
fn recycled_index_binding_stays_unavailable_until_explicit_recovery() {
    let _guard = lock();
    let role = CanisterRole::new("project_instance");
    let child = p(3);
    install_index_test_context(&role, child);
    PlacementIndexWorkflow::bind_instance("projects", "alpha", child).unwrap();
    CanisterChildrenOps::import_direct_children(p(2), vec![]);
    assert_eq!(
        query::PlacementIndexQuery::lookup_key("projects", "alpha"),
        None
    );
    CanisterChildrenOps::import_direct_children(p(2), vec![direct_child(child, role, [2; 32])]);
    assert_eq!(
        query::PlacementIndexQuery::lookup_key("projects", "alpha"),
        None
    );
    let error = block_on(PlacementIndexWorkflow::resolve_or_create(
        "projects", "alpha",
    ))
    .unwrap_err();
    assert_eq!(
        error.code(),
        crate::diagnostics::codes::POSITION_UNAVAILABLE
    );
    assert!(PlacementIndexRegistryOps::lookup_state("projects", "alpha").is_some());
    assert!(
        matches!(block_on(PlacementIndexWorkflow::recover_entry("projects", "alpha")).unwrap(), PlacementIndexRecoveryResponse::ReleasedUnavailableBinding { instance_pid, .. } if instance_pid == child)
    );
    PlacementIndexWorkflow::bind_instance("projects", "alpha", child).unwrap();
    assert_eq!(
        query::PlacementIndexQuery::lookup_key("projects", "alpha"),
        Some(child)
    );
}

#[test]
fn late_index_completion_cannot_bind_or_recycle_a_reused_principal() {
    let _guard = lock();
    let role = CanisterRole::new("project_instance");
    let child = p(3);
    install_index_test_context(&role, child);
    let PlacementIndexClaimResult::Claimed(claim) =
        PlacementIndexRegistryOps::claim_pending("projects", "alpha", p(2), 1, 1).unwrap()
    else {
        panic!("new claim");
    };
    let pool = IndexPool {
        canister_role: role,
        key_name: "project".into(),
    };
    let request =
        super::create::placement_index_allocation_request("projects", "alpha", &pool, claim);
    let permit =
        crate::workflow::placement::allocation::PlacementAllocationWorkflow::resume_permit(
            &request,
        )
        .unwrap();
    let error = block_on(PlacementIndexWorkflow::finalize_created_instance(
        "projects", "alpha", claim, child, &permit,
    ))
    .unwrap_err();
    assert_eq!(
        error.code(),
        crate::diagnostics::codes::POSITION_UNAVAILABLE
    );
    assert!(PlacementIndexRegistryOps::lookup_state("projects", "alpha").is_none());
    block_on(PlacementIndexWorkflow::recycle_abandoned_child(
        child, &permit,
    ))
    .unwrap();
    assert!(CanisterChildrenOps::matches_allocation(child, [1; 32]));
    assert_eq!(
        IntentStoreOps::totals(&request.identity.resource_key).pending_count,
        0
    );
    // An even later callback must also preserve a newly bound allocation after old receipt cleanup.
    PlacementIndexWorkflow::bind_instance("projects", "alpha", child).unwrap();
    let rebound = PlacementIndexRegistryOps::lookup_state("projects", "alpha");
    let error = block_on(PlacementIndexWorkflow::finalize_created_instance(
        "projects", "alpha", claim, child, &permit,
    ))
    .unwrap_err();
    assert_eq!(
        error.code(),
        crate::diagnostics::codes::POSITION_UNAVAILABLE
    );
    assert_eq!(
        PlacementIndexRegistryOps::lookup_state("projects", "alpha"),
        rebound
    );
    assert_eq!(
        query::PlacementIndexQuery::lookup_key("projects", "alpha"),
        Some(child)
    );
}

#[test]
fn stale_provisional_child_reuse_releases_only_the_old_index_claim() {
    let _guard = lock();
    let role = CanisterRole::new("project_instance");
    let child = p(3);
    install_index_test_context(&role, child);
    let PlacementIndexClaimResult::Claimed(claim) =
        PlacementIndexRegistryOps::claim_pending("projects", "alpha", p(2), 1, 1).unwrap()
    else {
        panic!("new claim");
    };
    PlacementIndexRegistryOps::set_provisional_pid_if_claim_matches(
        "projects",
        "alpha",
        claim.claim_id,
        child,
    )
    .unwrap();
    let result = block_on(PlacementIndexWorkflow::recover_entry("projects", "alpha")).unwrap();
    assert!(
        matches!(result, PlacementIndexRecoveryResponse::ReleasedStalePending { provisional_pid: Some(pid), .. } if pid == child)
    );
    assert!(CanisterChildrenOps::matches_allocation(child, [1; 32]));
    assert!(PlacementIndexRegistryOps::lookup_state("projects", "alpha").is_none());
}

#[test]
fn cleanup_rechecks_a_bound_result_after_the_retired_claim_finishes() {
    let _guard = lock();
    let role = CanisterRole::new("project_instance");
    let child = p(3);
    install_index_test_context(&role, child);
    PlacementIndexWorkflow::bind_instance("projects", "alpha", child).unwrap();
    CanisterChildrenOps::import_direct_children(
        p(2),
        vec![direct_child(child, role.clone(), [2; 32])],
    );
    let pool = IndexPool {
        canister_role: role,
        key_name: "project".into(),
    };
    assert_eq!(
        block_on(PlacementIndexWorkflow::recover_cleanup_stale_entry(
            "projects",
            "alpha",
            &pool,
            1,
            p(2),
            child
        ))
        .unwrap(),
        None
    );
    assert!(PlacementIndexRegistryOps::lookup_state("projects", "alpha").is_some());
    assert!(CanisterChildrenOps::matches_allocation(child, [2; 32]));
}

#[test]
fn retired_reply_preserves_an_already_disposed_allocation_outcome() {
    use crate::workflow::placement::allocation::PlacementAllocationWorkflow;
    let _guard = lock();
    let role = CanisterRole::new("project_instance");
    let child = p(3);
    install_index_test_context(&role, child);
    let PlacementIndexClaimResult::Claimed(claim) =
        PlacementIndexRegistryOps::claim_pending("projects", "alpha", p(2), 1, 1).unwrap()
    else {
        panic!("new claim");
    };
    let pool = IndexPool {
        canister_role: role,
        key_name: "project".into(),
    };
    let request =
        super::create::placement_index_allocation_request("projects", "alpha", &pool, claim);
    let permit = PlacementAllocationWorkflow::resume_permit(&request).unwrap();
    PlacementAllocationWorkflow::rollback_disposed_child(&permit, child).unwrap();
    let error = block_on(PlacementIndexWorkflow::finalize_created_instance(
        "projects", "alpha", claim, child, &permit,
    ))
    .unwrap_err();
    assert_eq!(
        error.code(),
        crate::diagnostics::codes::POSITION_UNAVAILABLE
    );
    assert!(PlacementIndexRegistryOps::lookup_state("projects", "alpha").is_none());
    let totals = IntentStoreOps::totals(&request.identity.resource_key);
    assert_eq!(
        (
            totals.pending_count,
            totals.committed_qty,
            totals.reserved_qty
        ),
        (0, 0, 0)
    );
    assert!(CanisterChildrenOps::matches_allocation(child, [1; 32]));
}
