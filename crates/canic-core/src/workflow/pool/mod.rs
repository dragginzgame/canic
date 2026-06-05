pub mod admin;
pub mod admissibility;
pub mod controllers;
pub mod query;
pub mod scheduler;

use crate::{
    InternalError, InternalErrorOrigin,
    cdk::types::TC,
    domain::policy::pool::authority::require_pool_admin,
    dto::{
        error::Error,
        pool::{CanisterPoolStatus, CreateEmptyPoolRequest, PoolAdminResponse, PoolBatchResult},
        rpc::RootRequestMetadata,
    },
    ids::{IntentId, IntentResourceKey},
    ops::{
        cost_guard::{CostGuardOps, CostGuardPermit, CostGuardRequest},
        ic::{
            IcOps,
            mgmt::{CanisterSettings, MgmtOps, UpdateSettingsArgs},
        },
        replay::{
            guard::secs_to_ns,
            model::{
                CommandKind, ExternalEffectDescriptor, OperationId, RecoveryReason, ReplayActor,
                ReplayPayloadHasher,
            },
            receipt::{
                ReplayReceiptDecision, ReplayReceiptReserveInput, ReplayReceiptStoreError,
                ReplayReceiptToken, abort_reserved_receipt, commit_receipt_response,
                mark_external_effect_in_flight, mark_recovery_required, reserve_or_replay_receipt,
            },
        },
        runtime::env::EnvOps,
        runtime::metrics::{
            intent::{
                IntentMetricOperation, IntentMetricOutcome, IntentMetricReason,
                IntentMetricSurface, IntentMetrics,
            },
            pool::{
                PoolMetricOperation as MetricOperation, PoolMetricOutcome as MetricOutcome,
                PoolMetricReason as MetricReason,
            },
            recording::PoolMetricEvent as MetricEvent,
        },
        storage::{intent::IntentStoreOps, pool::PoolOps, registry::subnet::SubnetRegistryOps},
    },
    replay_policy::CostClass,
    workflow::{
        pool::{query::PoolQuery, scheduler::PoolSchedulerWorkflow},
        prelude::*,
        runtime::intent::IntentCleanupWorkflow,
    },
};
use candid::{decode_one, encode_one};

/// Default cycles allocated to freshly created pool canisters.
const POOL_CANISTER_CYCLES: u128 = 5 * TC;
const POOL_CREATE_EMPTY_REPLAY_COMMAND_KIND: &str = "pool.create_empty.v1";
const POOL_CREATE_EMPTY_REPLAY_RESPONSE_SCHEMA_VERSION: u32 = 1;
const POOL_CREATE_EMPTY_MAX_REPLAY_TTL_SECONDS: u64 = 300;
const POOL_CREATE_EMPTY_QUOTA_WINDOW_SECONDS: u64 = 60;
const POOL_CREATE_EMPTY_MAX_OPERATIONS_PER_WINDOW: u64 = 10;
const POOL_CREATE_EMPTY_MIN_CYCLES_AFTER_RESERVATION: u128 = TC;

///
/// PoolWorkflow
///

pub struct PoolWorkflow;

impl PoolWorkflow {
    // -------------------------------------------------------------------------
    // Reset
    // -------------------------------------------------------------------------

    pub async fn reset_into_pool(pid: Principal) -> Result<Cycles, InternalError> {
        MetricEvent::started(MetricOperation::Reset);
        let controllers = match Self::pool_controllers() {
            Ok(controllers) => controllers,
            Err(err) => {
                MetricEvent::failed(MetricOperation::Reset, &err);
                return Err(err);
            }
        };

        if let Err(err) = MgmtOps::update_settings(&UpdateSettingsArgs {
            canister_id: pid,
            settings: CanisterSettings {
                controllers: Some(controllers),
                ..Default::default()
            },
            sender_canister_version: None,
        })
        .await
        {
            MetricEvent::failed(MetricOperation::Reset, &err);
            return Err(err);
        }

        if let Err(err) = MgmtOps::uninstall_code(pid).await {
            MetricEvent::failed(MetricOperation::Reset, &err);
            return Err(err);
        }

        match MgmtOps::get_cycles(pid).await {
            Ok(cycles) => {
                MetricEvent::completed(MetricOperation::Reset, MetricReason::Ok);
                Ok(cycles)
            }
            Err(err) => {
                MetricEvent::failed(MetricOperation::Reset, &err);
                Err(err)
            }
        }
    }

    // -------------------------------------------------------------------------
    // Metadata helpers
    // -------------------------------------------------------------------------

    fn mark_pending_reset(pid: Principal) {
        let created_at = IcOps::now_secs();
        PoolOps::mark_pending_reset(pid, created_at);
    }

    fn mark_ready(pid: Principal, cycles: Cycles) {
        let created_at = IcOps::now_secs();
        PoolOps::mark_ready(pid, cycles, created_at);
    }

    fn mark_failed(pid: Principal, err: &InternalError) {
        let created_at = IcOps::now_secs();
        PoolOps::mark_failed(pid, err, created_at);
    }

    // -------------------------------------------------------------------------
    // Selection
    // -------------------------------------------------------------------------

    #[must_use]
    pub fn pop_oldest_ready() -> Option<Principal> {
        let pid = PoolOps::pop_oldest_ready_pid();
        if pid.is_some() {
            MetricEvent::completed(MetricOperation::SelectReady, MetricReason::Ok);
        } else {
            MetricEvent::skipped(MetricOperation::SelectReady, MetricReason::Empty);
        }
        pid
    }

    #[must_use]
    pub fn pop_oldest_pending_reset() -> Option<Principal> {
        PoolOps::pop_oldest_pending_reset_pid()
    }

    // -------------------------------------------------------------------------
    // Auth
    // -------------------------------------------------------------------------

    fn require_pool_admin() -> Result<(), InternalError> {
        require_pool_admin(EnvOps::is_root()).map_err(Into::into)
    }

    // -------------------------------------------------------------------------
    // Creation
    // -------------------------------------------------------------------------

    pub async fn pool_create_canister(
        request: CreateEmptyPoolRequest,
    ) -> Result<Principal, InternalError> {
        MetricEvent::started(MetricOperation::CreateEmpty);
        if let Err(err) = Self::require_pool_admin() {
            MetricEvent::failed(MetricOperation::CreateEmpty, &err);
            return Err(err);
        }

        let metadata = match pool_create_empty_replay_metadata(request.metadata) {
            Ok(metadata) => metadata,
            Err(err) => {
                MetricEvent::failed(MetricOperation::CreateEmpty, &err);
                return Err(err);
            }
        };
        let caller = IcOps::msg_caller();
        let (command_kind, token) = match reserve_pool_create_empty_replay(metadata, caller) {
            Ok(PoolCreateEmptyReplayReservation::Fresh {
                command_kind,
                token,
            }) => (command_kind, token),
            Ok(PoolCreateEmptyReplayReservation::Replay(pid)) => {
                MetricEvent::completed(MetricOperation::CreateEmpty, MetricReason::Ok);
                return Ok(pid);
            }
            Err(err) => {
                MetricEvent::failed(MetricOperation::CreateEmpty, &err);
                return Err(err);
            }
        };

        let cycles = Cycles::new(POOL_CANISTER_CYCLES);
        let controllers = match Self::pool_controllers() {
            Ok(controllers) => controllers,
            Err(err) => {
                abort_reserved_receipt(&token);
                MetricEvent::failed(MetricOperation::CreateEmpty, &err);
                return Err(err);
            }
        };
        let cost_permit = match reserve_pool_create_empty_cost_guard(&command_kind, caller) {
            Ok(permit) => permit,
            Err(err) => {
                abort_reserved_receipt(&token);
                MetricEvent::failed(MetricOperation::CreateEmpty, &err);
                return Err(err);
            }
        };

        mark_pool_create_empty_external_effect(&token, &command_kind);

        let pid =
            match MgmtOps::create_canister_with_permit(&cost_permit, controllers, cycles.clone())
                .await
            {
                Ok(pid) => pid,
                Err(err) => {
                    let _ = CostGuardOps::recover(&cost_permit, IcOps::now_secs());
                    mark_recovery_required(
                        &token,
                        RecoveryReason::ExternalEffectStatusUnknown,
                        secs_to_ns(IcOps::now_secs()),
                    );
                    MetricEvent::failed(MetricOperation::CreateEmpty, &err);
                    return Err(err);
                }
            };

        let response = PoolAdminResponse::Created { pid };
        match encode_pool_create_empty_response(&response) {
            Ok(response_bytes) => {
                commit_pool_create_empty_success(&token, &cost_permit, pid, cycles, response_bytes);
            }
            Err(err) => {
                let _ = CostGuardOps::recover(&cost_permit, IcOps::now_secs());
                mark_recovery_required(
                    &token,
                    RecoveryReason::ResponseCommitFailed,
                    secs_to_ns(IcOps::now_secs()),
                );
                MetricEvent::failed(MetricOperation::CreateEmpty, &err);
                return Err(err);
            }
        }

        MetricEvent::completed(MetricOperation::CreateEmpty, MetricReason::Ok);

        Ok(pid)
    }

    // -------------------------------------------------------------------------
    // Import
    // -------------------------------------------------------------------------

    pub async fn pool_import_canister(pid: Principal) -> Result<(), InternalError> {
        MetricEvent::started(MetricOperation::ImportImmediate);
        if let Err(err) = Self::require_pool_admin() {
            MetricEvent::failed(MetricOperation::ImportImmediate, &err);
            return Err(err);
        }
        if let Err(err) = admissibility::check_can_enter_pool(pid).await {
            MetricEvent::record(
                MetricOperation::ImportImmediate,
                MetricOutcome::Failed,
                MetricReason::from_policy(&err),
            );
            return Err(err.into());
        }

        let intent_key = match pool_import_intent_key(pid) {
            Ok(intent_key) => intent_key,
            Err(err) => {
                MetricEvent::failed(MetricOperation::ImportImmediate, &err);
                return Err(err);
            }
        };

        let intent_id = match reserve_pool_import_intent(intent_key) {
            Ok(intent_id) => intent_id,
            Err(err) => {
                MetricEvent::failed(MetricOperation::ImportImmediate, &err);
                return Err(err);
            }
        };

        // Invariant: mark_pending_reset must remain synchronous and non-trapping.
        Self::mark_pending_reset(pid);

        match Self::reset_into_pool(pid).await {
            Ok(cycles) => {
                let _ = SubnetRegistryOps::remove(&pid);
                Self::mark_ready(pid, cycles);

                if let Err(err) = commit_pool_import_intent(intent_id, pid) {
                    MetricEvent::failed(MetricOperation::ImportImmediate, &err);
                    return Err(err);
                }

                MetricEvent::completed(MetricOperation::ImportImmediate, MetricReason::Ok);
                Ok(())
            }
            Err(err) => {
                let (class, origin) = err.log_fields();
                log!(
                    Topic::CanisterPool,
                    Warn,
                    "pool import failed for {pid} class={class} origin={origin}: {err}"
                );
                Self::mark_failed(pid, &err);

                abort_pool_import_intent(intent_id, pid);

                MetricEvent::failed(MetricOperation::ImportImmediate, &err);
                Err(err)
            }
        }
    }

    // -------------------------------------------------------------------------
    // Recycle
    // -------------------------------------------------------------------------

    pub async fn pool_recycle_canister(pid: Principal) -> Result<(), InternalError> {
        MetricEvent::started(MetricOperation::Recycle);
        if let Err(err) = Self::require_pool_admin() {
            MetricEvent::failed(MetricOperation::Recycle, &err);
            return Err(err);
        }

        // Recycling a missing child is an idempotent no-op so stale directory cleanup
        // never depends on the provisional child still existing.
        let Some(entry) = SubnetRegistryOps::get(pid) else {
            MetricEvent::skipped(MetricOperation::Recycle, MetricReason::NotFound);
            return Ok(());
        };

        let role = Some(entry.role.clone());
        let module_hash = entry.module_hash.clone();

        // Destructive reset
        let cycles = match Self::reset_into_pool(pid).await {
            Ok(cycles) => cycles,
            Err(err) => {
                MetricEvent::failed(MetricOperation::Recycle, &err);
                return Err(err);
            }
        };

        // Remove from topology
        let _ = SubnetRegistryOps::remove(&pid);

        // Register back into pool, preserving metadata
        let created_at = IcOps::now_secs();
        PoolOps::register_ready(pid, cycles, role, None, module_hash, created_at);

        MetricEvent::completed(MetricOperation::Recycle, MetricReason::Ok);

        Ok(())
    }

    // -------------------------------------------------------------------------
    // Bulk import
    // -------------------------------------------------------------------------

    pub async fn pool_import_queued_canisters(
        pids: Vec<Principal>,
    ) -> Result<PoolBatchResult, InternalError> {
        MetricEvent::started(MetricOperation::ImportQueued);
        if let Err(err) = Self::require_pool_admin() {
            MetricEvent::failed(MetricOperation::ImportQueued, &err);
            return Err(err);
        }

        let total = pids.len() as u64;

        let mut added = 0;
        let mut requeued = 0;
        let mut skipped = 0;

        for pid in pids {
            match admissibility::check_can_enter_pool(pid).await {
                Ok(()) => {
                    if let Some(entry) = PoolQuery::pool_entry(pid) {
                        if let CanisterPoolStatus::Failed { .. } = entry.status {
                            Self::mark_pending_reset(pid);
                            MetricEvent::record(
                                MetricOperation::ImportQueued,
                                MetricOutcome::Requeued,
                                MetricReason::FailedEntry,
                            );
                            requeued += 1;
                        } else {
                            // already ready or pending reset
                            MetricEvent::skipped(
                                MetricOperation::ImportQueued,
                                MetricReason::AlreadyPresent,
                            );
                            skipped += 1;
                        }
                    } else {
                        Self::mark_pending_reset(pid);
                        MetricEvent::completed(MetricOperation::ImportQueued, MetricReason::Ok);
                        added += 1;
                    }
                }

                // Any policy rejection is treated as a skip
                Err(err) => {
                    MetricEvent::record(
                        MetricOperation::ImportQueued,
                        MetricOutcome::Skipped,
                        MetricReason::from_policy(&err),
                    );
                    skipped += 1;
                }
            }
        }

        let result = PoolBatchResult {
            total,
            added,
            requeued,
            skipped,
        };

        if result.added > 0 || result.requeued > 0 {
            PoolSchedulerWorkflow::schedule();
        }

        MetricEvent::completed(MetricOperation::ImportQueued, MetricReason::Ok);

        Ok(result)
    }
}

fn pool_create_empty_replay_metadata(
    metadata: Option<RootRequestMetadata>,
) -> Result<RootRequestMetadata, InternalError> {
    let metadata = metadata.ok_or_else(|| {
        InternalError::public(Error::invalid(
            "pool create-empty request requires replay metadata",
        ))
    })?;
    if metadata.ttl_seconds == 0 {
        return Err(InternalError::public(Error::invalid(
            "pool create-empty replay metadata ttl_seconds must be greater than zero",
        )));
    }
    if metadata.ttl_seconds > POOL_CREATE_EMPTY_MAX_REPLAY_TTL_SECONDS {
        return Err(InternalError::public(Error::invalid(format!(
            "pool create-empty replay metadata ttl_seconds={} exceeds max {}",
            metadata.ttl_seconds, POOL_CREATE_EMPTY_MAX_REPLAY_TTL_SECONDS
        ))));
    }
    Ok(metadata)
}

///
/// PoolCreateEmptyReplayReservation
///
enum PoolCreateEmptyReplayReservation {
    Fresh {
        command_kind: CommandKind,
        token: Box<ReplayReceiptToken>,
    },
    Replay(Principal),
}

fn reserve_pool_create_empty_replay(
    metadata: RootRequestMetadata,
    caller: Principal,
) -> Result<PoolCreateEmptyReplayReservation, InternalError> {
    let command_kind = pool_create_empty_command_kind();
    let actor = ReplayActor::direct_caller(caller);
    let payload_hash = pool_create_empty_payload_hash(&command_kind, &actor);
    let now_secs = IcOps::now_secs();
    let replay_input = ReplayReceiptReserveInput::new(
        command_kind.clone(),
        OperationId::from_bytes(metadata.request_id),
        actor,
        payload_hash,
        secs_to_ns(now_secs),
    )
    .with_expires_at_ns(secs_to_ns(now_secs.saturating_add(metadata.ttl_seconds)));

    match reserve_or_replay_receipt(replay_input) {
        Ok(ReplayReceiptDecision::Fresh(token)) => Ok(PoolCreateEmptyReplayReservation::Fresh {
            command_kind,
            token: Box::new(token),
        }),
        Ok(decision) => map_pool_create_empty_replay_decision(decision)
            .map(PoolCreateEmptyReplayReservation::Replay),
        Err(err) => Err(map_pool_create_empty_replay_store_error(err)),
    }
}

fn reserve_pool_create_empty_cost_guard(
    command_kind: &CommandKind,
    caller: Principal,
) -> Result<CostGuardPermit, InternalError> {
    CostGuardOps::reserve(CostGuardRequest {
        cost_class: CostClass::ManagementDeployment,
        command_kind: command_kind.clone(),
        quota_subject: caller,
        payer: IcOps::canister_self(),
        now_secs: IcOps::now_secs(),
        quota_window_secs: POOL_CREATE_EMPTY_QUOTA_WINDOW_SECONDS,
        max_operations_per_window: POOL_CREATE_EMPTY_MAX_OPERATIONS_PER_WINDOW,
        current_cycle_balance: MgmtOps::canister_cycle_balance().to_u128(),
        cycle_reservation_cycles: POOL_CANISTER_CYCLES,
        min_cycles_after_reservation: POOL_CREATE_EMPTY_MIN_CYCLES_AFTER_RESERVATION,
    })
}

fn mark_pool_create_empty_external_effect(token: &ReplayReceiptToken, command_kind: &CommandKind) {
    mark_external_effect_in_flight(
        token,
        ExternalEffectDescriptor::ManagementCreateCanister {
            command_kind: command_kind.clone(),
        },
        secs_to_ns(IcOps::now_secs()),
    );
}

fn commit_pool_create_empty_success(
    token: &ReplayReceiptToken,
    cost_permit: &CostGuardPermit,
    pid: Principal,
    cycles: Cycles,
    response_bytes: Vec<u8>,
) {
    let created_at = IcOps::now_secs();
    PoolOps::register_ready(pid, cycles, None, None, None, created_at);
    commit_receipt_response(
        token,
        POOL_CREATE_EMPTY_REPLAY_RESPONSE_SCHEMA_VERSION,
        response_bytes,
        secs_to_ns(IcOps::now_secs()),
    );
    if let Err(err) = CostGuardOps::complete(cost_permit, IcOps::now_secs()) {
        log!(
            Topic::CanisterPool,
            Error,
            "pool create cost guard completion failed pid={pid}: {err}"
        );
    }
}

fn pool_create_empty_command_kind() -> CommandKind {
    CommandKind::new(POOL_CREATE_EMPTY_REPLAY_COMMAND_KIND)
        .expect("pool create-empty replay command kind is a valid static label")
}

fn pool_create_empty_payload_hash(command_kind: &CommandKind, actor: &ReplayActor) -> [u8; 32] {
    let mut hasher = ReplayPayloadHasher::new(command_kind, actor);
    hasher.hash_str("create_empty");
    hasher.finish()
}

fn map_pool_create_empty_replay_decision(
    decision: ReplayReceiptDecision,
) -> Result<Principal, InternalError> {
    match decision {
        ReplayReceiptDecision::Fresh(_) => Err(InternalError::invariant(
            InternalErrorOrigin::Workflow,
            "fresh pool create replay decision escaped",
        )),
        ReplayReceiptDecision::ReturnCommitted(receipt) => {
            decode_pool_create_empty_response(&receipt)
        }
        ReplayReceiptDecision::OperationInProgress => Err(InternalError::public(Error::conflict(
            "pool create-empty request is already in progress; retry later with the same request id",
        ))),
        ReplayReceiptDecision::ActorMismatch => Err(InternalError::public(Error::conflict(
            "pool create-empty request id was reused by a different caller",
        ))),
        ReplayReceiptDecision::PayloadMismatch => Err(InternalError::public(Error::conflict(
            "pool create-empty request id was reused with a different payload",
        ))),
        ReplayReceiptDecision::Expired => Err(InternalError::public(Error::conflict(
            "pool create-empty replay receipt expired; retry with a new request id",
        ))),
        ReplayReceiptDecision::RecoveryRequired(reason) => {
            Err(InternalError::public(Error::conflict(format!(
                "pool create-empty request requires recovery before replay: {reason:?}"
            ))))
        }
        ReplayReceiptDecision::TerminalFailed {
            error_code,
            error_bytes,
            error_bytes_truncated,
        } => Err(InternalError::public(Error::conflict(format!(
            "pool create-empty request previously failed: {error_code:?}; error_bytes_len={}; truncated={error_bytes_truncated}",
            error_bytes.len()
        )))),
    }
}

fn map_pool_create_empty_replay_store_error(err: ReplayReceiptStoreError) -> InternalError {
    match err {
        ReplayReceiptStoreError::ReceiptDecodeFailed(message) => InternalError::workflow(
            InternalErrorOrigin::Workflow,
            format!("failed to decode pool create-empty replay receipt: {message}"),
        ),
    }
}

fn encode_pool_create_empty_response(
    response: &PoolAdminResponse,
) -> Result<Vec<u8>, InternalError> {
    encode_one(response).map_err(|err| {
        InternalError::workflow(
            InternalErrorOrigin::Workflow,
            format!("failed to encode pool create-empty replay response: {err}"),
        )
    })
}

fn decode_pool_create_empty_response(
    receipt: &crate::ops::replay::model::ReplayReceipt,
) -> Result<Principal, InternalError> {
    let response_schema_version = receipt.response_schema_version.ok_or_else(|| {
        InternalError::workflow(
            InternalErrorOrigin::Workflow,
            "pool create-empty replay receipt is missing response schema version",
        )
    })?;
    if response_schema_version != POOL_CREATE_EMPTY_REPLAY_RESPONSE_SCHEMA_VERSION {
        return Err(InternalError::workflow(
            InternalErrorOrigin::Workflow,
            format!(
                "unsupported pool create-empty replay response schema version {response_schema_version}"
            ),
        ));
    }
    let response_bytes = receipt.response_bytes.as_deref().ok_or_else(|| {
        InternalError::workflow(
            InternalErrorOrigin::Workflow,
            "pool create-empty replay receipt is missing response bytes",
        )
    })?;
    let response = decode_one(response_bytes).map_err(|err| {
        InternalError::workflow(
            InternalErrorOrigin::Workflow,
            format!("failed to decode pool create-empty replay response: {err}"),
        )
    })?;
    match response {
        PoolAdminResponse::Created { pid } => Ok(pid),
        _ => Err(InternalError::workflow(
            InternalErrorOrigin::Workflow,
            "pool create-empty replay receipt contains the wrong response variant",
        )),
    }
}

//
// ─────────────────────────────────────────────────────────────
// Intent helpers
// ─────────────────────────────────────────────────────────────
//

// Build the stable intent resource key for an imported pool canister.
fn pool_import_intent_key(pid: Principal) -> Result<IntentResourceKey, InternalError> {
    let bytes = pid.as_slice();
    let mut buf = String::with_capacity(3 + bytes.len() * 2);
    buf.push_str("pi:");
    buf.push_str(&hex_encode(bytes));

    IntentResourceKey::try_new(buf).map_err(|err| {
        InternalError::invariant(
            InternalErrorOrigin::Workflow,
            format!("pool import intent key: {err}"),
        )
    })
}

// Reserve the import intent before resetting an external canister into the pool.
fn reserve_pool_import_intent(intent_key: IntentResourceKey) -> Result<IntentId, InternalError> {
    let intent_id = match IntentStoreOps::allocate_intent_id() {
        Ok(intent_id) => intent_id,
        Err(err) => {
            record_pool_intent(
                IntentMetricOperation::Reserve,
                IntentMetricOutcome::Failed,
                IntentMetricReason::StorageFailed,
            );
            return Err(err);
        }
    };

    let now_secs = IcOps::now_secs();
    IntentCleanupWorkflow::ensure_started();
    if let Err(err) =
        IntentStoreOps::try_reserve(intent_id, intent_key, 1, now_secs, None, now_secs)
    {
        record_pool_intent(
            IntentMetricOperation::Reserve,
            IntentMetricOutcome::Failed,
            IntentMetricReason::StorageFailed,
        );
        return Err(err);
    }

    record_pool_intent(
        IntentMetricOperation::Reserve,
        IntentMetricOutcome::Completed,
        IntentMetricReason::Ok,
    );

    Ok(intent_id)
}

// Commit the import intent after the canister has been reset and registered.
fn commit_pool_import_intent(intent_id: IntentId, pid: Principal) -> Result<(), InternalError> {
    if let Err(err) = IntentStoreOps::commit_at(intent_id, IcOps::now_secs()) {
        record_pool_intent(
            IntentMetricOperation::Commit,
            IntentMetricOutcome::Failed,
            IntentMetricReason::StorageFailed,
        );
        log!(
            Topic::CanisterPool,
            Warn,
            "pool import commit failed for {pid}: {err}"
        );
        return Err(err);
    }

    record_pool_intent(
        IntentMetricOperation::Commit,
        IntentMetricOutcome::Completed,
        IntentMetricReason::Ok,
    );
    Ok(())
}

// Abort the import intent after reset fails; the reset error remains authoritative.
fn abort_pool_import_intent(intent_id: IntentId, pid: Principal) {
    if let Err(abort_err) = IntentStoreOps::abort(intent_id) {
        record_pool_intent(
            IntentMetricOperation::Abort,
            IntentMetricOutcome::Failed,
            IntentMetricReason::StorageFailed,
        );
        log!(
            Topic::CanisterPool,
            Warn,
            "pool import abort failed for {pid}: {abort_err}"
        );
    } else {
        record_pool_intent(
            IntentMetricOperation::Abort,
            IntentMetricOutcome::Completed,
            IntentMetricReason::Ok,
        );
    }
}

// Record a pool-surface intent metric with fixed labels only.
fn record_pool_intent(
    operation: IntentMetricOperation,
    outcome: IntentMetricOutcome,
    reason: IntentMetricReason,
) {
    IntentMetrics::record(IntentMetricSurface::Pool, operation, outcome, reason);
}

// Encode raw principal bytes as lowercase hex for intent resource keys.
fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);

    for &b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        dto::error::ErrorCode,
        ops::replay::model::{
            REPLAY_PAYLOAD_HASH_SCHEMA_VERSION, REPLAY_RECEIPT_SCHEMA_VERSION, ReplayReceipt,
            ReplayReceiptStatus,
        },
    };

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn metadata(id: u8, ttl_seconds: u64) -> RootRequestMetadata {
        RootRequestMetadata {
            request_id: [id; 32],
            ttl_seconds,
        }
    }

    fn committed_pool_create_empty_receipt(pid: Principal) -> ReplayReceipt {
        let command_kind = pool_create_empty_command_kind();
        let actor = ReplayActor::direct_caller(p(2));
        ReplayReceipt {
            schema_version: REPLAY_RECEIPT_SCHEMA_VERSION,
            command_kind: command_kind.clone(),
            operation_id: OperationId::from_bytes([7; 32]),
            actor,
            payload_hash_schema_version: REPLAY_PAYLOAD_HASH_SCHEMA_VERSION,
            payload_hash: pool_create_empty_payload_hash(&command_kind, &actor),
            status: ReplayReceiptStatus::Committed,
            created_at_ns: 100,
            updated_at_ns: 200,
            expires_at_ns: Some(1_000),
            response_schema_version: Some(POOL_CREATE_EMPTY_REPLAY_RESPONSE_SCHEMA_VERSION),
            response_bytes: Some(
                encode_pool_create_empty_response(&PoolAdminResponse::Created { pid })
                    .expect("response encodes"),
            ),
            effect: Some(ExternalEffectDescriptor::ManagementCreateCanister { command_kind }),
        }
    }

    #[test]
    fn pool_create_empty_replay_metadata_rejects_missing_or_invalid_ttl() {
        let missing = pool_create_empty_replay_metadata(None).expect_err("metadata is required");
        assert_eq!(
            missing.public_error().expect("public error").code,
            ErrorCode::InvalidInput
        );

        let zero = pool_create_empty_replay_metadata(Some(metadata(1, 0)))
            .expect_err("zero ttl is invalid");
        assert_eq!(
            zero.public_error().expect("public error").code,
            ErrorCode::InvalidInput
        );

        let too_large = pool_create_empty_replay_metadata(Some(metadata(
            1,
            POOL_CREATE_EMPTY_MAX_REPLAY_TTL_SECONDS + 1,
        )))
        .expect_err("oversized ttl is invalid");
        assert_eq!(
            too_large.public_error().expect("public error").code,
            ErrorCode::InvalidInput
        );
    }

    #[test]
    fn pool_create_empty_replay_metadata_accepts_bounded_ttl() {
        let accepted = pool_create_empty_replay_metadata(Some(metadata(
            3,
            POOL_CREATE_EMPTY_MAX_REPLAY_TTL_SECONDS,
        )))
        .expect("bounded ttl is accepted");

        assert_eq!(accepted.request_id, [3; 32]);
        assert_eq!(
            accepted.ttl_seconds,
            POOL_CREATE_EMPTY_MAX_REPLAY_TTL_SECONDS
        );
    }

    #[test]
    fn pool_create_empty_payload_hash_binds_actor() {
        let command_kind = pool_create_empty_command_kind();
        let actor_a = ReplayActor::direct_caller(p(2));
        let actor_b = ReplayActor::direct_caller(p(3));

        assert_ne!(
            pool_create_empty_payload_hash(&command_kind, &actor_a),
            pool_create_empty_payload_hash(&command_kind, &actor_b)
        );
    }

    #[test]
    fn pool_create_empty_replay_decision_returns_committed_created_response() {
        let pid = p(9);
        let decision =
            ReplayReceiptDecision::ReturnCommitted(committed_pool_create_empty_receipt(pid));

        assert_eq!(
            map_pool_create_empty_replay_decision(decision).expect("committed receipt replays"),
            pid
        );
    }
}
