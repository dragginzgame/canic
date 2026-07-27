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
        storage::registry::subnet::SubnetRegistryOps,
    },
    test::{
        config::ConfigTestBuilder,
        seams::{lock, p},
        support::import_test_env,
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
        icp_refill: None,
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

fn clear_subnet_registry() {
    for entry in SubnetRegistryOps::data().entries {
        let _ = SubnetRegistryOps::unregister(&entry.pid);
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

    clear_subnet_registry();
    PlacementIndexRegistryOps::clear_for_test();
    IntentStoreOps::reset_for_tests();
    CanisterChildrenOps::import_direct_children(hub_pid, vec![(child_pid, child_role.clone())]);

    let created_at = 0;
    SubnetRegistryOps::register_root(root_pid, created_at);
    SubnetRegistryOps::register_unchecked(
        hub_pid,
        &CanisterRole::new("project_hub"),
        root_pid,
        vec![],
        created_at,
    )
    .expect("register hub");
    SubnetRegistryOps::register_unchecked(child_pid, child_role, hub_pid, vec![], created_at)
        .expect("register child");
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
    clear_subnet_registry();

    let root_pid = p(1);
    let hub_pid = p(2);
    let created_at = 0;
    SubnetRegistryOps::register_root(root_pid, created_at);
    SubnetRegistryOps::register_unchecked(
        hub_pid,
        &CanisterRole::new("project_hub"),
        root_pid,
        vec![],
        created_at,
    )
    .expect("register hub");
    SubnetRegistryOps::register_unchecked(child_pid, &actual_role, hub_pid, vec![], created_at)
        .expect("register mismatched child");

    PlacementIndexWorkflow::bind_instance("projects", "alpha", child_pid)
        .expect_err("bind should reject mismatched child role");
}

#[test]
fn resolve_or_create_returns_existing_bound_entry_without_create() {
    let _guard = lock();
    let child_role = CanisterRole::new("project_instance");
    let child_pid = p(3);
    install_index_test_context(&child_role, child_pid);
    PlacementIndexRegistryOps::bind("projects", "alpha", child_pid, 10).expect("seed bound entry");

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
        error.public_error().map(|error| error.code),
        Some(crate::dto::error::ErrorCode::Conflict)
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
    PlacementIndexRegistryOps::set_provisional_pid_if_claim_matches(
        "projects",
        "alpha",
        claim.claim_id,
        child_pid,
    )
    .expect("seed provisional child");

    let result = block_on(PlacementIndexWorkflow::recover_entry("projects", "alpha"))
        .expect("valid provisional child should be repaired");

    assert_eq!(
        result,
        PlacementIndexRecoveryResponse::RepairedToBound {
            instance_pid: child_pid,
            bound_at: IcOps::now_secs(),
        }
    );
    std::assert_matches!(
        PlacementIndexRegistryOps::lookup_entry("projects", "alpha"),
        Some(PlacementIndexStatusResponse::Bound { instance_pid, .. }) if instance_pid == child_pid
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

    let result = block_on(PlacementIndexWorkflow::recover_entry("projects", "alpha"))
        .expect("missing provisional child should still release stale key");

    assert_eq!(
        result,
        PlacementIndexRecoveryResponse::ReleasedStalePending {
            owner_pid: p(7),
            created_at: 1,
            provisional_pid: Some(p(8)),
            released_at: IcOps::now_secs(),
        }
    );
    assert_eq!(
        PlacementIndexRegistryOps::lookup_entry("projects", "alpha"),
        None
    );
}
