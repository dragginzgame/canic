//! Module: workflow::placement::allocation
//!
//! Responsibility: compose receipt-backed admission with replayed root child creation.
//! Does not own: placement policy, domain registry mutation, or root replay storage.
//! Boundary: placement workflows register the returned child before settling the permit.

use crate::{
    InternalError,
    cdk::types::Principal,
    dto::rpc::CreateCanisterParent,
    model::{
        intent::{
            BeginPlacementReceiptBackedIntentInput, BeginReceiptBackedIntentResult,
            ReceiptBackedIntent, ReceiptBackedIntentState, RemoveTerminalReceiptBackedIntentInput,
            RemoveTerminalReceiptBackedIntentResult, SettleReceiptBackedIntentInput,
            SettleReceiptBackedIntentResult, TerminalEvidence, TerminalEvidenceDecision,
        },
        placement::allocation::PlacementAllocationIdentity,
        replay::{OperationId, ReplayPayloadHasher},
    },
    ops::{
        rpc::request::RequestOps,
        runtime::env::EnvOps,
        storage::intent::{IntentStoreOps, ReceiptBackedIntentOps},
    },
    workflow::{
        placement::acknowledgement::PlacementAcknowledgementWorkflow,
        runtime::intent::ReceiptBackedIntentWorkflow,
    },
};
use canic_contracts::ids::CanisterRole;

const ALLOCATION_RESULT_COMMAND: &str = "placement.allocate_child.result";

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
    root_receipt_may_exist: bool,
}

impl PlacementAllocationPermit {
    /// Exact Root allocation identity carried by this local receipt.
    #[must_use]
    pub const fn operation_id(&self) -> [u8; 32] {
        self.identity.operation_id.into_bytes()
    }

    /// Reject an old allocation reply after its physical canister has been removed or reused.
    pub fn require_current_child(&self, pid: Principal) -> Result<(), InternalError> {
        if crate::ops::storage::children::CanisterChildrenOps::matches_allocation(
            pid,
            self.operation_id(),
        ) {
            Ok(())
        } else {
            Err(InternalError::public(
                crate::diagnostics::codes::POSITION_UNAVAILABLE,
            ))
        }
    }
}

/// A locally reserved request that has not yet crossed the Root call boundary.
pub(super) struct PreparedPlacementAllocation {
    request: PlacementAllocationRequest,
    permit: PlacementAllocationPermit,
}

///
/// PlacementAllocationWorkflow
///
/// Shared child-allocation orchestration used by placement strategies.
///

pub struct PlacementAllocationWorkflow;

impl PlacementAllocationWorkflow {
    /// Derive the stable root operation for disposing one returned child.
    #[must_use]
    pub fn disposed_child_operation_id(
        permit: &PlacementAllocationPermit,
        child_pid: Principal,
    ) -> OperationId {
        permit.identity.disposed_child_operation_id(child_pid)
    }

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
        Self::create_prepared_child(Self::prepare_child(request)?).await
    }

    /// Complete local admission before a domain owner commits to an async create.
    pub(super) fn prepare_child(
        request: PlacementAllocationRequest,
    ) -> Result<PreparedPlacementAllocation, InternalError> {
        let permit = begin_allocation(&request)?;
        Ok(PreparedPlacementAllocation { request, permit })
    }

    pub(super) async fn create_prepared_child(
        prepared: PreparedPlacementAllocation,
    ) -> Result<(PlacementAllocationPermit, Principal), InternalError> {
        let PreparedPlacementAllocation {
            request,
            mut permit,
        } = prepared;
        let response = RequestOps::allocate_placement_child::<Vec<u8>>(
            &request.canister_role,
            CreateCanisterParent::ThisCanister,
            request.extra_arg,
            permit.identity.operation_id,
        )
        .await?;
        permit.root_receipt_may_exist = true;

        Ok((permit, response.new_canister_pid))
    }

    /// Recover a previously admitted create; never invent a new operation for unknown history.
    pub async fn recover_child(
        request: PlacementAllocationRequest,
    ) -> Result<(PlacementAllocationPermit, Principal), InternalError> {
        if ReceiptBackedIntentOps::load(request.identity.operation_id)?.is_none() {
            return Err(InternalError::public(
                crate::diagnostics::codes::STATE_CONFLICT,
            ));
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

    /// Commit completed creation after registration or observed retirement, then drain its receipt.
    pub fn finish_created_child(
        permit: &PlacementAllocationPermit,
        child_pid: Principal,
    ) -> Result<(), InternalError> {
        Self::commit_registered_child(permit, child_pid)?;
        finish_terminal_allocation(permit)
    }

    /// A retired result may already have been disposed by another resumer before its reply.
    pub fn finish_retired_child(
        permit: &PlacementAllocationPermit,
        child_pid: Principal,
    ) -> Result<(), InternalError> {
        if let Some(intent) = ReceiptBackedIntentOps::load(permit.identity.operation_id)? {
            if intent.payload_binding != permit.identity.payload_binding {
                return Err(InternalError::invariant());
            }
            if matches!(intent.state, ReceiptBackedIntentState::RolledBack { .. }) {
                return finish_terminal_allocation(permit);
            }
        }
        Self::finish_created_child(permit, child_pid)
    }

    /// Roll back a disposed child, then hand retained receipt release to the durable drain.
    pub fn finish_disposed_child(
        permit: &PlacementAllocationPermit,
        child_pid: Principal,
    ) -> Result<(), InternalError> {
        Self::rollback_disposed_child(permit, child_pid)?;
        finish_terminal_allocation(permit)
    }
}

fn settle_allocation(
    permit: &PlacementAllocationPermit,
    child_pid: Principal,
    decision: TerminalEvidenceDecision,
) -> Result<(), InternalError> {
    let evidence = allocation_terminal_evidence(&permit.identity, child_pid, decision)?;
    let result = ReceiptBackedIntentWorkflow::settle_canic_owned_if_pending(
        &SettleReceiptBackedIntentInput {
            operation_id: permit.identity.operation_id,
            expected_revision: permit.revision,
            expected_payload_binding: permit.identity.payload_binding,
            evidence,
        },
    )?;

    match result {
        SettleReceiptBackedIntentResult::Settled { state, .. }
        | SettleReceiptBackedIntentResult::AlreadySettled { state, .. }
            if state_matches_decision(&state, decision) =>
        {
            Ok(())
        }
        // A valid permit may outlive acknowledgement cleanup while another resumer awaits Root.
        // The caller has proved either registered membership or a completed, now-retired creation.
        SettleReceiptBackedIntentResult::NotFound
            if decision == TerminalEvidenceDecision::Committed =>
        {
            Ok(())
        }
        SettleReceiptBackedIntentResult::Settled { .. }
        | SettleReceiptBackedIntentResult::AlreadySettled { .. }
        | SettleReceiptBackedIntentResult::NotFound
        | SettleReceiptBackedIntentResult::RevisionConflict { .. }
        | SettleReceiptBackedIntentResult::BindingConflict => Err(InternalError::invariant()),
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

fn finish_terminal_allocation(permit: &PlacementAllocationPermit) -> Result<(), InternalError> {
    if permit.root_receipt_may_exist {
        PlacementAcknowledgementWorkflow::schedule_if_pending()
    } else {
        remove_terminal_intent(
            permit.identity.operation_id,
            permit.identity.payload_binding,
        )
    }
}

fn remove_terminal_intent(
    operation_id: OperationId,
    expected_payload_binding: crate::model::intent::PayloadBinding,
) -> Result<(), InternalError> {
    let Some(intent) = ReceiptBackedIntentOps::load(operation_id)? else {
        return Ok(());
    };
    if intent.payload_binding != expected_payload_binding {
        return Err(InternalError::invariant());
    }
    remove_exact_terminal_intent(&intent)
}

pub(super) fn remove_exact_terminal_intent(
    intent: &ReceiptBackedIntent,
) -> Result<(), InternalError> {
    let result =
        ReceiptBackedIntentOps::remove_terminal(&RemoveTerminalReceiptBackedIntentInput {
            operation_id: intent.operation_id,
            expected_revision: intent.revision,
            expected_payload_binding: intent.payload_binding,
        })?;
    match result {
        RemoveTerminalReceiptBackedIntentResult::Removed
        | RemoveTerminalReceiptBackedIntentResult::NotFound => Ok(()),
        RemoveTerminalReceiptBackedIntentResult::NotTerminal
        | RemoveTerminalReceiptBackedIntentResult::RevisionConflict { .. }
        | RemoveTerminalReceiptBackedIntentResult::BindingConflict => {
            Err(InternalError::invariant())
        }
    }
}

fn begin_allocation(
    request: &PlacementAllocationRequest,
) -> Result<PlacementAllocationPermit, InternalError> {
    let input = BeginPlacementReceiptBackedIntentInput {
        operation_id: request.identity.operation_id,
        payload_binding: request.identity.payload_binding,
        resource_key: request.identity.resource_key.clone(),
        quantity: 1,
        reservation_limit: request.reservation_limit,
    };
    let begin_result = ReceiptBackedIntentWorkflow::begin_placement_or_load(&input)?;
    let (revision, root_receipt_may_exist) = match begin_result {
        BeginReceiptBackedIntentResult::Created { revision } => (revision, false),
        BeginReceiptBackedIntentResult::ExistingPending { revision } => (revision, true),
        BeginReceiptBackedIntentResult::ExistingCommitted { .. }
        | BeginReceiptBackedIntentResult::BindingConflict
        | BeginReceiptBackedIntentResult::ReplayWindowClosed { .. }
        | BeginReceiptBackedIntentResult::ReplayWindowTooLong { .. } => {
            return Err(InternalError::invariant());
        }
        BeginReceiptBackedIntentResult::ExistingRolledBack { .. } => {
            return Err(InternalError::public(
                crate::diagnostics::codes::STATE_CONFLICT,
            ));
        }
        BeginReceiptBackedIntentResult::CapacityExceeded { .. }
        | BeginReceiptBackedIntentResult::StoreCapacityReached { .. } => {
            return Err(InternalError::resource_exhausted());
        }
    };

    Ok(PlacementAllocationPermit {
        identity: request.identity.clone(),
        revision,
        root_receipt_may_exist,
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
    };

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn reset_intents() {
        IntentStoreOps::reset_for_tests();
        EnvOps::set_root_pid_for_tests(p(99));
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
        assert!(!first_permit.root_receipt_may_exist);
        assert!(replay_permit.root_receipt_may_exist);

        let error = begin_allocation(&request(1, 1)).expect_err("second allocation exceeds cap");
        assert!(error.is_public_resource_exhausted());
        let totals = IntentStoreOps::totals(&first.identity.resource_key);
        assert_eq!(totals.reserved_qty, 1);
        assert_eq!(totals.committed_qty, 0);
        assert_eq!(totals.pending_count, 1);
        assert!(
            !ReceiptBackedIntentOps::has_placement_acknowledgements()
                .expect("pending placement must not be queued for acknowledgement")
        );
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
            error.public_error().code(),
            crate::diagnostics::codes::STATE_CONFLICT.raw_code()
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
        assert!(
            !ReceiptBackedIntentOps::has_placement_acknowledgements()
                .expect("placement acknowledgement index")
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

    #[test]
    fn completed_index_keys_do_not_consume_lifetime_quota_capacity() {
        use crate::ops::storage::intent::INTENT_RESOURCE_TOTAL_RECORD_LIMIT;

        reset_intents();
        for key in 0..=INTENT_RESOURCE_TOTAL_RECORD_LIMIT {
            let mut request = request(0, 1);
            request.identity = PlacementAllocationIdentity::index(
                p(1),
                "pool",
                &key.to_string(),
                1,
                &request.canister_role,
                None,
            );
            let permit = begin_allocation(&request).unwrap();
            PlacementAllocationWorkflow::finish_created_child(&permit, p(9)).unwrap();
            PlacementAllocationWorkflow::finish_created_child(&permit, p(9)).unwrap();
            let capacity = ReceiptBackedIntentOps::receipt_capacity().unwrap();
            assert_eq!(capacity.resource_total_records, 0);
            assert_eq!(capacity.total_records, 0);
        }
    }

    #[test]
    fn index_cleanup_preserves_another_pending_reservation_for_the_key() {
        reset_intents();
        let mut first = request(0, 1);
        first.identity =
            PlacementAllocationIdentity::index(p(1), "pool", "key", 1, &first.canister_role, None);
        let first_permit = begin_allocation(&first).unwrap();
        let mut second = first.clone();
        second.identity =
            PlacementAllocationIdentity::index(p(1), "pool", "key", 2, &second.canister_role, None);
        second.reservation_limit = 2;
        let second_permit = begin_allocation(&second).unwrap();
        PlacementAllocationWorkflow::finish_created_child(&first_permit, p(8)).unwrap();
        let retained = IntentStoreOps::totals(&first.identity.resource_key);
        assert_eq!(retained.pending_count, 1);
        assert_eq!(retained.reserved_qty, 1);
        assert_eq!(retained.committed_qty, 1);

        PlacementAllocationWorkflow::finish_created_child(&second_permit, p(9)).unwrap();
        assert_eq!(
            ReceiptBackedIntentOps::receipt_capacity()
                .unwrap()
                .resource_total_records,
            0
        );
    }
}
