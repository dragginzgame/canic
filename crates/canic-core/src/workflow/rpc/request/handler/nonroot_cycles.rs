//! Module: workflow::rpc::request::handler::nonroot_cycles
//!
//! Responsibility: authorize and execute replay-protected non-root cycles requests.
//! Does not own: endpoint auth, stable replay receipts, or management-call primitives.
//! Boundary: RPC request handler calls this for root and non-root cycles funding paths.

use super::{
    MAX_ROOT_REPLAY_ENTRIES, MAX_ROOT_REPLAY_ENTRIES_PER_CALLER, MAX_ROOT_TTL_NS,
    REPLAY_PURGE_SCAN_LIMIT, RootContext,
};
use crate::{
    InternalError,
    cdk::types::Principal,
    domain::policy::cycles_funding::{self, FundingPolicyViolation},
    dto::rpc::{CyclesRequest, CyclesResponse},
    ids::CanisterRole,
    log,
    log::Topic,
    ops::{
        cost_guard::{CostGuardOps, CostGuardPermit, CostGuardRequest},
        ic::{IcOps, mgmt::MgmtOps},
        replay::{
            self as replay_ops, ReplayDecodeError, ReplayReserveError,
            guard::{ReplayDecision, ReplayGuardError, ReplayPending, RootReplayGuardInput},
            model::{CommandKind, ExternalEffectDescriptor, OperationId, RecoveryReason},
        },
        runtime::{
            cycles_funding::CyclesFundingLedgerOps,
            env::EnvOps,
            metrics::{
                cycles_funding::{CyclesFundingDeniedReason, CyclesFundingMetrics},
                replay::{
                    ReplayMetricOperation, ReplayMetricOutcome, ReplayMetricReason, ReplayMetrics,
                },
                root_capability::{
                    RootCapabilityMetricKey, RootCapabilityMetricOutcome, RootCapabilityMetrics,
                },
            },
        },
        storage::{children::CanisterChildrenOps, registry::subnet::SubnetRegistryOps},
    },
    replay_policy::CostClass,
    workflow::{cost_guard::map_cost_guard_reserve_error, rpc::RpcWorkflowError},
};

const ROOT_REQUEST_CYCLES_COMMAND_KIND: &str = "root.request_cycles.v1";
const ROOT_REQUEST_CYCLES_VALUE_TRANSFER_QUOTA_WINDOW_SECONDS: u64 = 60;
const MAX_ROOT_REQUEST_CYCLES_VALUE_TRANSFER_OPERATIONS_PER_WINDOW: u64 = 60;
const MIN_ROOT_REQUEST_CYCLES_AFTER_RESERVATION: u128 = 1_000_000_000;

///
/// NonrootCyclesCapabilityWorkflow
///
/// Workflow entrypoint for replay-first non-root cycles requests.
/// Owned by RPC workflow and called after endpoint/root metadata handling.
///

pub struct NonrootCyclesCapabilityWorkflow;

///
/// AuthorizedCyclesGrant
///
/// Approved cycle transfer amount after authorization and policy checks.
/// Owned by RPC workflow and passed into execution helpers.
///

#[derive(Clone, Copy, Debug)]
pub(super) struct AuthorizedCyclesGrant {
    approved_cycles: u128,
}

///
/// ResolvedCyclesChild
///
/// Child role and parent relationship used during cycles authorization.
/// Owned by RPC workflow and resolved from child registries.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct ResolvedCyclesChild {
    role: CanisterRole,
    parent_pid: Option<Principal>,
}

impl NonrootCyclesCapabilityWorkflow {
    /// Execute the non-root cycles capability path with replay-first semantics.
    pub async fn response_replay_first(
        req: CyclesRequest,
    ) -> Result<CyclesResponse, InternalError> {
        response_replay_first_with_planner(
            extract_cycles_context(false)?,
            req,
            authorize_request_cycles_plan,
        )
        .await
    }
}

pub(super) async fn response_replay_first_root(
    req: CyclesRequest,
) -> Result<CyclesResponse, InternalError> {
    response_replay_first_with_planner(
        extract_cycles_context(true)?,
        req,
        authorize_root_request_cycles_plan,
    )
    .await
}

enum ReplayPreflight {
    Fresh(ReplayPending),
    Cached(CyclesResponse),
}

async fn response_replay_first_with_planner(
    ctx: RootContext,
    req: CyclesRequest,
    authorize_plan: fn(
        &RootContext,
        &CyclesRequest,
    ) -> Result<AuthorizedCyclesGrant, InternalError>,
) -> Result<CyclesResponse, InternalError> {
    let preflight = check_cycles_replay(&ctx, &req)?;
    let pending = match preflight {
        ReplayPreflight::Fresh(pending) => pending,
        ReplayPreflight::Cached(response) => return Ok(response),
    };

    let grant = match authorize_plan(&ctx, &req) {
        Ok(grant) => grant,
        Err(err) => {
            abort_replay(pending);
            return Err(err);
        }
    };

    let response = match execute_authorized_request_cycles(&ctx, &pending, grant).await {
        Ok(response) => response,
        Err(err) => {
            abort_replay(pending);
            RootCapabilityMetrics::record_execution(
                RootCapabilityMetricKey::RequestCycles,
                RootCapabilityMetricOutcome::Error,
            );
            return Err(err);
        }
    };

    commit_cycles_replay(pending, &response);

    RootCapabilityMetrics::record_execution(
        RootCapabilityMetricKey::RequestCycles,
        RootCapabilityMetricOutcome::Success,
    );

    Ok(response)
}

fn extract_cycles_context(is_root_env: bool) -> Result<RootContext, InternalError> {
    Ok(RootContext {
        caller: IcOps::msg_caller(),
        self_pid: IcOps::canister_self(),
        is_root_env,
        subnet_id: EnvOps::subnet_pid()?,
        now: IcOps::now_secs(),
    })
}

/// Run cycles authorization while preserving the existing root-capability metrics.
pub(super) fn authorize_request_cycles(
    ctx: &RootContext,
    req: &CyclesRequest,
) -> Result<(), InternalError> {
    authorize_request_cycles_plan(ctx, req).map(|_| ())
}

/// Run root cycles authorization against the authoritative subnet registry.
pub(super) fn authorize_root_request_cycles(
    ctx: &RootContext,
    req: &CyclesRequest,
) -> Result<(), InternalError> {
    authorize_root_request_cycles_plan(ctx, req).map(|_| ())
}

/// Resolve an approved non-root cycles grant in one authorization pass.
pub(super) fn authorize_request_cycles_plan(
    ctx: &RootContext,
    req: &CyclesRequest,
) -> Result<AuthorizedCyclesGrant, InternalError> {
    authorize_request_cycles_with_resolver(ctx, req, direct_child_record)
}

/// Resolve an approved root cycles grant in one authorization pass.
pub(super) fn authorize_root_request_cycles_plan(
    ctx: &RootContext,
    req: &CyclesRequest,
) -> Result<AuthorizedCyclesGrant, InternalError> {
    authorize_request_cycles_with_resolver(ctx, req, registry_child_record)
}

fn authorize_request_cycles_with_resolver(
    ctx: &RootContext,
    req: &CyclesRequest,
    resolve_child: fn(Principal) -> Option<ResolvedCyclesChild>,
) -> Result<AuthorizedCyclesGrant, InternalError> {
    let decision = authorize_request_cycles_inner(ctx, req, resolve_child);

    match &decision {
        Ok(_) => {
            RootCapabilityMetrics::record_authorization(
                RootCapabilityMetricKey::RequestCycles,
                RootCapabilityMetricOutcome::Accepted,
            );
            log!(
                Topic::Rpc,
                Info,
                "capability authorized (capability=RequestCycles, caller={}, subnet={}, now={})",
                ctx.caller,
                ctx.subnet_id,
                ctx.now
            );
        }
        Err(err) => {
            RootCapabilityMetrics::record_authorization(
                RootCapabilityMetricKey::RequestCycles,
                RootCapabilityMetricOutcome::Denied,
            );
            log!(
                Topic::Rpc,
                Warn,
                "capability denied (capability=RequestCycles, caller={}, subnet={}, now={}): {err}",
                ctx.caller,
                ctx.subnet_id,
                ctx.now
            );
        }
    }

    decision
}

/// Apply the existing cycles funding policy and structural child checks.
pub(super) fn authorize_request_cycles_inner(
    ctx: &RootContext,
    req: &CyclesRequest,
    resolve_child: fn(Principal) -> Option<ResolvedCyclesChild>,
) -> Result<AuthorizedCyclesGrant, InternalError> {
    CyclesFundingMetrics::record_requested(ctx.caller, req.cycles);

    let Some(child) = resolve_child(ctx.caller) else {
        CyclesFundingMetrics::record_denied(
            ctx.caller,
            req.cycles,
            CyclesFundingDeniedReason::ChildNotFound,
        );
        return Err(RpcWorkflowError::ChildNotFound(ctx.caller).into());
    };
    if child.parent_pid != Some(ctx.self_pid) {
        CyclesFundingMetrics::record_denied(
            ctx.caller,
            req.cycles,
            CyclesFundingDeniedReason::NotDirectChild,
        );
        return Err(RpcWorkflowError::NotChildOfCaller(ctx.caller, ctx.self_pid).into());
    }

    if !crate::ops::storage::state::app::AppStateOps::cycles_funding_enabled() {
        CyclesFundingMetrics::record_denied(
            ctx.caller,
            req.cycles,
            CyclesFundingDeniedReason::KillSwitchDisabled,
        );
        return Err(RpcWorkflowError::CyclesFundingDisabled.into());
    }

    let policy = cycles_funding::policy_for_child_role(&child.role);
    let ledger = CyclesFundingLedgerOps::snapshot(ctx.caller);
    let decision = match policy.evaluate(ledger, req.cycles, ctx.now) {
        Ok(decision) => decision,
        Err(violation) => return Err(map_funding_policy_violation(ctx, req.cycles, violation)),
    };

    if decision.clamped_max_per_request || decision.clamped_max_per_child {
        log!(
            Topic::Rpc,
            Info,
            "cycles request clamped (caller={}, requested={}, approved={}, max_per_request_clamped={}, child_budget_clamped={})",
            ctx.caller,
            req.cycles,
            decision.approved_cycles,
            decision.clamped_max_per_request,
            decision.clamped_max_per_child
        );
    }

    let available = MgmtOps::canister_cycle_balance().to_u128();
    if decision.approved_cycles > available {
        CyclesFundingMetrics::record_denied(
            ctx.caller,
            decision.approved_cycles,
            CyclesFundingDeniedReason::InsufficientCycles,
        );
        return Err(RpcWorkflowError::InsufficientFundingCycles {
            requested: decision.approved_cycles,
            available,
        }
        .into());
    }

    Ok(AuthorizedCyclesGrant {
        approved_cycles: decision.approved_cycles,
    })
}

/// Execute the approved cycles transfer and return the canonical cycles response.
pub(super) async fn execute_request_cycles(
    ctx: &RootContext,
    pending: &ReplayPending,
    req: &CyclesRequest,
) -> Result<CyclesResponse, InternalError> {
    let grant = authorize_request_cycles_plan(ctx, req)?;
    execute_authorized_request_cycles(ctx, pending, grant).await
}

/// Execute root cycles funding against the authoritative subnet registry.
pub(super) async fn execute_root_request_cycles(
    ctx: &RootContext,
    pending: &ReplayPending,
    req: &CyclesRequest,
) -> Result<CyclesResponse, InternalError> {
    let grant = authorize_root_request_cycles_plan(ctx, req)?;
    execute_authorized_request_cycles(ctx, pending, grant).await
}

/// Execute an already-authorized cycles transfer.
pub(super) async fn execute_authorized_request_cycles(
    ctx: &RootContext,
    pending: &ReplayPending,
    grant: AuthorizedCyclesGrant,
) -> Result<CyclesResponse, InternalError> {
    let cost_permit = reserve_request_cycles_cost_guard(ctx, grant.approved_cycles)?;
    mark_request_cycles_external_effect(pending, ctx, grant.approved_cycles);

    if let Err(err) =
        MgmtOps::deposit_cycles_with_permit(&cost_permit, ctx.caller, grant.approved_cycles).await
    {
        let _ = CostGuardOps::recover(&cost_permit, IcOps::now_secs());
        mark_request_cycles_recovery_required(pending, ctx, grant.approved_cycles, &err);
        CyclesFundingMetrics::record_denied(
            ctx.caller,
            grant.approved_cycles,
            CyclesFundingDeniedReason::ExecutionError,
        );
        return Err(err);
    }

    CyclesFundingMetrics::record_granted(ctx.caller, grant.approved_cycles);
    CyclesFundingLedgerOps::record_child_grant(ctx.caller, grant.approved_cycles, ctx.now);

    if let Err(err) = CostGuardOps::complete(&cost_permit, IcOps::now_secs()) {
        replay_ops::mark_root_replay_recovery_required(
            pending,
            RecoveryReason::ResponseCommitFailed,
            replay_ops::guard::secs_to_ns(IcOps::now_secs()),
        );
        return Err(err);
    }

    Ok(CyclesResponse {
        cycles_transferred: grant.approved_cycles,
    })
}

fn direct_child_record(pid: Principal) -> Option<ResolvedCyclesChild> {
    CanisterChildrenOps::role_parent(pid)
        .map(|(role, parent_pid)| ResolvedCyclesChild { role, parent_pid })
}

fn registry_child_record(pid: Principal) -> Option<ResolvedCyclesChild> {
    SubnetRegistryOps::role_parent(pid)
        .map(|(role, parent_pid)| ResolvedCyclesChild { role, parent_pid })
}

fn map_funding_policy_violation(
    ctx: &RootContext,
    requested_cycles: u128,
    violation: FundingPolicyViolation,
) -> InternalError {
    match violation {
        FundingPolicyViolation::MaxPerChild {
            requested,
            max_per_child,
            remaining_budget,
        } => {
            CyclesFundingMetrics::record_denied(
                ctx.caller,
                requested_cycles,
                CyclesFundingDeniedReason::MaxPerChildExceeded,
            );
            RpcWorkflowError::FundingRequestExceedsChildBudget {
                requested,
                remaining_budget,
                max_per_child,
            }
            .into()
        }
        FundingPolicyViolation::CooldownActive { retry_after_secs } => {
            CyclesFundingMetrics::record_denied(
                ctx.caller,
                requested_cycles,
                CyclesFundingDeniedReason::CooldownActive,
            );
            RpcWorkflowError::FundingCooldownActive { retry_after_secs }.into()
        }
    }
}

fn check_cycles_replay(
    ctx: &RootContext,
    req: &CyclesRequest,
) -> Result<ReplayPreflight, InternalError> {
    let metadata = req.metadata.ok_or_else(|| {
        ReplayMetrics::record(
            ReplayMetricOperation::Check,
            ReplayMetricOutcome::Failed,
            ReplayMetricReason::MissingMetadata,
        );
        RpcWorkflowError::MissingReplayMetadata("RequestCycles")
    })?;
    let payload_hash = hash_cycles_payload(req);

    let decision = replay_ops::guard::evaluate_root_replay(RootReplayGuardInput {
        caller: ctx.caller,
        command_kind: root_request_cycles_command_kind()
            .expect("root request cycles command kind is valid"),
        operation_id: OperationId::from_bytes(metadata.request_id),
        ttl_ns: metadata.ttl_ns,
        payload_hash,
        now_ns: replay_ops::guard::secs_to_ns(ctx.now),
        max_ttl_ns: MAX_ROOT_TTL_NS,
        purge_scan_limit: REPLAY_PURGE_SCAN_LIMIT,
    })
    .map_err(map_replay_guard_error)?;

    match decision {
        ReplayDecision::Fresh(pending) => {
            ReplayMetrics::record(
                ReplayMetricOperation::Check,
                ReplayMetricOutcome::Completed,
                ReplayMetricReason::Fresh,
            );
            replay_ops::reserve_root_replay(
                &pending,
                MAX_ROOT_REPLAY_ENTRIES,
                MAX_ROOT_REPLAY_ENTRIES_PER_CALLER,
            )
            .map_err(map_replay_reserve_error)?;
            ReplayMetrics::record(
                ReplayMetricOperation::Reserve,
                ReplayMetricOutcome::Completed,
                ReplayMetricReason::Ok,
            );
            RootCapabilityMetrics::record_replay(
                RootCapabilityMetricKey::RequestCycles,
                RootCapabilityMetricOutcome::Accepted,
            );
            Ok(ReplayPreflight::Fresh(pending))
        }
        ReplayDecision::DuplicateSame(cached) => {
            ReplayMetrics::record(
                ReplayMetricOperation::Check,
                ReplayMetricOutcome::Completed,
                ReplayMetricReason::Duplicate,
            );
            RootCapabilityMetrics::record_replay(
                RootCapabilityMetricKey::RequestCycles,
                RootCapabilityMetricOutcome::DuplicateSame,
            );
            decode_cycles_response(&cached.response_bytes).map(ReplayPreflight::Cached)
        }
        ReplayDecision::InFlight => {
            ReplayMetrics::record(
                ReplayMetricOperation::Check,
                ReplayMetricOutcome::Failed,
                ReplayMetricReason::InFlight,
            );
            RootCapabilityMetrics::record_replay(
                RootCapabilityMetricKey::RequestCycles,
                RootCapabilityMetricOutcome::DuplicateSame,
            );
            Err(RpcWorkflowError::ReplayDuplicateSame("RequestCycles").into())
        }
        ReplayDecision::DuplicateConflict => {
            ReplayMetrics::record(
                ReplayMetricOperation::Check,
                ReplayMetricOutcome::Failed,
                ReplayMetricReason::Conflict,
            );
            RootCapabilityMetrics::record_replay(
                RootCapabilityMetricKey::RequestCycles,
                RootCapabilityMetricOutcome::DuplicateConflict,
            );
            Err(RpcWorkflowError::ReplayConflict("RequestCycles").into())
        }
        ReplayDecision::Expired => {
            ReplayMetrics::record(
                ReplayMetricOperation::Check,
                ReplayMetricOutcome::Failed,
                ReplayMetricReason::Expired,
            );
            RootCapabilityMetrics::record_replay(
                RootCapabilityMetricKey::RequestCycles,
                RootCapabilityMetricOutcome::Expired,
            );
            Err(RpcWorkflowError::ReplayExpired("RequestCycles").into())
        }
        ReplayDecision::DecodeFailed(message) => Err(map_replay_decode_error(
            ReplayDecodeError::DecodeFailed(message),
        )),
    }
}

fn reserve_request_cycles_cost_guard(
    ctx: &RootContext,
    approved_cycles: u128,
) -> Result<CostGuardPermit, InternalError> {
    CostGuardOps::reserve(request_cycles_cost_guard_request(
        ctx,
        approved_cycles,
        MgmtOps::canister_cycle_balance().to_u128(),
    ))
    .map_err(map_cost_guard_reserve_error)
}

pub(super) fn request_cycles_cost_guard_request(
    ctx: &RootContext,
    approved_cycles: u128,
    current_cycle_balance: u128,
) -> CostGuardRequest {
    CostGuardRequest {
        cost_class: CostClass::ValueTransfer,
        command_kind: root_request_cycles_command_kind()
            .expect("root request cycles command kind is valid"),
        quota_subject: ctx.caller,
        payer: ctx.self_pid,
        now_secs: ctx.now,
        quota_window_secs: ROOT_REQUEST_CYCLES_VALUE_TRANSFER_QUOTA_WINDOW_SECONDS,
        max_operations_per_window: MAX_ROOT_REQUEST_CYCLES_VALUE_TRANSFER_OPERATIONS_PER_WINDOW,
        current_cycle_balance,
        cycle_reservation_cycles: approved_cycles,
        min_cycles_after_reservation: MIN_ROOT_REQUEST_CYCLES_AFTER_RESERVATION,
    }
}

fn root_request_cycles_command_kind()
-> Result<CommandKind, crate::ops::replay::model::CommandKindError> {
    CommandKind::new(ROOT_REQUEST_CYCLES_COMMAND_KIND)
}

pub(super) fn mark_request_cycles_external_effect(
    pending: &ReplayPending,
    ctx: &RootContext,
    approved_cycles: u128,
) {
    replay_ops::mark_root_replay_external_effect(
        pending,
        ExternalEffectDescriptor::ManagementCall {
            canister: ctx.caller,
            method: "deposit_cycles".to_string(),
        },
        replay_ops::guard::secs_to_ns(IcOps::now_secs()),
    );
    log!(
        Topic::Rpc,
        Info,
        "request cycles replay effect marked effect=deposit_cycles command_kind={} caller={} approved_cycles={}",
        ROOT_REQUEST_CYCLES_COMMAND_KIND,
        ctx.caller,
        approved_cycles
    );
}

fn mark_request_cycles_recovery_required(
    pending: &ReplayPending,
    ctx: &RootContext,
    approved_cycles: u128,
    err: &InternalError,
) {
    let (error_class, error_origin) = err.log_fields();
    replay_ops::mark_root_replay_recovery_required(
        pending,
        RecoveryReason::ExternalEffectStatusUnknown,
        replay_ops::guard::secs_to_ns(IcOps::now_secs()),
    );
    log!(
        Topic::Rpc,
        Error,
        "request cycles replay recovery required effect=deposit_cycles command_kind={} caller={} approved_cycles={} error_class={} error_origin={}",
        ROOT_REQUEST_CYCLES_COMMAND_KIND,
        ctx.caller,
        approved_cycles,
        error_class,
        error_origin
    );
}

fn map_replay_guard_error(err: ReplayGuardError) -> InternalError {
    match err {
        ReplayGuardError::InvalidTtl { ttl_ns, max_ttl_ns } => {
            ReplayMetrics::record(
                ReplayMetricOperation::Check,
                ReplayMetricOutcome::Failed,
                ReplayMetricReason::InvalidTtl,
            );
            RootCapabilityMetrics::record_replay(
                RootCapabilityMetricKey::RequestCycles,
                RootCapabilityMetricOutcome::TtlExceeded,
            );
            RpcWorkflowError::InvalidReplayTtl { ttl_ns, max_ttl_ns }.into()
        }
        ReplayGuardError::TtlOverflow { now_ns, ttl_ns } => {
            ReplayMetrics::record(
                ReplayMetricOperation::Check,
                ReplayMetricOutcome::Failed,
                ReplayMetricReason::InvalidTtl,
            );
            RootCapabilityMetrics::record_replay(
                RootCapabilityMetricKey::RequestCycles,
                RootCapabilityMetricOutcome::TtlExceeded,
            );
            RpcWorkflowError::ReplayTtlOverflow { now_ns, ttl_ns }.into()
        }
        ReplayGuardError::ReceiptDecodeFailed(message) => {
            map_replay_decode_error(ReplayDecodeError::DecodeFailed(message))
        }
    }
}

fn map_replay_reserve_error(err: ReplayReserveError) -> InternalError {
    match err {
        ReplayReserveError::CapacityReached { max_entries } => {
            ReplayMetrics::record(
                ReplayMetricOperation::Reserve,
                ReplayMetricOutcome::Failed,
                ReplayMetricReason::Capacity,
            );
            RpcWorkflowError::ReplayStoreCapacityReached(max_entries).into()
        }
        ReplayReserveError::CallerCapacityReached {
            caller,
            max_entries,
        } => {
            ReplayMetrics::record(
                ReplayMetricOperation::Reserve,
                ReplayMetricOutcome::Failed,
                ReplayMetricReason::Capacity,
            );
            RpcWorkflowError::ReplayStoreCallerCapacityReached {
                caller,
                max_entries,
            }
            .into()
        }
    }
}

fn map_replay_decode_error(err: ReplayDecodeError) -> InternalError {
    match err {
        ReplayDecodeError::DecodeFailed(message) => {
            ReplayMetrics::record(
                ReplayMetricOperation::Decode,
                ReplayMetricOutcome::Failed,
                ReplayMetricReason::DecodeFailed,
            );
            RpcWorkflowError::ReplayDecodeFailed(message).into()
        }
    }
}

fn decode_cycles_response(bytes: &[u8]) -> Result<CyclesResponse, InternalError> {
    match replay_ops::decode_root_cycles_replay_response(bytes) {
        Ok(response) => {
            ReplayMetrics::record(
                ReplayMetricOperation::Decode,
                ReplayMetricOutcome::Completed,
                ReplayMetricReason::Ok,
            );
            Ok(response)
        }
        Err(err) => Err(map_replay_decode_error(err)),
    }
}

fn commit_cycles_replay(pending: ReplayPending, response: &CyclesResponse) {
    replay_ops::commit_root_cycles_replay(pending, response);
    ReplayMetrics::record(
        ReplayMetricOperation::Commit,
        ReplayMetricOutcome::Completed,
        ReplayMetricReason::Ok,
    );
}

fn abort_replay(pending: ReplayPending) {
    replay_ops::abort_root_replay(pending);
    ReplayMetrics::record(
        ReplayMetricOperation::Abort,
        ReplayMetricOutcome::Completed,
        ReplayMetricReason::Ok,
    );
}

fn hash_cycles_payload(req: &CyclesRequest) -> [u8; 32] {
    let mut hasher = super::replay::payload_hasher();
    super::replay::hash_str(&mut hasher, "RequestCycles");
    super::replay::hash_u128(&mut hasher, req.cycles);
    super::replay::finish_payload_hash(hasher)
}
