//! Retained retirement demand, exact replay and fair selection regressions.

use super::*;
use crate::view::component_registry::RootComponentRemovalKind;
use canic_contracts::diagnostics::codes;

#[test]
fn component_retry_refusal_preserves_paid_journal_and_reserved_capacity() {
    use crate::ops::continuation_retry as retry;
    use canic_contracts::dto::operation_retry::{Operation, ResumeRequest, State};
    let (partition, draining, _) = import_empty_quiescent_component();
    ComponentRegistryOps::request_component_removal_continuation(
        partition.binding.component,
        draining.operation_id,
    )
    .unwrap();
    let operation = Operation::ComponentRemoval {
        component: partition.binding.component,
        operation_id: draining.operation_id,
    };
    let original = RootComponentRegistryStore::export();
    let attempt = retry::begin(operation, 0).unwrap();
    retry::finish(attempt, Some(&InternalError::conflict())).unwrap();
    let suspended = retry::status(operation).unwrap();
    assert_eq!(suspended.state, State::ReviewRequired);
    restart_component_registry();
    ComponentRegistryOps::request_component_removal_continuation(
        partition.binding.component,
        draining.operation_id,
    )
    .unwrap();
    assert_eq!(retry::status(operation).unwrap(), suspended);
    let mut effects = RootComponentRegistryStore::export();
    for record in &mut effects.component_drainings {
        record.retry =
            crate::storage::stable::continuation_retry::ContinuationRetryRecord::default();
    }
    assert_eq!(effects, original);
    retry::resume_operation(ResumeRequest {
        operation,
        expected_generation: 0,
    })
    .unwrap();
    assert_eq!(retry::status(operation).unwrap().state, State::Active);
}

#[test]
fn subtree_retry_budget_preserves_cursor_and_capacity_across_restart() {
    use crate::ops::continuation_retry as retry;
    use canic_contracts::dto::operation_retry::{Operation, State};
    let fixture = import_active_component_tree();
    ComponentRegistryOps::begin_subtree_removal(
        fixture.component,
        [1; 32],
        fixture.target.canister_id,
        component_registry_head(&fixture.partition),
        16_777_216,
    )
    .unwrap();
    ComponentRegistryOps::request_subtree_removal_continuation(fixture.component, [1; 32]).unwrap();
    let operation = Operation::SubtreeRemoval {
        component: fixture.component,
        operation_id: [1; 32],
    };
    let original = RootComponentRegistryStore::export();
    for _ in 0..retry::status(operation)
        .unwrap()
        .maximum_attempts_without_progress
    {
        let attempt =
            retry::begin(operation, retry::status(operation).unwrap().retry_at_ns).unwrap();
        retry::finish(attempt, None).unwrap();
        restart_component_registry();
        ComponentRegistryOps::request_subtree_removal_continuation(fixture.component, [1; 32])
            .unwrap();
    }
    assert_eq!(
        retry::status(operation).unwrap().state,
        State::BudgetExhausted
    );
    let mut effects = RootComponentRegistryStore::export();
    for record in &mut effects.subtree_removals {
        record.retry =
            crate::storage::stable::continuation_retry::ContinuationRetryRecord::default();
    }
    assert_eq!(effects, original);
}

#[test]
fn component_removal_continuation_preserves_progress_and_capacity_across_restart() {
    let (partition, draining, _) = import_empty_quiescent_component();
    assert_eq!(
        ComponentRegistryOps::next_removal_continuation(None).unwrap(),
        None
    );
    let mut expected = RootComponentRegistryStore::export();
    ComponentRegistryOps::request_component_removal_continuation(
        partition.binding.component,
        draining.operation_id,
    )
    .unwrap();
    expected.component_drainings[0].continuation_requested = true;
    assert_eq!(restart_component_registry(), expected);
    ComponentRegistryOps::request_component_removal_continuation(
        partition.binding.component,
        draining.operation_id,
    )
    .unwrap();
    assert_eq!(RootComponentRegistryStore::export(), expected);
    let next = ComponentRegistryOps::next_removal_continuation(None)
        .unwrap()
        .unwrap();
    assert_eq!(next.component, partition.binding.component);
    assert_eq!(next.operation_id, draining.operation_id);
    assert_eq!(next.kind, RootComponentRemovalKind::Component);
    assert_eq!(
        ComponentRegistryOps::request_component_removal_continuation(
            partition.binding.component,
            [1; 32]
        )
        .unwrap_err()
        .public_code(),
        Some(codes::STATE_CONFLICT)
    );
    assert_eq!(RootComponentRegistryStore::export(), expected);
}

#[test]
fn subtree_removal_continuation_preserves_journal_and_capacity() {
    let fixture = import_active_component_tree();
    ComponentRegistryOps::begin_subtree_removal(
        fixture.component,
        [1; 32],
        fixture.target.canister_id,
        component_registry_head(&fixture.partition),
        16_777_216,
    )
    .unwrap();
    assert_eq!(
        ComponentRegistryOps::next_removal_continuation(None).unwrap(),
        None
    );
    let mut expected = RootComponentRegistryStore::export();
    ComponentRegistryOps::request_subtree_removal_continuation(fixture.component, [1; 32]).unwrap();
    for record in &mut expected.subtree_removals {
        record.continuation_requested = true;
    }
    assert_eq!(restart_component_registry(), expected);
    ComponentRegistryOps::request_subtree_removal_continuation(fixture.component, [1; 32]).unwrap();
    assert_eq!(RootComponentRegistryStore::export(), expected);
    let first = ComponentRegistryOps::next_removal_continuation(None)
        .unwrap()
        .unwrap();
    let second = ComponentRegistryOps::next_removal_continuation(Some(first))
        .unwrap()
        .unwrap();
    assert_eq!(first.kind, RootComponentRemovalKind::Subtree);
    assert_eq!(first.operation_id, [1; 32]);
    assert_eq!(second, first);
    assert_eq!(
        ComponentRegistryOps::next_removal_continuation(Some(second)).unwrap(),
        Some(first)
    );
    assert_eq!(
        ComponentRegistryOps::request_subtree_removal_continuation(fixture.component, [3; 32])
            .unwrap_err()
            .public_code(),
        Some(codes::STATE_UNAVAILABLE)
    );
    assert_eq!(RootComponentRegistryStore::export(), expected);
}
