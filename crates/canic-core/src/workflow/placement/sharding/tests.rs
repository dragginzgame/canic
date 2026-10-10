use super::*;
use crate::{
    cdk::types::Principal,
    dto::placement::sharding::ShardingPlanStateResponse,
    ops::storage::{children::CanisterChildrenOps, placement::sharding::ShardingRegistryOps},
    storage::stable::sharding::registry::ShardingRegistry,
    test::{
        seams::{lock, p},
        support::{direct_child, init_sharding_test_config},
    },
};
use canic_contracts::ids::CanisterRole;
use futures::executor::block_on;

fn install_shards() -> (Principal, Principal) {
    init_sharding_test_config();
    ShardingRegistryOps::clear_for_test();
    let role = CanisterRole::new("shard");
    let first = p(7);
    let second = p(8);
    ShardingRegistryOps::create(first, "primary", 0, &role, 2, [1; 32], 10).unwrap();
    ShardingRegistryOps::create(second, "primary", 1, &role, 2, [1; 32], 20).unwrap();
    CanisterChildrenOps::import_direct_children(
        p(2),
        vec![
            direct_child(first, role.clone(), [1; 32]),
            direct_child(second, role, [1; 32]),
        ],
    );
    ShardingRegistryOps::assign("primary", "key", first).unwrap();
    (first, second)
}

fn set_active(shard: Principal, active: bool) {
    ShardingRegistry::with_mut(|core| {
        let mut entry = core.get_entry(&shard).unwrap();
        entry.active = active;
        core.insert_entry(shard, entry);
    });
}

#[test]
fn unavailable_assignment_preserves_ownership_before_allocation_or_reassignment() {
    let _guard = lock();
    let (first, second) = install_shards();
    let cfg = ShardingWorkflow::get_shard_pool_cfg("primary").unwrap();
    set_active(first, false);

    // Cover both spare existing capacity and the otherwise-empty bootstrap branch.
    for second_active in [true, false] {
        set_active(second, second_active);
        let error = ShardingWorkflow::plan_assign_to_pool("primary", "key").unwrap_err();
        assert_eq!(
            error.code(),
            crate::diagnostics::codes::POSITION_UNAVAILABLE
        );
        let error = block_on(ShardingWorkflow::assign_with_policy(
            &cfg.canister_role,
            "primary",
            "key",
            cfg.policy.clone(),
            None,
        ))
        .unwrap_err();
        assert_eq!(
            error.code(),
            crate::diagnostics::codes::POSITION_UNAVAILABLE
        );
        assert_eq!(
            ShardingRegistryOps::assignment_for_key("primary", "key").map(|record| record.shard),
            Some(first)
        );
        assert_eq!(ShardingRegistryOps::get(first).unwrap().count, 1);
        assert_eq!(ShardingRegistryOps::get(second).unwrap().count, 0);
    }

    set_active(first, true);
    assert!(matches!(
        ShardingWorkflow::plan_assign_to_pool("primary", "key").unwrap(),
        ShardingPlanStateResponse::AlreadyAssigned { pid } if pid == first
    ));
    assert_eq!(
        block_on(ShardingWorkflow::assign_with_policy(
            &cfg.canister_role,
            "primary",
            "key",
            cfg.policy,
            None,
        ))
        .unwrap(),
        first
    );
}

#[test]
fn late_completion_uses_current_routing_and_keeps_the_first_mapping() {
    let _guard = lock();
    let (first, second) = install_shards();
    assert_eq!(
        ShardingWorkflow::assign_available_key("primary", "key", second).unwrap(),
        first
    );
    set_active(first, false);
    assert_eq!(
        ShardingWorkflow::assign_available_key("primary", "key", second)
            .unwrap_err()
            .code(),
        crate::diagnostics::codes::POSITION_UNAVAILABLE,
    );
    assert_eq!(ShardingRegistryOps::get(first).unwrap().count, 1);
    assert_eq!(ShardingRegistryOps::get(second).unwrap().count, 0);

    // A missing child directory never grants routing authority to registry rows.
    CanisterChildrenOps::import_direct_children(p(2), vec![]);
    assert_eq!(
        ShardingWorkflow::assign_available_key("primary", "new", second)
            .unwrap_err()
            .code(),
        crate::diagnostics::codes::POSITION_UNAVAILABLE,
    );
    assert!(
        ShardingRegistryOps::assignment_for_key("primary", "new")
            .map(|record| record.shard)
            .is_none()
    );
    CanisterChildrenOps::import_direct_children(
        p(2),
        vec![direct_child(second, CanisterRole::new("shard"), [1; 32])],
    );
    assert_eq!(
        ShardingWorkflow::assign_available_key("primary", "new", second).unwrap(),
        second
    );
    assert_eq!(ShardingRegistryOps::get(second).unwrap().count, 1);
}

#[test]
fn recycled_shard_does_not_inherit_routes_or_old_assignment_counts() {
    let _guard = lock();
    let (first, second) = install_shards();
    let role = CanisterRole::new("shard");
    CanisterChildrenOps::import_direct_children(
        p(2),
        vec![
            direct_child(first, role.clone(), [2; 32]),
            direct_child(second, role.clone(), [1; 32]),
        ],
    );
    assert_eq!(
        query::ShardingQuery::lookup_partition_key("primary", "key"),
        None
    );
    assert_eq!(
        ShardingWorkflow::assign_available_key("primary", "key", second)
            .unwrap_err()
            .code(),
        crate::diagnostics::codes::POSITION_UNAVAILABLE
    );
    ShardingRegistryOps::create(first, "primary", 0, &role, 2, [2; 32], 30).unwrap();
    assert_eq!(
        query::ShardingQuery::lookup_partition_key("primary", "key"),
        None
    );
    assert_eq!(
        ShardingWorkflow::assign_available_key("primary", "fresh", first).unwrap(),
        first
    );
    assert_eq!(ShardingRegistryOps::get(first).unwrap().count, 1);
    assert_eq!(
        query::ShardingQuery::partition_keys("primary", first).0,
        vec!["fresh"]
    );
    ShardingWorkflow::release_partition_key("primary", "key").unwrap();
    assert_eq!(ShardingRegistryOps::get(first).unwrap().count, 1);
    assert_eq!(
        ShardingWorkflow::assign_available_key("primary", "key", first).unwrap(),
        first
    );
    assert_eq!(ShardingRegistryOps::get(first).unwrap().count, 2);
}

#[test]
fn removal_frees_shard_slot_without_scanning_or_discarding_partition_assignments() {
    let _guard = lock();
    let (first, second) = install_shards();
    let replacement = p(9);
    let role = CanisterRole::new("shard");
    CanisterChildrenOps::import_direct_children(
        p(2),
        vec![
            direct_child(replacement, role.clone(), [3; 32]),
            direct_child(second, role.clone(), [1; 32]),
        ],
    );
    ShardingRegistryOps::create(replacement, "primary", 0, &role, 2, [3; 32], 30).unwrap();
    assert_eq!(
        ShardingRegistryOps::assignment_for_key("primary", "key").map(|record| record.shard),
        Some(first)
    );
    assert_eq!(
        query::ShardingQuery::lookup_partition_key("primary", "key"),
        None
    );
    assert_eq!(
        ShardingWorkflow::assign_available_key("primary", "fresh", replacement).unwrap(),
        replacement
    );
}
