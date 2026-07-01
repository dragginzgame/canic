//! Module: workflow::pool::create_empty
//!
//! Responsibility: create empty pool canisters behind replay and cost guards.
//! Does not own: endpoint authorization, stable pool schemas, or management-call ops.
//! Boundary: pool workflow validates admin access, reserves replay/cost, then calls ops.

use crate::{
    InternalError, InternalErrorOrigin,
    cdk::types::TC,
    dto::{
        error::Error,
        pool::{CreateEmptyPoolRequest, PoolAdminResponse},
        rpc::RootRequestMetadata,
    },
    ops::{
        cost_guard::{CostGuardOps, CostGuardPermit, CostGuardRequest},
        ic::{IcOps, mgmt::MgmtOps},
        replay::{
            self as replay_ops, POOL_CREATE_EMPTY_REPLAY_RESPONSE_SCHEMA_VERSION,
            guard::secs_to_ns,
            model::{
                CommandKind, ExternalEffectDescriptor, OperationId, RecoveryReason, ReplayActor,
                ReplayPayloadHasher, ReplayReceipt,
            },
            receipt::{
                ReplayReceiptDecision, ReplayReceiptReserveInput, ReplayReceiptStoreError,
                ReplayReceiptToken, abort_reserved_receipt, commit_receipt_response,
                mark_external_effect_in_flight, mark_recovery_required, reserve_or_replay_receipt,
            },
        },
        runtime::metrics::{
            pool::{PoolMetricOperation as MetricOperation, PoolMetricReason as MetricReason},
            recording::PoolMetricEvent as MetricEvent,
        },
        storage::pool::PoolOps,
    },
    replay_policy::CostClass,
    workflow::{cost_guard::map_cost_guard_reserve_error, pool::PoolWorkflow, prelude::*},
};

/// Default cycles allocated to freshly created pool canisters.
const POOL_CANISTER_CYCLES: u128 = 5 * TC;
const POOL_CREATE_EMPTY_REPLAY_COMMAND_KIND: &str = "pool.create_empty.v1";
const POOL_CREATE_EMPTY_MAX_REPLAY_TTL_NS: u64 = 300_000_000_000;
const POOL_CREATE_EMPTY_QUOTA_WINDOW_SECONDS: u64 = 60;
const POOL_CREATE_EMPTY_MAX_OPERATIONS_PER_WINDOW: u64 = 10;
const POOL_CREATE_EMPTY_MIN_CYCLES_AFTER_RESERVATION: u128 = TC;

impl PoolWorkflow {
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
}

fn pool_create_empty_replay_metadata(
    metadata: Option<RootRequestMetadata>,
) -> Result<RootRequestMetadata, InternalError> {
    let metadata = metadata.ok_or_else(|| InternalError::public(Error::operation_id_required()))?;
    if metadata.ttl_ns == 0 {
        return Err(InternalError::public(Error::invalid(
            "pool create-empty replay metadata ttl_ns must be greater than zero",
        )));
    }
    if metadata.ttl_ns > POOL_CREATE_EMPTY_MAX_REPLAY_TTL_NS {
        return Err(InternalError::public(Error::invalid(format!(
            "pool create-empty replay metadata ttl_ns={} exceeds max {}",
            metadata.ttl_ns, POOL_CREATE_EMPTY_MAX_REPLAY_TTL_NS
        ))));
    }
    Ok(metadata)
}

///
/// PoolCreateEmptyReplayReservation
///
/// Replay reservation outcome for one empty-pool creation request.
/// Owned by pool workflow and mapped into either execution or cached response.
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
    let replay_input = pool_create_empty_replay_input(
        command_kind.clone(),
        metadata.request_id,
        actor,
        payload_hash,
        secs_to_ns(now_secs),
        metadata.ttl_ns,
    )?;

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

fn pool_create_empty_replay_input(
    command_kind: CommandKind,
    request_id: [u8; 32],
    actor: ReplayActor,
    payload_hash: [u8; 32],
    now_ns: u64,
    ttl_ns: u64,
) -> Result<ReplayReceiptReserveInput, InternalError> {
    let expires_at_ns = now_ns.checked_add(ttl_ns).ok_or_else(|| {
        InternalError::public(Error::invalid(
            "pool create-empty replay metadata ttl_ns overflows nanoseconds",
        ))
    })?;
    Ok(ReplayReceiptReserveInput::new(
        command_kind,
        OperationId::from_bytes(request_id),
        actor,
        payload_hash,
        now_ns,
    )
    .with_expires_at_ns(expires_at_ns))
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
        current_cycle_balance: IcOps::canister_cycle_balance().to_u128(),
        cycle_reservation_cycles: POOL_CANISTER_CYCLES,
        min_cycles_after_reservation: POOL_CREATE_EMPTY_MIN_CYCLES_AFTER_RESERVATION,
    })
    .map_err(map_cost_guard_reserve_error)
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
        ReplayReceiptDecision::PendingActorQuotaExceeded { max_pending, .. } => {
            Err(InternalError::public(Error::exhausted(format!(
                "pool create-empty pending replay receipt quota exceeded for caller; max_pending={max_pending}"
            ))))
        }
        ReplayReceiptDecision::PendingCommandQuotaExceeded { max_pending, .. } => {
            Err(InternalError::public(Error::exhausted(format!(
                "pool create-empty pending replay receipt quota exceeded for command kind; max_pending={max_pending}"
            ))))
        }
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
    replay_ops::encode_pool_create_empty_replay_response(response).map_err(|err| match err {
        replay_ops::ReplayCommitError::EncodeFailed(message) => InternalError::workflow(
            InternalErrorOrigin::Workflow,
            format!("failed to encode pool create-empty replay response: {message}"),
        ),
    })
}

fn decode_pool_create_empty_response(receipt: &ReplayReceipt) -> Result<Principal, InternalError> {
    replay_ops::decode_pool_create_empty_replay_response(receipt).map_err(|err| match err {
        replay_ops::ReplayDecodeError::DecodeFailed(message) => {
            InternalError::workflow(InternalErrorOrigin::Workflow, message)
        }
    })
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

    fn metadata(id: u8, ttl_ns: u64) -> RootRequestMetadata {
        RootRequestMetadata {
            request_id: [id; 32],
            ttl_ns,
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
            ErrorCode::OperationIdRequired
        );
        assert_eq!(
            missing.public_error().expect("public error").message,
            "operation_id is required for this command"
        );

        let zero = pool_create_empty_replay_metadata(Some(metadata(1, 0)))
            .expect_err("zero ttl is invalid");
        assert_eq!(
            zero.public_error().expect("public error").code,
            ErrorCode::InvalidInput
        );

        let too_large = pool_create_empty_replay_metadata(Some(metadata(
            1,
            POOL_CREATE_EMPTY_MAX_REPLAY_TTL_NS + 1,
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
            POOL_CREATE_EMPTY_MAX_REPLAY_TTL_NS,
        )))
        .expect("bounded ttl is accepted");

        assert_eq!(accepted.request_id, [3; 32]);
        assert_eq!(accepted.ttl_ns, POOL_CREATE_EMPTY_MAX_REPLAY_TTL_NS);
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
