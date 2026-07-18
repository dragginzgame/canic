//! Module: workflow::placement::allocation
//!
//! Responsibility: compose receipt-backed admission with replayed root child creation.
//! Does not own: placement policy, domain registry mutation, or root replay storage.
//! Boundary: placement workflows register the returned child before settling the permit.

use crate::{
    InternalError, InternalErrorOrigin,
    cdk::types::Principal,
    dto::{error::Error, rpc::CreateCanisterParent},
    ids::CanisterRole,
    log,
    log::Topic,
    model::{
        intent::{
            BeginReceiptBackedIntentInput, BeginReceiptBackedIntentResult, ReceiptBackedIntent,
            ReceiptBackedIntentState, RemoveTerminalReceiptBackedIntentInput,
            RemoveTerminalReceiptBackedIntentResult, SettleReceiptBackedIntentInput,
            SettleReceiptBackedIntentResult, TerminalEvidence, TerminalEvidenceDecision,
        },
        placement::allocation::PlacementAllocationIdentity,
        replay::{OperationId, ReplayPayloadHasher},
    },
    ops::{
        rpc::request::RequestOps,
        runtime::{env::EnvOps, timer::TimerId},
        storage::intent::{IntentStoreOps, ReceiptBackedIntentOps},
    },
    workflow::runtime::{intent::ReceiptBackedIntentWorkflow, timer::TimerWorkflow},
};
use std::{
    cell::{Cell, RefCell},
    time::Duration,
};

const ALLOCATION_RESULT_COMMAND: &str = "placement.allocate_child.result";
const PLACEMENT_RESOURCE_PREFIX: &str = "placement:";
const ROOT_RECEIPT_ACK_BATCH_SIZE: usize = 32;
const ROOT_RECEIPT_ACK_RETRY_DELAY: Duration = Duration::from_mins(1);

thread_local! {
    static ROOT_RECEIPT_ACK_TIMER: RefCell<Option<TimerId>> = const { RefCell::new(None) };
    static ROOT_RECEIPT_ACK_CURSOR: Cell<Option<OperationId>> = const { Cell::new(None) };
    static ROOT_RECEIPT_ACK_RETRY_NEEDED: Cell<bool> = const { Cell::new(false) };
}

///
/// PlacementAllocationRequest
///
/// Complete shared input for one receipt-backed child creation attempt.
///

#[derive(Clone, Debug)]
pub struct PlacementAllocationRequest {
    pub identity: PlacementAllocationIdentity,
    pub canister_role: CanisterRole,
    pub extra_arg: Option<Vec<u8>>,
    pub reservation_limit: u64,
}

///
/// PlacementAllocationPermit
///
/// Durable intent identity required to settle a successfully registered child.
///

#[derive(Clone, Debug)]
pub struct PlacementAllocationPermit {
    identity: PlacementAllocationIdentity,
    revision: u64,
}

///
/// PlacementAllocationWorkflow
///
/// Shared child-allocation orchestration used by placement strategies.
///

pub struct PlacementAllocationWorkflow;

impl PlacementAllocationWorkflow {
    /// Return the current committed allocation sequence for one capacity resource.
    ///
    /// Pending callers deliberately reuse this value so retries and concurrent
    /// admission converge on the same root operation until it settles.
    #[must_use]
    pub fn next_sequence(identity: &PlacementAllocationIdentity) -> u64 {
        IntentStoreOps::totals(&identity.resource_key).committed_qty
    }

    /// Translate currently available live capacity into the intent ledger's cumulative limit.
    #[must_use]
    pub fn reservation_limit_for_available_capacity(
        identity: &PlacementAllocationIdentity,
        available_capacity: u64,
    ) -> u64 {
        IntentStoreOps::totals(&identity.resource_key)
            .committed_qty
            .saturating_add(available_capacity)
    }

    /// Reserve local capacity and execute or recover one root child creation.
    pub async fn create_child(
        request: PlacementAllocationRequest,
    ) -> Result<(PlacementAllocationPermit, Principal), InternalError> {
        let permit = begin_allocation(&request)?;
        let response = RequestOps::allocate_placement_child::<Vec<u8>>(
            &request.canister_role,
            CreateCanisterParent::ThisCanister,
            request.extra_arg,
            permit.identity.operation_id,
        )
        .await?;

        Ok((permit, response.new_canister_pid))
    }

    /// Recover a previously admitted create; never invent a new operation for unknown history.
    pub async fn recover_child(
        request: PlacementAllocationRequest,
    ) -> Result<(PlacementAllocationPermit, Principal), InternalError> {
        if ReceiptBackedIntentOps::load(request.identity.operation_id)?.is_none() {
            return Err(InternalError::public(Error::conflict(format!(
                "placement allocation {} has no durable intent and cannot be resumed safely",
                request.identity.operation_id
            ))));
        }
        Self::create_child(request).await
    }

    /// Load or create the local permit when durable domain state already proves the result.
    pub fn resume_permit(
        request: &PlacementAllocationRequest,
    ) -> Result<PlacementAllocationPermit, InternalError> {
        begin_allocation(request)
    }

    /// Settle local capacity only after the domain registry owns the returned child.
    pub fn commit_registered_child(
        permit: &PlacementAllocationPermit,
        child_pid: Principal,
    ) -> Result<(), InternalError> {
        settle_allocation(permit, child_pid, TerminalEvidenceDecision::Committed)
    }

    /// Roll back local capacity after the domain owner proves the child was disposed.
    pub fn rollback_disposed_child(
        permit: &PlacementAllocationPermit,
        child_pid: Principal,
    ) -> Result<(), InternalError> {
        settle_allocation(permit, child_pid, TerminalEvidenceDecision::RolledBack)
    }

    /// Release the root replay response after local membership and intent settlement succeed.
    pub async fn acknowledge_root_receipt(
        permit: &PlacementAllocationPermit,
    ) -> Result<(), InternalError> {
        RequestOps::acknowledge_placement_receipt(permit.identity.operation_id).await?;
        remove_terminal_intent(
            permit.identity.operation_id,
            permit.identity.payload_binding,
        )
    }

    /// Schedule a bounded drain of durable terminal placement acknowledgements.
    pub fn schedule_root_receipt_acknowledgement_drain() {
        schedule_root_receipt_acknowledgement_drain(Duration::ZERO);
    }

    /// Commit registered membership, then best-effort release its retained root receipt.
    pub async fn finish_registered_child(
        permit: &PlacementAllocationPermit,
        child_pid: Principal,
    ) -> Result<(), InternalError> {
        Self::commit_registered_child(permit, child_pid)?;
        acknowledge_root_receipt_best_effort(permit, child_pid).await;
        Ok(())
    }

    /// Roll back a disposed child, then best-effort release its retained root receipt.
    pub async fn finish_disposed_child(
        permit: &PlacementAllocationPermit,
        child_pid: Principal,
    ) -> Result<(), InternalError> {
        Self::rollback_disposed_child(permit, child_pid)?;
        acknowledge_root_receipt_best_effort(permit, child_pid).await;
        Ok(())
    }
}

fn settle_allocation(
    permit: &PlacementAllocationPermit,
    child_pid: Principal,
    decision: TerminalEvidenceDecision,
) -> Result<(), InternalError> {
    let evidence = allocation_terminal_evidence(&permit.identity, child_pid, decision)?;
    let result = ReceiptBackedIntentWorkflow::settle_if_pending(&SettleReceiptBackedIntentInput {
        operation_id: permit.identity.operation_id,
        expected_revision: permit.revision,
        expected_payload_binding: permit.identity.payload_binding,
        evidence,
    })?;

    match result {
        SettleReceiptBackedIntentResult::Settled { state, .. }
        | SettleReceiptBackedIntentResult::AlreadySettled { state, .. }
            if state_matches_decision(&state, decision) =>
        {
            Ok(())
        }
        SettleReceiptBackedIntentResult::Settled { state, .. }
        | SettleReceiptBackedIntentResult::AlreadySettled { state, .. } => {
            Err(InternalError::invariant(
                InternalErrorOrigin::Workflow,
                format!(
                    "placement allocation {} settled to unexpected state {state:?}",
                    permit.identity.operation_id
                ),
            ))
        }
        SettleReceiptBackedIntentResult::NotFound => Err(InternalError::invariant(
            InternalErrorOrigin::Workflow,
            format!(
                "placement allocation intent {} disappeared before settlement",
                permit.identity.operation_id
            ),
        )),
        SettleReceiptBackedIntentResult::RevisionConflict { actual_revision } => {
            Err(InternalError::invariant(
                InternalErrorOrigin::Workflow,
                format!(
                    "placement allocation intent {} revision changed from {} to {actual_revision}",
                    permit.identity.operation_id, permit.revision
                ),
            ))
        }
        SettleReceiptBackedIntentResult::BindingConflict => Err(InternalError::invariant(
            InternalErrorOrigin::Workflow,
            format!(
                "placement allocation intent {} payload binding changed before settlement",
                permit.identity.operation_id
            ),
        )),
    }
}

const fn state_matches_decision(
    state: &ReceiptBackedIntentState,
    decision: TerminalEvidenceDecision,
) -> bool {
    matches!(
        (state, decision),
        (
            ReceiptBackedIntentState::Committed { .. },
            TerminalEvidenceDecision::Committed
        ) | (
            ReceiptBackedIntentState::RolledBack { .. },
            TerminalEvidenceDecision::RolledBack
        )
    )
}

async fn acknowledge_root_receipt_best_effort(
    permit: &PlacementAllocationPermit,
    child_pid: Principal,
) {
    if let Err(err) = PlacementAllocationWorkflow::acknowledge_root_receipt(permit).await {
        log!(
            Topic::Rpc,
            Warn,
            "settled placement child but root placement receipt acknowledgement failed operation_id={} child={}: {err}",
            permit.identity.operation_id,
            child_pid
        );
        ROOT_RECEIPT_ACK_RETRY_NEEDED.set(true);
        schedule_root_receipt_acknowledgement_drain(ROOT_RECEIPT_ACK_RETRY_DELAY);
    }
}

fn schedule_root_receipt_acknowledgement_drain(delay: Duration) {
    let _ = TimerWorkflow::set_guarded(
        &ROOT_RECEIPT_ACK_TIMER,
        delay,
        "placement:receipt_ack",
        async {
            ROOT_RECEIPT_ACK_TIMER.with_borrow_mut(|slot| *slot = None);
            if let Some(next_delay) = drain_root_receipt_acknowledgements().await {
                schedule_root_receipt_acknowledgement_drain(next_delay);
            }
        },
    );
}

async fn drain_root_receipt_acknowledgements() -> Option<Duration> {
    let cursor = ROOT_RECEIPT_ACK_CURSOR.get();
    let page = match ReceiptBackedIntentOps::list_page(cursor, ROOT_RECEIPT_ACK_BATCH_SIZE) {
        Ok(page) => page,
        Err(err) => {
            log!(
                Topic::Rpc,
                Warn,
                "placement receipt acknowledgement queue scan failed: {err}"
            );
            ROOT_RECEIPT_ACK_RETRY_NEEDED.set(true);
            return Some(ROOT_RECEIPT_ACK_RETRY_DELAY);
        }
    };
    ROOT_RECEIPT_ACK_CURSOR.set(page.next_cursor);
    let mut page_failed = false;

    for intent in page.intents.into_iter().filter(|intent| {
        !matches!(intent.state, ReceiptBackedIntentState::Pending)
            && intent.resource_key.starts_with(PLACEMENT_RESOURCE_PREFIX)
    }) {
        let operation_id = intent.operation_id;
        let result = async {
            RequestOps::acknowledge_placement_receipt(operation_id).await?;
            remove_exact_terminal_intent(&intent)
        }
        .await;
        if let Err(err) = result {
            page_failed = true;
            ROOT_RECEIPT_ACK_RETRY_NEEDED.set(true);
            log!(
                Topic::Rpc,
                Warn,
                "placement receipt acknowledgement retry failed operation_id={operation_id}: {err}"
            );
        }
    }

    if page.next_cursor.is_some() {
        return Some(if page_failed {
            ROOT_RECEIPT_ACK_RETRY_DELAY
        } else {
            Duration::ZERO
        });
    }

    ROOT_RECEIPT_ACK_CURSOR.set(None);
    ROOT_RECEIPT_ACK_RETRY_NEEDED
        .replace(false)
        .then_some(ROOT_RECEIPT_ACK_RETRY_DELAY)
}

fn remove_terminal_intent(
    operation_id: OperationId,
    expected_payload_binding: crate::model::intent::PayloadBinding,
) -> Result<(), InternalError> {
    let Some(intent) = ReceiptBackedIntentOps::load(operation_id)? else {
        return Ok(());
    };
    if intent.payload_binding != expected_payload_binding {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Workflow,
            format!("placement allocation {operation_id} payload binding changed before cleanup"),
        ));
    }
    remove_exact_terminal_intent(&intent)
}

fn remove_exact_terminal_intent(intent: &ReceiptBackedIntent) -> Result<(), InternalError> {
    let result =
        ReceiptBackedIntentOps::remove_terminal(&RemoveTerminalReceiptBackedIntentInput {
            operation_id: intent.operation_id,
            expected_revision: intent.revision,
            expected_payload_binding: intent.payload_binding,
        })?;
    match result {
        RemoveTerminalReceiptBackedIntentResult::Removed
        | RemoveTerminalReceiptBackedIntentResult::NotFound => Ok(()),
        RemoveTerminalReceiptBackedIntentResult::NotTerminal => Err(InternalError::invariant(
            InternalErrorOrigin::Workflow,
            format!(
                "placement allocation {} is pending during receipt cleanup",
                intent.operation_id
            ),
        )),
        RemoveTerminalReceiptBackedIntentResult::RevisionConflict { actual_revision } => {
            Err(InternalError::invariant(
                InternalErrorOrigin::Workflow,
                format!(
                    "placement allocation {} revision changed from {} to {actual_revision} during receipt cleanup",
                    intent.operation_id, intent.revision
                ),
            ))
        }
        RemoveTerminalReceiptBackedIntentResult::BindingConflict => Err(InternalError::invariant(
            InternalErrorOrigin::Workflow,
            format!(
                "placement allocation {} payload binding changed during receipt cleanup",
                intent.operation_id
            ),
        )),
    }
}

fn begin_allocation(
    request: &PlacementAllocationRequest,
) -> Result<PlacementAllocationPermit, InternalError> {
    let input = BeginReceiptBackedIntentInput {
        operation_id: request.identity.operation_id,
        payload_binding: request.identity.payload_binding,
        resource_key: request.identity.resource_key.clone(),
        quantity: 1,
        reservation_limit: request.reservation_limit,
    };
    let revision = match ReceiptBackedIntentWorkflow::begin_or_load(&input)? {
        BeginReceiptBackedIntentResult::Created { revision }
        | BeginReceiptBackedIntentResult::ExistingPending { revision } => revision,
        BeginReceiptBackedIntentResult::ExistingCommitted { .. } => {
            return Err(InternalError::invariant(
                InternalErrorOrigin::Workflow,
                format!(
                    "placement allocation intent {} is committed but domain membership is absent",
                    request.identity.operation_id
                ),
            ));
        }
        BeginReceiptBackedIntentResult::ExistingRolledBack { .. } => {
            return Err(InternalError::public(Error::conflict(format!(
                "placement allocation {} was durably rolled back",
                request.identity.operation_id
            ))));
        }
        BeginReceiptBackedIntentResult::BindingConflict => {
            return Err(InternalError::invariant(
                InternalErrorOrigin::Workflow,
                format!(
                    "placement allocation operation {} has conflicting bound input",
                    request.identity.operation_id
                ),
            ));
        }
        BeginReceiptBackedIntentResult::CapacityExceeded {
            current_quantity,
            requested_quantity,
            limit,
        } => {
            return Err(InternalError::resource_exhausted(format!(
                "placement allocation capacity exceeded: current={current_quantity} requested={requested_quantity} limit={limit}"
            )));
        }
        BeginReceiptBackedIntentResult::StoreCapacityReached {
            current_records,
            limit,
        } => {
            return Err(InternalError::resource_exhausted(format!(
                "placement allocation intent capacity reached: current={current_records} limit={limit}"
            )));
        }
    };

    Ok(PlacementAllocationPermit {
        identity: request.identity.clone(),
        revision,
    })
}

fn allocation_terminal_evidence(
    identity: &PlacementAllocationIdentity,
    child_pid: Principal,
    decision: TerminalEvidenceDecision,
) -> Result<TerminalEvidence, InternalError> {
    let root_pid = EnvOps::root_pid()?;
    let command = crate::model::replay::CommandKind::new(ALLOCATION_RESULT_COMMAND)
        .expect("allocation result command kind is a valid static label");
    let actor = crate::model::replay::ReplayActor::direct_caller(root_pid);
    let mut hasher = ReplayPayloadHasher::new(&command, &actor);
    hasher.hash_bytes(identity.operation_id.as_bytes());
    hasher.hash_bytes(&identity.payload_binding.digest);
    hasher.hash_principal(&child_pid);
    hasher.hash_str(match decision {
        TerminalEvidenceDecision::Committed => "committed",
        TerminalEvidenceDecision::RolledBack => "rolled_back",
    });

    Ok(TerminalEvidence::new(root_pid, decision, hasher.finish()))
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        model::placement::allocation::PlacementAllocationIdentity,
        ops::storage::intent::IntentStoreOps,
        storage::stable::{
            env::Env,
            intent::{IntentStore, ReceiptBackedIntentStore},
        },
    };

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn reset_intents() {
        IntentStore::reset_for_tests();
        ReceiptBackedIntentStore::reset_for_tests();
        let mut env = Env::export();
        env.record.root_pid = Some(p(99));
        Env::import(env);
    }

    fn request(slot: u32, limit: u64) -> PlacementAllocationRequest {
        let role = CanisterRole::new("worker");
        PlacementAllocationRequest {
            identity: PlacementAllocationIdentity::scaling(
                p(1),
                "pool",
                u64::from(slot),
                &role,
                None,
            ),
            canister_role: role,
            extra_arg: None,
            reservation_limit: limit,
        }
    }

    #[test]
    fn begin_is_idempotent_and_pending_reservations_enforce_capacity() {
        reset_intents();
        let first = request(0, 1);
        let first_permit = begin_allocation(&first).expect("first allocation reserves");
        let replay_permit = begin_allocation(&first).expect("same allocation reloads");
        assert_eq!(first_permit.revision, replay_permit.revision);

        let error = begin_allocation(&request(1, 1)).expect_err("second allocation exceeds cap");
        assert!(error.is_public_resource_exhausted());
        let totals = IntentStoreOps::totals(&first.identity.resource_key);
        assert_eq!(totals.reserved_qty, 1);
        assert_eq!(totals.committed_qty, 0);
        assert_eq!(totals.pending_count, 1);
        assert_eq!(
            PlacementAllocationWorkflow::next_sequence(&first.identity),
            0,
            "pending callers must reuse the admitted operation sequence"
        );
    }

    #[test]
    fn recovery_rejects_untracked_operation_before_root_rpc() {
        reset_intents();

        let error =
            futures::executor::block_on(PlacementAllocationWorkflow::recover_child(request(0, 1)))
                .expect_err("untracked operation must not invent a replacement effect");
        assert_eq!(
            error.public_error().map(|error| error.code),
            Some(crate::dto::error::ErrorCode::Conflict)
        );
    }

    #[test]
    fn registered_child_settlement_is_idempotent_and_moves_capacity_once() {
        reset_intents();
        let request = request(0, 1);
        let permit = begin_allocation(&request).expect("allocation reserves");

        PlacementAllocationWorkflow::commit_registered_child(&permit, p(9))
            .expect("first settlement commits");
        PlacementAllocationWorkflow::commit_registered_child(&permit, p(9))
            .expect("same settlement replays");

        let totals = IntentStoreOps::totals(&request.identity.resource_key);
        assert_eq!(totals.reserved_qty, 0);
        assert_eq!(totals.committed_qty, 1);
        assert_eq!(totals.pending_count, 0);
        assert!(
            matches!(
                ReceiptBackedIntentOps::load(request.identity.operation_id)
                    .expect("load intent")
                    .expect("intent exists")
                    .state,
                ReceiptBackedIntentState::Committed { .. }
            ),
            "allocation intent must retain committed evidence"
        );
    }

    #[test]
    fn terminal_cleanup_removes_evidence_without_reversing_committed_capacity() {
        reset_intents();
        let request = request(0, 1);
        let permit = begin_allocation(&request).expect("allocation reserves");
        PlacementAllocationWorkflow::commit_registered_child(&permit, p(9))
            .expect("allocation commits");
        let totals_before = IntentStoreOps::totals(&request.identity.resource_key);

        remove_terminal_intent(
            request.identity.operation_id,
            request.identity.payload_binding,
        )
        .expect("terminal evidence cleanup succeeds");

        assert!(
            ReceiptBackedIntentOps::load(request.identity.operation_id)
                .expect("load cleaned intent")
                .is_none()
        );
        assert_eq!(
            IntentStoreOps::totals(&request.identity.resource_key),
            totals_before
        );
    }

    #[test]
    fn cumulative_history_does_not_consume_replacement_capacity() {
        reset_intents();
        let first = request(0, 1);
        let first_permit = begin_allocation(&first).expect("first allocation reserves");
        PlacementAllocationWorkflow::commit_registered_child(&first_permit, p(8))
            .expect("first allocation commits");

        assert_eq!(
            PlacementAllocationWorkflow::next_sequence(&first.identity),
            1
        );
        let mut replacement = request(1, 0);
        replacement.reservation_limit =
            PlacementAllocationWorkflow::reservation_limit_for_available_capacity(
                &replacement.identity,
                1,
            );
        begin_allocation(&replacement).expect("one live replacement slot remains available");

        let totals = IntentStoreOps::totals(&first.identity.resource_key);
        assert_eq!(totals.committed_qty, 1);
        assert_eq!(totals.reserved_qty, 1);
    }

    #[test]
    fn disposed_child_rollback_is_idempotent_and_releases_reserved_capacity() {
        reset_intents();
        let request = request(0, 1);
        let permit = begin_allocation(&request).expect("allocation reserves");

        PlacementAllocationWorkflow::rollback_disposed_child(&permit, p(9))
            .expect("first rollback settles");
        PlacementAllocationWorkflow::rollback_disposed_child(&permit, p(9))
            .expect("same rollback replays");

        let totals = IntentStoreOps::totals(&request.identity.resource_key);
        assert_eq!(totals.reserved_qty, 0);
        assert_eq!(totals.committed_qty, 0);
        assert_eq!(totals.pending_count, 0);
        assert!(matches!(
            ReceiptBackedIntentOps::load(request.identity.operation_id)
                .expect("load intent")
                .expect("intent exists")
                .state,
            ReceiptBackedIntentState::RolledBack { .. }
        ));
    }
}
