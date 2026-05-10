use super::*;
use crate::{
    cdk::types::Cycles,
    config::schema::{
        CanisterAuthConfig, CanisterConfig, CanisterKind, DirectoryConfig, DirectoryPool,
        MetricsCanisterConfig, RandomnessConfig, StandardsCanisterConfig,
    },
    ids::{CanisterRole, SubnetRole},
    ops::{
        storage::children::CanisterChildrenOps,
        storage::placement::directory::{DirectoryClaimResult, DirectoryRegistryOps},
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

fn directory_hub_config(instance_role: &CanisterRole) -> CanisterConfig {
    let mut directory = DirectoryConfig::default();
    directory.pools.insert(
        "projects".to_string(),
        DirectoryPool {
            canister_role: instance_role.clone(),
            key_name: "project".to_string(),
        },
    );

    CanisterConfig {
        kind: CanisterKind::Singleton,
        initial_cycles: Cycles::new(0),
        topup_policy: None,
        randomness: RandomnessConfig::default(),
        scaling: None,
        sharding: None,
        directory: Some(directory),
        auth: CanisterAuthConfig::default(),
        standards: StandardsCanisterConfig::default(),
        metrics: MetricsCanisterConfig::default(),
    }
}

fn clear_subnet_registry() {
    for (pid, _) in SubnetRegistryOps::data().entries {
        let _ = SubnetRegistryOps::remove(&pid);
    }
}

fn install_directory_test_context(child_role: &CanisterRole, child_pid: Principal) {
    let root_pid = p(1);
    let hub_pid = p(2);

    let _cfg = ConfigTestBuilder::new()
        .with_prime_canister("project_hub", directory_hub_config(child_role))
        .with_prime_canister(
            "project_instance",
            ConfigTestBuilder::canister_config(CanisterKind::Instance),
        )
        .install();

    import_test_env(
        CanisterRole::new("project_hub"),
        SubnetRole::PRIME,
        root_pid,
    );

    clear_subnet_registry();
    DirectoryRegistryOps::clear_for_test();
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
    install_directory_test_context(&child_role, child_pid);

    DirectoryWorkflow::bind_instance("projects", "alpha", child_pid).expect("bind should succeed");

    assert_eq!(
        query::DirectoryQuery::lookup_key("projects", "alpha"),
        Some(child_pid)
    );
}

#[test]
fn bind_instance_rejects_non_child_pid() {
    let _guard = lock();
    let child_role = CanisterRole::new("project_instance");
    let child_pid = p(3);
    install_directory_test_context(&child_role, child_pid);
    CanisterChildrenOps::import_direct_children(p(2), vec![]);

    let err = DirectoryWorkflow::bind_instance("projects", "alpha", child_pid)
        .expect_err("bind should reject non-child pid");

    assert!(err.to_string().contains("not a direct child"));
}

#[test]
fn bind_instance_rejects_role_mismatch() {
    let _guard = lock();
    let configured_role = CanisterRole::new("project_instance");
    let actual_role = CanisterRole::new("wrong_instance_role");
    let child_pid = p(3);
    install_directory_test_context(&configured_role, child_pid);
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

    let err = DirectoryWorkflow::bind_instance("projects", "alpha", child_pid)
        .expect_err("bind should reject mismatched child role");

    assert!(err.to_string().contains("expected"));
}

#[test]
fn resolve_or_create_returns_existing_bound_entry_without_create() {
    let _guard = lock();
    let child_role = CanisterRole::new("project_instance");
    let child_pid = p(3);
    install_directory_test_context(&child_role, child_pid);
    DirectoryRegistryOps::bind("projects", "alpha", child_pid, 10).expect("seed bound entry");

    let result = block_on(DirectoryWorkflow::resolve_or_create("projects", "alpha"))
        .expect("bound entry should resolve without create");

    assert_eq!(
        result,
        DirectoryEntryStatusResponse::Bound {
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
    install_directory_test_context(&child_role, child_pid);

    let owner_pid = p(7);
    let created_at = IcOps::now_secs();
    let claim = DirectoryRegistryOps::claim_pending(
        "projects",
        "alpha",
        owner_pid,
        claim_id(1),
        created_at,
    )
    .expect("seed pending entry");
    assert_eq!(
        claim,
        DirectoryClaimResult::Claimed(DirectoryPendingClaim {
            claim_id: claim_id(1),
            owner_pid,
            created_at,
        })
    );

    let result = block_on(DirectoryWorkflow::resolve_or_create("projects", "alpha"))
        .expect("fresh pending should be surfaced");

    assert_eq!(
        result,
        DirectoryEntryStatusResponse::Pending {
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
    install_directory_test_context(&child_role, child_pid);

    let claim = DirectoryRegistryOps::claim_pending("projects", "alpha", p(7), claim_id(1), 1)
        .expect("seed stale pending entry");
    let DirectoryClaimResult::Claimed(claim) = claim else {
        panic!("expected stale claim");
    };
    DirectoryRegistryOps::set_provisional_pid_if_claim_matches(
        "projects",
        "alpha",
        claim.claim_id,
        child_pid,
    )
    .expect("seed provisional child");

    let result = block_on(DirectoryWorkflow::resolve_or_create("projects", "alpha"))
        .expect("stale pending should repair to bound");

    match result {
        DirectoryEntryStatusResponse::Bound { instance_pid, .. } => {
            assert_eq!(instance_pid, child_pid);
        }
        other @ DirectoryEntryStatusResponse::Pending { .. } => {
            panic!("expected bound result, got {other:?}")
        }
    }
}

#[test]
fn classify_entry_returns_none_for_missing_key() {
    let _guard = lock();
    let child_role = CanisterRole::new("project_instance");
    let child_pid = p(3);
    install_directory_test_context(&child_role, child_pid);

    let pool_cfg =
        DirectoryWorkflow::get_directory_pool_cfg("projects").expect("pool config should exist");
    let classification =
        DirectoryWorkflow::classify_entry("projects", "alpha", &pool_cfg, IcOps::now_secs());

    assert_eq!(classification, None);
}

#[test]
fn classify_entry_marks_stale_pending_without_provisional_for_cleanup() {
    let _guard = lock();
    let child_role = CanisterRole::new("project_instance");
    let child_pid = p(3);
    install_directory_test_context(&child_role, child_pid);
    DirectoryRegistryOps::claim_pending("projects", "alpha", p(7), claim_id(1), 1)
        .expect("seed stale pending entry");

    let pool_cfg =
        DirectoryWorkflow::get_directory_pool_cfg("projects").expect("pool config should exist");
    let classification =
        DirectoryWorkflow::classify_entry("projects", "alpha", &pool_cfg, IcOps::now_secs());

    assert_eq!(
        classification,
        Some(DirectoryEntryClassification::NeedsCleanup {
            claim_id: claim_id(1),
            provisional_pid: None
        })
    );
}

#[test]
fn classify_entry_marks_invalid_provisional_child_for_cleanup() {
    let _guard = lock();
    let child_role = CanisterRole::new("project_instance");
    let child_pid = p(3);
    install_directory_test_context(&child_role, child_pid);
    let claim = DirectoryRegistryOps::claim_pending("projects", "alpha", p(7), claim_id(1), 1)
        .expect("seed stale pending entry");
    let DirectoryClaimResult::Claimed(claim) = claim else {
        panic!("expected stale claim");
    };
    DirectoryRegistryOps::set_provisional_pid_if_claim_matches(
        "projects",
        "alpha",
        claim.claim_id,
        p(8),
    )
    .expect("seed invalid provisional child");

    let pool_cfg =
        DirectoryWorkflow::get_directory_pool_cfg("projects").expect("pool config should exist");
    let classification =
        DirectoryWorkflow::classify_entry("projects", "alpha", &pool_cfg, IcOps::now_secs());

    assert_eq!(
        classification,
        Some(DirectoryEntryClassification::NeedsCleanup {
            claim_id: claim_id(1),
            provisional_pid: Some(p(8))
        })
    );
}

#[test]
fn recover_entry_releases_stale_pending_without_provisional_child() {
    let _guard = lock();
    let child_role = CanisterRole::new("project_instance");
    let child_pid = p(3);
    install_directory_test_context(&child_role, child_pid);
    DirectoryRegistryOps::claim_pending("projects", "alpha", p(7), claim_id(1), 1)
        .expect("seed stale pending entry");

    let result = block_on(DirectoryWorkflow::recover_entry("projects", "alpha"))
        .expect("stale dead key should be released");

    assert_eq!(
        result,
        DirectoryRecoveryResponse::ReleasedStalePending {
            owner_pid: p(7),
            created_at: 1,
            provisional_pid: None,
            released_at: IcOps::now_secs(),
        }
    );
    assert_eq!(
        DirectoryRegistryOps::lookup_entry("projects", "alpha"),
        None
    );
}

#[test]
fn recover_entry_repairs_valid_stale_provisional_child() {
    let _guard = lock();
    let child_role = CanisterRole::new("project_instance");
    let child_pid = p(3);
    install_directory_test_context(&child_role, child_pid);
    let claim = DirectoryRegistryOps::claim_pending("projects", "alpha", p(7), claim_id(1), 1)
        .expect("seed stale pending entry");
    let DirectoryClaimResult::Claimed(claim) = claim else {
        panic!("expected stale claim");
    };
    DirectoryRegistryOps::set_provisional_pid_if_claim_matches(
        "projects",
        "alpha",
        claim.claim_id,
        child_pid,
    )
    .expect("seed provisional child");

    let result = block_on(DirectoryWorkflow::recover_entry("projects", "alpha"))
        .expect("valid provisional child should be repaired");

    assert_eq!(
        result,
        DirectoryRecoveryResponse::RepairedToBound {
            instance_pid: child_pid,
            bound_at: IcOps::now_secs(),
        }
    );
    assert!(matches!(
        DirectoryRegistryOps::lookup_entry("projects", "alpha"),
        Some(DirectoryEntryStatusResponse::Bound { instance_pid, .. }) if instance_pid == child_pid
    ));
}

#[test]
fn recover_entry_releases_stale_pending_when_provisional_child_is_missing() {
    let _guard = lock();
    let child_role = CanisterRole::new("project_instance");
    let child_pid = p(3);
    install_directory_test_context(&child_role, child_pid);

    let claim = DirectoryRegistryOps::claim_pending("projects", "alpha", p(7), claim_id(1), 1)
        .expect("seed stale pending entry");
    let DirectoryClaimResult::Claimed(claim) = claim else {
        panic!("expected stale claim");
    };
    DirectoryRegistryOps::set_provisional_pid_if_claim_matches(
        "projects",
        "alpha",
        claim.claim_id,
        p(8),
    )
    .expect("seed missing provisional child");

    let result = block_on(DirectoryWorkflow::recover_entry("projects", "alpha"))
        .expect("missing provisional child should still release stale key");

    assert_eq!(
        result,
        DirectoryRecoveryResponse::ReleasedStalePending {
            owner_pid: p(7),
            created_at: 1,
            provisional_pid: Some(p(8)),
            released_at: IcOps::now_secs(),
        }
    );
    assert_eq!(
        DirectoryRegistryOps::lookup_entry("projects", "alpha"),
        None
    );
}
