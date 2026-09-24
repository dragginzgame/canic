//! Focused retained child failure, capacity and progress regressions.

use super::*;
use canic_core::{
    control_plane_support::error::{ProvisioningFailureStage, ProvisioningRetryCategory},
    diagnostics::codes,
};

fn bootstrap_fixture() -> (ActiveComponentTreeFixture, RootComponentChildAllocationView) {
    let (fixture, allocation) = reserve();
    let mut data = RootComponentRegistryStore::export();
    for child in &mut data.child_allocations {
        child.initial_bootstrap = true;
    }
    set_partition_status(&mut data, ComponentLifecycleStatus::Prepared);
    RootComponentRegistryStore::import(data);
    (fixture, allocation)
}

fn set_partition_status(data: &mut RootComponentRegistryData, status: ComponentLifecycleStatus) {
    for partition in &mut data.partitions {
        partition.status = status;
        partition.content_hash = component_partition_content_hash(
            &partition.binding,
            partition.protocol_profile_digest,
            &partition.provisioning_origin,
            partition.release_set,
            status,
            partition.revision,
            partition.descendant_content_hash,
            partition.committed_descendants,
        )
        .unwrap();
    }
}

pub(super) fn assert_terminal_origin_excluded(data: &RootComponentRegistryData) {
    let mut prepared = data.clone();
    let child = prepared.child_allocations.first_mut().unwrap();
    assert!(child_allocation_is_terminal(child));
    child.initial_bootstrap = true;
    let component = child.component;
    let operation_id = child.operation_id;
    set_partition_status(&mut prepared, ComponentLifecycleStatus::Prepared);
    RootComponentRegistryStore::import(prepared);
    ComponentRegistryOps::record_child_failure(
        component,
        operation_id,
        &InternalError::unavailable(),
        100,
    )
    .unwrap();
    assert!(
        ComponentRegistryOps::initial_child_failure(component, &InternalError::unavailable())
            .unwrap()
            .is_none()
    );
    RootComponentRegistryStore::import(data.clone());
}

#[test]
fn initial_child_origin_survives_outer_failure_context_without_changing_state() {
    let (fixture, allocation) = bootstrap_fixture();
    let failure = ComponentRegistryOps::record_child_failure(
        fixture.component,
        allocation.operation_id,
        &InternalError::public(codes::DEPLOYMENT_CYCLE_RESERVE_REQUIRED),
        100,
    )
    .unwrap();
    let before = restart_component_registry();
    for (parent_error, stage) in [
        (
            InternalError::public(codes::STATE_INVALID),
            ProvisioningFailureStage::ComponentRuntime,
        ),
        (
            InternalError::unavailable(),
            ProvisioningFailureStage::ComponentMembership,
        ),
    ] {
        let parent_code = parent_error.public_error().code();
        let origin = ComponentRegistryOps::initial_child_failure(fixture.component, &parent_error)
            .unwrap()
            .unwrap();
        assert_eq!(origin.recorded_at_ns, Some(failure.failed_at_ns));
        assert_eq!(origin.retry_at_ns, Some(failure.retry_at_ns));
        assert_eq!(
            origin.stage,
            ProvisioningFailureStage::ComponentChildAllocation
        );
        assert_eq!(origin.target, root_binding().fleet_subnet_root);
        assert_eq!(origin.operation_id, allocation.operation_id);
        assert_eq!(origin.diagnostic_code, failure.diagnostic_code);
        assert_eq!(origin.retry_category, ProvisioningRetryCategory::Backoff);
        let error = parent_error
            .with_observed_provisioning_failure(origin)
            .with_provisioning_failure(
                stage,
                fixture.partition.binding.canister_id,
                [92; 32],
                ProvisioningRetryCategory::Backoff,
            );
        assert_eq!(error.provisioning_failure(), Some(origin));
        assert_eq!(error.public_error().code(), parent_code);
    }
    assert_eq!(RootComponentRegistryStore::export(), before);
}

#[test]
fn initial_child_origin_excludes_dynamic_active_and_unrelated_failures() {
    let (fixture, allocation) = bootstrap_fixture();
    ComponentRegistryOps::record_child_failure(
        fixture.component,
        allocation.operation_id,
        &InternalError::public(codes::DEPLOYMENT_CYCLE_RESERVE_REQUIRED),
        100,
    )
    .unwrap();
    let data = RootComponentRegistryStore::export();
    let parent_error = InternalError::unavailable();
    for error in [
        InternalError::forbidden(),
        InternalError::conflict(),
        InternalError::unavailable().with_provisioning_failure(
            ProvisioningFailureStage::StoreStatus,
            candid::Principal::from_slice(&[94]),
            [95; 32],
            ProvisioningRetryCategory::Backoff,
        ),
    ] {
        assert!(
            ComponentRegistryOps::initial_child_failure(fixture.component, &error)
                .unwrap()
                .is_none()
        );
    }
    let mut dynamic = data.clone();
    for child in &mut dynamic.child_allocations {
        child.initial_bootstrap = false;
    }
    RootComponentRegistryStore::import(dynamic);
    assert!(
        ComponentRegistryOps::initial_child_failure(fixture.component, &parent_error)
            .unwrap()
            .is_none()
    );
    let mut active = data.clone();
    set_partition_status(&mut active, ComponentLifecycleStatus::Active);
    RootComponentRegistryStore::import(active);
    assert!(
        ComponentRegistryOps::initial_child_failure(fixture.component, &parent_error)
            .unwrap()
            .is_none()
    );
    let mut unrelated = data;
    for child in &mut unrelated.child_allocations {
        child.component = ComponentInstanceId::from_generated_bytes([99; 32]);
        child.reserved_against_registry.component = child.component;
    }
    RootComponentRegistryStore::import(unrelated);
    assert!(
        ComponentRegistryOps::initial_child_failure(fixture.component, &parent_error)
            .unwrap()
            .is_none()
    );
}

fn reserve() -> (ActiveComponentTreeFixture, RootComponentChildAllocationView) {
    let fixture = import_active_component_tree();
    let allocation = ComponentRegistryOps::reserve_child_allocation(
        child_allocation_decision(&fixture.partition, "project_machine"),
        [91; 32],
        None,
        component_registry_head(&fixture.partition),
    )
    .unwrap();
    (fixture, allocation)
}

#[test]
fn retained_child_failure_does_not_change_work_or_capacity() {
    let (fixture, allocation) = reserve();
    let before = RootComponentRegistryStore::export();
    let error = InternalError::resource_exhausted();
    let first = ComponentRegistryOps::record_child_failure(
        fixture.component,
        allocation.operation_id,
        &error,
        100,
    )
    .unwrap();
    assert_eq!(first.diagnostic_code, error.public_error().raw_code());
    assert_eq!(first.retry_at_ns - first.failed_at_ns, 1_000_000_000);
    let repeated = ComponentRegistryOps::record_child_failure(
        fixture.component,
        allocation.operation_id,
        &error,
        first.retry_at_ns,
    )
    .unwrap();
    assert_eq!(repeated.consecutive_failures, 2);
    assert_eq!(repeated.retry_at_ns - repeated.failed_at_ns, 2_000_000_000);
    let retained = restart_component_registry();
    let mut normalized = retained.clone();
    for record in &mut normalized.child_allocations {
        record.last_failure = None;
    }
    assert_eq!(normalized, before);
    assert_eq!(
        exact_registry_entry_bytes(&retained),
        exact_registry_entry_bytes(&before)
    );
    ComponentRegistryOps::clear_child_failure_after_progress(&allocation, false).unwrap();
    assert_eq!(RootComponentRegistryStore::export(), retained);
    let view = ComponentRegistryOps::child_allocation(fixture.component, allocation.operation_id)
        .unwrap()
        .unwrap();
    assert_eq!(view.last_failure, Some(repeated));
    let response = ComponentRegistryOps::child_failure_response(repeated);
    assert_eq!(response.diagnostic_code, error.public_error().raw_code());
    assert_eq!(response.consecutive_failures, 2);
}

#[test]
fn retained_child_failure_clears_after_progress_and_preserves_effects() {
    let (fixture, allocation) = reserve();
    ComponentRegistryOps::record_child_failure(
        fixture.component,
        allocation.operation_id,
        &InternalError::resource_exhausted(),
        100,
    )
    .unwrap();
    ComponentRegistryOps::begin_child_creation(
        fixture.component,
        allocation.operation_id,
        child_creation_plan(&root_binding(), 92),
        ReplayCostGuardSettlement {
            quota_intent_id: IntentId(93),
            reservation_intent_id: IntentId(94),
        },
    )
    .unwrap();
    let progressed = RootComponentRegistryStore::export();
    ComponentRegistryOps::clear_child_failure_after_progress(&allocation, false).unwrap();
    let mut expected = progressed;
    for record in &mut expected.child_allocations {
        record.last_failure = None;
    }
    assert_eq!(RootComponentRegistryStore::export(), expected);
    let view = ComponentRegistryOps::child_allocation(fixture.component, allocation.operation_id)
        .unwrap()
        .unwrap();
    assert!(view.last_failure.is_none());
    assert!(matches!(
        view.progress,
        RootComponentChildAllocationProgressView::CreationIntent(_)
    ));
}

#[test]
fn retained_child_failure_rejects_wrong_identity_and_time_without_mutation() {
    let (fixture, allocation) = reserve();
    let error = InternalError::resource_exhausted();
    ComponentRegistryOps::record_child_failure(
        fixture.component,
        allocation.operation_id,
        &error,
        100,
    )
    .unwrap();
    let before = RootComponentRegistryStore::export();
    for (operation, time) in [
        ([99; 32], 101),
        (allocation.operation_id, 99),
        (allocation.operation_id, u64::MAX),
    ] {
        assert!(
            ComponentRegistryOps::record_child_failure(fixture.component, operation, &error, time)
                .is_err()
        );
        assert_eq!(RootComponentRegistryStore::export(), before);
    }
}
