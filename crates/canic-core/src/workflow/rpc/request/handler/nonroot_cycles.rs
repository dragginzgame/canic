use super::{
    MAX_ROOT_REPLAY_ENTRIES, MAX_ROOT_TTL_SECONDS, REPLAY_PURGE_SCAN_LIMIT, RootContext, funding,
};
use crate::{
    InternalError,
    cdk::types::Principal,
    dto::rpc::{CyclesRequest, CyclesResponse},
    log,
    log::Topic,
    ops::{
        ic::{IcOps, mgmt::MgmtOps},
        replay::{
            self as replay_ops, ReplayDecodeError, ReplayReserveError,
            guard::{ReplayDecision, ReplayGuardError, ReplayPending, RootReplayGuardInput},
        },
        runtime::env::EnvOps,
        runtime::metrics::{
            cycles_funding::{CyclesFundingDeniedReason, CyclesFundingMetrics},
            root_capability::{
                RootCapabilityMetricKey, RootCapabilityMetricOutcome, RootCapabilityMetrics,
            },
        },
        storage::{children::CanisterChildrenOps, registry::subnet::SubnetRegistryOps},
    },
    storage::canister::CanisterRecord,
    workflow::rpc::RpcWorkflowError,
};

///
/// NonrootCyclesCapabilityWorkflow
///

pub struct NonrootCyclesCapabilityWorkflow;

#[derive(Clone, Copy, Debug)]
pub(super) struct AuthorizedCyclesGrant {
    approved_cycles: u128,
}

impl NonrootCyclesCapabilityWorkflow {
    /// Execute the non-root cycles capability path with replay-first semantics.
    pub async fn response_replay_first(
        req: CyclesRequest,
    ) -> Result<CyclesResponse, InternalError> {
        let ctx = extract_root_context()?;
        let preflight = check_cycles_replay(&ctx, &req)?;
        let pending = match preflight {
            ReplayPreflight::Fresh(pending) => pending,
            ReplayPreflight::Cached(response) => return Ok(response),
        };

        let grant = match authorize_request_cycles_plan(&ctx, &req) {
            Ok(grant) => grant,
            Err(err) => {
                abort_replay(pending);
                return Err(err);
            }
        };

        let response = match execute_authorized_request_cycles(&ctx, grant).await {
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
}

enum ReplayPreflight {
    Fresh(ReplayPending),
    Cached(CyclesResponse),
}

// Build the current root-like execution context for non-root cycles requests.
fn extract_root_context() -> Result<RootContext, InternalError> {
    Ok(RootContext {
        caller: IcOps::msg_caller(),
        self_pid: IcOps::canister_self(),
        is_root_env: EnvOps::is_root(),
        subnet_id: EnvOps::subnet_pid()?,
        now: IcOps::now_secs(),
    })
}

// Run cycles authorization while preserving the existing root-capability metrics.
pub(super) fn authorize_request_cycles(
    ctx: &RootContext,
    req: &CyclesRequest,
) -> Result<(), InternalError> {
    authorize_request_cycles_plan(ctx, req).map(|_| ())
}

// Run root cycles authorization against the authoritative subnet registry.
pub(super) fn authorize_root_request_cycles(
    ctx: &RootContext,
    req: &CyclesRequest,
) -> Result<(), InternalError> {
    authorize_root_request_cycles_plan(ctx, req).map(|_| ())
}

// Resolve an approved non-root cycles grant in one authorization pass.
pub(super) fn authorize_request_cycles_plan(
    ctx: &RootContext,
    req: &CyclesRequest,
) -> Result<AuthorizedCyclesGrant, InternalError> {
    authorize_request_cycles_with_resolver(ctx, req, direct_child_record)
}

// Resolve an approved root cycles grant in one authorization pass.
pub(super) fn authorize_root_request_cycles_plan(
    ctx: &RootContext,
    req: &CyclesRequest,
) -> Result<AuthorizedCyclesGrant, InternalError> {
    authorize_request_cycles_with_resolver(ctx, req, registry_child_record)
}

// Apply cycles authorization with a caller->child lookup chosen by the caller type.
fn authorize_request_cycles_with_resolver(
    ctx: &RootContext,
    req: &CyclesRequest,
    resolve_child: fn(Principal) -> Option<CanisterRecord>,
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

// Apply the existing cycles funding policy and structural child checks.
pub(super) fn authorize_request_cycles_inner(
    ctx: &RootContext,
    req: &CyclesRequest,
    resolve_child: fn(Principal) -> Option<CanisterRecord>,
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

    let policy = funding::policy_for_child_role(&child.role);
    let decision = match policy.evaluate(ctx.caller, req.cycles, ctx.now) {
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

// Execute the approved cycles transfer and return the canonical cycles response.
pub(super) async fn execute_request_cycles(
    ctx: &RootContext,
    req: &CyclesRequest,
) -> Result<CyclesResponse, InternalError> {
    let grant = authorize_request_cycles_plan(ctx, req)?;
    execute_authorized_request_cycles(ctx, grant).await
}

// Execute root cycles funding against the authoritative subnet registry.
pub(super) async fn execute_root_request_cycles(
    ctx: &RootContext,
    req: &CyclesRequest,
) -> Result<CyclesResponse, InternalError> {
    let grant = authorize_root_request_cycles_plan(ctx, req)?;
    execute_authorized_request_cycles(ctx, grant).await
}

// Execute an already-authorized cycles transfer.
pub(super) async fn execute_authorized_request_cycles(
    ctx: &RootContext,
    grant: AuthorizedCyclesGrant,
) -> Result<CyclesResponse, InternalError> {
    if let Err(err) = MgmtOps::deposit_cycles(ctx.caller, grant.approved_cycles).await {
        CyclesFundingMetrics::record_denied(
            ctx.caller,
            grant.approved_cycles,
            CyclesFundingDeniedReason::ExecutionError,
        );
        return Err(err);
    }

    CyclesFundingMetrics::record_granted(ctx.caller, grant.approved_cycles);
    funding::record_child_grant(ctx.caller, grant.approved_cycles, ctx.now);

    Ok(CyclesResponse {
        cycles_transferred: grant.approved_cycles,
    })
}

// Resolve one direct child record from the locally cascaded children cache.
fn direct_child_record(pid: Principal) -> Option<CanisterRecord> {
    CanisterChildrenOps::data()
        .entries
        .into_iter()
        .find_map(|(child_pid, record)| (child_pid == pid).then_some(record))
}

// Resolve one child record from the authoritative root subnet registry.
fn registry_child_record(pid: Principal) -> Option<CanisterRecord> {
    SubnetRegistryOps::get(pid)
}

// Map pure funding policy failures into workflow errors with current metrics.
fn map_funding_policy_violation(
    ctx: &RootContext,
    requested_cycles: u128,
    violation: funding::FundingPolicyViolation,
) -> InternalError {
    match violation {
        funding::FundingPolicyViolation::MaxPerChild {
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
        funding::FundingPolicyViolation::CooldownActive { retry_after_secs } => {
            CyclesFundingMetrics::record_denied(
                ctx.caller,
                requested_cycles,
                CyclesFundingDeniedReason::CooldownActive,
            );
            RpcWorkflowError::FundingCooldownActive { retry_after_secs }.into()
        }
    }
}

// Check replay state for a cycles request using the same root replay store and payload hash.
fn check_cycles_replay(
    ctx: &RootContext,
    req: &CyclesRequest,
) -> Result<ReplayPreflight, InternalError> {
    let metadata = req
        .metadata
        .ok_or(RpcWorkflowError::MissingReplayMetadata("RequestCycles"))?;
    let payload_hash = hash_cycles_payload(req);

    let decision = replay_ops::guard::evaluate_root_replay(RootReplayGuardInput {
        caller: ctx.caller,
        target_canister: ctx.self_pid,
        request_id: metadata.request_id,
        ttl_seconds: metadata.ttl_seconds,
        payload_hash,
        now: ctx.now,
        max_ttl_seconds: MAX_ROOT_TTL_SECONDS,
        purge_scan_limit: REPLAY_PURGE_SCAN_LIMIT,
    })
    .map_err(map_replay_guard_error)?;

    match decision {
        ReplayDecision::Fresh(pending) => {
            replay_ops::reserve_root_replay(pending, MAX_ROOT_REPLAY_ENTRIES)
                .map_err(map_replay_reserve_error)?;
            RootCapabilityMetrics::record_replay(
                RootCapabilityMetricKey::RequestCycles,
                RootCapabilityMetricOutcome::Accepted,
            );
            Ok(ReplayPreflight::Fresh(pending))
        }
        ReplayDecision::DuplicateSame(cached) => {
            RootCapabilityMetrics::record_replay(
                RootCapabilityMetricKey::RequestCycles,
                RootCapabilityMetricOutcome::DuplicateSame,
            );
            decode_cycles_response(&cached.response_bytes).map(ReplayPreflight::Cached)
        }
        ReplayDecision::InFlight => {
            RootCapabilityMetrics::record_replay(
                RootCapabilityMetricKey::RequestCycles,
                RootCapabilityMetricOutcome::DuplicateSame,
            );
            Err(RpcWorkflowError::ReplayDuplicateSame("RequestCycles").into())
        }
        ReplayDecision::DuplicateConflict => {
            RootCapabilityMetrics::record_replay(
                RootCapabilityMetricKey::RequestCycles,
                RootCapabilityMetricOutcome::DuplicateConflict,
            );
            Err(RpcWorkflowError::ReplayConflict("RequestCycles").into())
        }
        ReplayDecision::Expired => {
            RootCapabilityMetrics::record_replay(
                RootCapabilityMetricKey::RequestCycles,
                RootCapabilityMetricOutcome::Expired,
            );
            Err(RpcWorkflowError::ReplayExpired("RequestCycles").into())
        }
    }
}

// Convert replay guard failures into the existing workflow replay error surface.
fn map_replay_guard_error(err: ReplayGuardError) -> InternalError {
    match err {
        ReplayGuardError::InvalidTtl {
            ttl_seconds,
            max_ttl_seconds,
        } => {
            RootCapabilityMetrics::record_replay(
                RootCapabilityMetricKey::RequestCycles,
                RootCapabilityMetricOutcome::TtlExceeded,
            );
            RpcWorkflowError::InvalidReplayTtl {
                ttl_seconds,
                max_ttl_seconds,
            }
            .into()
        }
    }
}

// Convert replay reservation failures into the current workflow replay errors.
fn map_replay_reserve_error(err: ReplayReserveError) -> InternalError {
    match err {
        ReplayReserveError::CapacityReached { max_entries } => {
            RpcWorkflowError::ReplayStoreCapacityReached(max_entries).into()
        }
    }
}

// Convert replay decode failures into the current workflow replay errors.
fn map_replay_decode_error(err: ReplayDecodeError) -> InternalError {
    match err {
        ReplayDecodeError::DecodeFailed(message) => {
            RpcWorkflowError::ReplayDecodeFailed(message).into()
        }
    }
}

// Decode a cached replay entry back into the cycles response shape.
fn decode_cycles_response(bytes: &[u8]) -> Result<CyclesResponse, InternalError> {
    replay_ops::decode_root_cycles_replay_response(bytes).map_err(map_replay_decode_error)
}

// Persist a successful cycles response into the shared replay store.
fn commit_cycles_replay(pending: ReplayPending, response: &CyclesResponse) {
    replay_ops::commit_root_cycles_replay(pending, response);
}

// Abort an in-flight cycles replay reservation after a failed request.
fn abort_replay(pending: ReplayPending) {
    replay_ops::abort_root_replay(pending);
}

// Hash the canonical cycles payload using the shared replay field-hashing helpers.
fn hash_cycles_payload(req: &CyclesRequest) -> [u8; 32] {
    let mut hasher = super::replay::payload_hasher();
    super::replay::hash_str(&mut hasher, "RequestCycles");
    super::replay::hash_u128(&mut hasher, req.cycles);
    super::replay::finish_payload_hash(hasher)
}
