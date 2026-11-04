//! Durable ordinary allocation continuation, fair selection and ownership regressions.

use super::*;
use canic_contracts::diagnostics::codes;

#[test]
fn allocation_retry_budget_survives_interruption_replay_and_exact_resume() {
    use crate::ops::continuation_retry as retry;
    use canic_contracts::dto::operation_retry::{Operation, ResumeRequest, State};
    let (_, allocation, _) = prepared_created_allocation();
    ComponentRegistryOps::request_allocation_continuation(allocation.operation_id).unwrap();
    let operation = Operation::RootAllocation {
        operation_id: allocation.operation_id,
    };
    let original = RootComponentRegistryStore::export();
    let maximum = retry::status(operation)
        .unwrap()
        .maximum_attempts_without_progress;
    for attempt in 1..=maximum {
        let now = retry::status(operation).unwrap().retry_at_ns;
        let claim = retry::begin(operation, now).unwrap();
        if attempt != maximum {
            let error = if attempt % 2 == 0 {
                InternalError::unavailable()
            } else {
                InternalError::resource_exhausted()
            };
            assert!(!retry::finish(claim, Some(&error)).unwrap());
        }
        // The last callback is interrupted after dispatch. Restart must retain its charge.
        let retained = restart_component_registry();
        ComponentRegistryOps::request_allocation_continuation(allocation.operation_id).unwrap();
        assert_eq!(RootComponentRegistryStore::export(), retained);
        assert_eq!(
            retry::status(operation).unwrap().attempts_without_progress,
            attempt
        );
    }
    assert_eq!(
        retry::status(operation).unwrap().state,
        State::BudgetExhausted
    );
    assert!(retry::begin(operation, u64::MAX).is_err());
    let mut effects = RootComponentRegistryStore::export();
    for allocation in &mut effects.allocations {
        allocation.retry =
            crate::storage::stable::continuation_retry::ContinuationRetryRecord::default();
    }
    assert_eq!(effects, original);
    let request = ResumeRequest {
        operation,
        expected_generation: 0,
    };
    retry::resume_operation(request).unwrap();
    let claim = retry::begin(operation, 0).unwrap();
    retry::finish(claim, Some(&InternalError::unavailable())).unwrap();
    let retained = RootComponentRegistryStore::export();
    retry::resume_operation(request).unwrap();
    assert_eq!(RootComponentRegistryStore::export(), retained);
    assert!(
        retry::resume_operation(ResumeRequest {
            expected_generation: 2,
            ..request
        })
        .is_err()
    );
    assert_eq!(RootComponentRegistryStore::export(), retained);
}

#[test]
fn allocation_retry_deadline_does_not_delay_ready_peers() {
    use crate::ops::continuation_retry::{self as retry, Selection};
    use canic_contracts::dto::operation_retry::Operation;
    let (_, first, _) = prepared_created_allocation();
    let second = ComponentRegistryOps::reserve_allocation(
        TopLevelComponentAllocationDecision {
            allocation_sequence: 2,
            component: ComponentInstanceId::from_generated_bytes([31; 32]),
            component_spec: first.component_spec,
            spec_hash: first.spec_hash,
            role: first.role,
        },
        [13; 32],
        first.provisioning_origin,
        false,
    )
    .unwrap();
    for id in [first.operation_id, second.operation_id] {
        ComponentRegistryOps::request_allocation_continuation(id).unwrap();
    }
    let first = Operation::RootAllocation {
        operation_id: first.operation_id,
    };
    let second = Operation::RootAllocation {
        operation_id: second.operation_id,
    };
    let attempt = retry::begin(first, 0).unwrap();
    retry::finish(attempt, None).unwrap();
    assert!(
        matches!(retry::select(vec![first, second], None, 0).unwrap(), Selection::Ready(operation) if operation == second)
    );
    assert!(matches!(
        retry::select(vec![first], None, 0).unwrap(),
        Selection::WaitUntil(_)
    ));
    assert!(retry::status(first).unwrap().diagnostic_code.is_none());
}

#[test]
fn allocation_continuation_survives_restart_without_changing_effects_or_capacity() {
    let (_, allocation, _) = prepared_created_allocation();
    assert_eq!(
        ComponentRegistryOps::next_allocation_continuation(None).unwrap(),
        None
    );
    let mut expected = RootComponentRegistryStore::export();
    ComponentRegistryOps::request_allocation_continuation(allocation.operation_id).unwrap();
    expected
        .allocations
        .iter_mut()
        .find(|record| record.operation_id == allocation.operation_id)
        .unwrap()
        .continuation_requested = true;
    assert_eq!(RootComponentRegistryStore::export(), expected);
    RootComponentRegistryStore::import(expected.clone());
    ComponentRegistryOps::request_allocation_continuation(allocation.operation_id).unwrap();
    assert_eq!(RootComponentRegistryStore::export(), expected);
    assert_eq!(
        ComponentRegistryOps::next_allocation_continuation(None).unwrap(),
        Some(allocation.operation_id)
    );
    assert_eq!(
        ComponentRegistryOps::request_allocation_continuation([99; 32])
            .unwrap_err()
            .public_code(),
        Some(codes::STATE_UNAVAILABLE)
    );
    assert_eq!(RootComponentRegistryStore::export(), expected);
}

#[test]
fn allocation_continuation_rotates_and_exact_reservation_replay_preserves_demand() {
    let (_, first, _) = prepared_created_allocation();
    let decision = TopLevelComponentAllocationDecision {
        allocation_sequence: 2,
        component: ComponentInstanceId::from_generated_bytes([31; 32]),
        component_spec: first.component_spec,
        spec_hash: first.spec_hash,
        role: first.role,
    };
    let origin = first.provisioning_origin;
    let second =
        ComponentRegistryOps::reserve_allocation(decision.clone(), [13; 32], origin.clone(), false)
            .unwrap();
    for id in [first.operation_id, second.operation_id] {
        ComponentRegistryOps::request_allocation_continuation(id).unwrap();
    }
    let retained = RootComponentRegistryStore::export();
    assert_eq!(
        ComponentRegistryOps::reserve_allocation(decision, [13; 32], origin, false).unwrap(),
        second
    );
    assert_eq!(RootComponentRegistryStore::export(), retained);
    for (previous, expected) in [
        (None, first.operation_id),
        (Some(first.operation_id), second.operation_id),
        (Some(second.operation_id), first.operation_id),
    ] {
        assert_eq!(
            ComponentRegistryOps::next_allocation_continuation(previous).unwrap(),
            Some(expected)
        );
    }
}

#[test]
fn allocation_continuation_excludes_terminal_and_aggregate_owned_operations() {
    let fixture = import_active_component_tree();
    let before = import_terminal_allocation(&fixture);
    let id = before.allocations[0].operation_id;
    ComponentRegistryOps::request_allocation_continuation(id).unwrap();
    assert_eq!(RootComponentRegistryStore::export(), before);
    assert_eq!(
        ComponentRegistryOps::next_allocation_continuation(None).unwrap(),
        None
    );
    let fixture = import_grouped_active_component_tree();
    let before = import_terminal_allocation(&fixture);
    assert_eq!(
        ComponentRegistryOps::request_allocation_continuation(before.allocations[0].operation_id)
            .unwrap_err()
            .public_code(),
        Some(codes::STATE_CONFLICT)
    );
    assert_eq!(RootComponentRegistryStore::export(), before);
    assert_eq!(
        ComponentRegistryOps::next_allocation_continuation(None).unwrap(),
        None
    );
}

fn import_terminal_allocation(fixture: &ActiveComponentTreeFixture) -> RootComponentRegistryData {
    let mut data = RootComponentRegistryStore::export();
    data.allocations
        .push(active_component_allocation(&fixture.partition));
    RootComponentRegistryStore::import(data.clone());
    data
}
