use super::{
    MAX_ROOT_REPLAY_ENTRIES, MAX_ROOT_TTL_SECONDS, REPLAY_PAYLOAD_HASH_DOMAIN,
    REPLAY_PURGE_SCAN_LIMIT, RootContext, funding,
};
use crate::{
    InternalError,
    dto::rpc::{CyclesRequest, CyclesResponse, Response, RootCapabilityCommand},
    log,
    log::Topic,
    ops::{
        ic::{IcOps, mgmt::MgmtOps},
        replay::{
            self as replay_ops, ReplayCommitError, ReplayReserveError,
            guard::{ReplayDecision, ReplayGuardError, ReplayPending, RootReplayGuardInput},
        },
        runtime::env::EnvOps,
        runtime::metrics::{
            cycles_funding::{CyclesFundingDeniedReason, CyclesFundingMetrics},
            root_capability::{
                RootCapabilityMetricKey, RootCapabilityMetricOutcome, RootCapabilityMetrics,
            },
        },
        storage::registry::subnet::SubnetRegistryOps,
    },
    workflow::rpc::RpcWorkflowError,
};
use candid::{decode_one, encode_one};
use sha2::{Digest, Sha256};

///
/// NonrootCyclesCapabilityWorkflow
///

pub struct NonrootCyclesCapabilityWorkflow;

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

        if let Err(err) = authorize_request_cycles(&ctx, &req) {
            abort_replay(pending);
            return Err(err);
        }

        let response = match execute_request_cycles(&ctx, &req).await {
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

        if let Err(err) = commit_cycles_replay(pending, &response) {
            log!(
                Topic::Rpc,
                Warn,
                "cycles replay finalize failed after successful execution (caller={}, subnet={}, now={}): {err}",
                ctx.caller,
                ctx.subnet_id,
                ctx.now
            );
        }

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
    let decision = authorize_request_cycles_inner(ctx, req);

    match &decision {
        Ok(()) => {
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
) -> Result<(), InternalError> {
    CyclesFundingMetrics::record_requested(ctx.caller, req.cycles);

    let Some(child) = SubnetRegistryOps::get(ctx.caller) else {
        CyclesFundingMetrics::record_denied(
            ctx.caller,
            req.cycles,
            CyclesFundingDeniedReason::ChildNotFound,
        );
        return Err(RpcWorkflowError::ChildNotFound(ctx.caller).into());
    };
    let root_self_request = ctx.is_root_env
        && ctx.caller == ctx.self_pid
        && child.role == crate::ids::CanisterRole::ROOT
        && child.parent_pid.is_none();
    if child.parent_pid != Some(ctx.self_pid) && !root_self_request {
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

    Ok(())
}

// Execute the approved cycles transfer and return the canonical cycles response.
pub(super) async fn execute_request_cycles(
    ctx: &RootContext,
    req: &CyclesRequest,
) -> Result<CyclesResponse, InternalError> {
    let child =
        SubnetRegistryOps::get(ctx.caller).ok_or(RpcWorkflowError::ChildNotFound(ctx.caller))?;
    let root_self_request = ctx.is_root_env
        && ctx.caller == ctx.self_pid
        && child.role == crate::ids::CanisterRole::ROOT
        && child.parent_pid.is_none();
    if child.parent_pid != Some(ctx.self_pid) && !root_self_request {
        return Err(RpcWorkflowError::NotChildOfCaller(ctx.caller, ctx.self_pid).into());
    }

    let policy = funding::policy_for_child_role(&child.role);
    let approved_cycles = match policy.evaluate(ctx.caller, req.cycles, ctx.now) {
        Ok(decision) => decision.approved_cycles,
        Err(violation) => return Err(map_funding_policy_violation(ctx, req.cycles, violation)),
    };

    if let Err(err) = MgmtOps::deposit_cycles(ctx.caller, approved_cycles).await {
        CyclesFundingMetrics::record_denied(
            ctx.caller,
            approved_cycles,
            CyclesFundingDeniedReason::ExecutionError,
        );
        return Err(err);
    }

    CyclesFundingMetrics::record_granted(ctx.caller, approved_cycles);
    funding::record_child_grant(ctx.caller, approved_cycles, ctx.now);

    Ok(CyclesResponse {
        cycles_transferred: approved_cycles,
    })
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
    let payload_hash = hash_cycles_payload(req)?;

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
            decode_cycles_response(&cached.response_candid).map(ReplayPreflight::Cached)
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

// Convert replay commit failures into the current workflow replay errors.
fn map_replay_commit_error(err: ReplayCommitError) -> InternalError {
    match err {
        ReplayCommitError::EncodeFailed(message) => {
            RpcWorkflowError::ReplayEncodeFailed(message).into()
        }
    }
}

// Decode a cached replay entry back into the cycles response shape.
fn decode_cycles_response(bytes: &[u8]) -> Result<CyclesResponse, InternalError> {
    let response: Response =
        decode_one(bytes).map_err(|err| RpcWorkflowError::ReplayDecodeFailed(err.to_string()))?;
    match response {
        Response::Cycles(response) => Ok(response),
        _ => Err(RpcWorkflowError::ReplayDecodeFailed(
            "cached replay payload was not a cycles response".to_string(),
        )
        .into()),
    }
}

// Persist a successful cycles response into the shared replay store.
fn commit_cycles_replay(
    pending: ReplayPending,
    response: &CyclesResponse,
) -> Result<(), InternalError> {
    replay_ops::commit_root_replay(pending, &Response::Cycles(response.clone()))
        .map_err(map_replay_commit_error)
}

// Abort an in-flight cycles replay reservation after a failed request.
fn abort_replay(pending: ReplayPending) {
    replay_ops::abort_root_replay(pending);
}

// Hash the canonical cycles payload using the same root replay domain and command shape.
fn hash_cycles_payload(req: &CyclesRequest) -> Result<[u8; 32], InternalError> {
    let mut canonical = req.clone();
    canonical.metadata = None;
    let payload = RootCapabilityCommand::RequestCycles(canonical);
    let bytes = encode_one(payload).map_err(|err| {
        RpcWorkflowError::ReplayEncodeFailed(format!("canonical payload encode failed: {err}"))
    })?;
    Ok(hash_domain_separated(REPLAY_PAYLOAD_HASH_DOMAIN, &bytes))
}

// Compute deterministic domain-separated replay hashes for cycles payloads.
fn hash_domain_separated(domain: &[u8], payload: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update((domain.len() as u64).to_be_bytes());
    hasher.update(domain);
    hasher.update((payload.len() as u64).to_be_bytes());
    hasher.update(payload);
    hasher.finalize().into()
}
