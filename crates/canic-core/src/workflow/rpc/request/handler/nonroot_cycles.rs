//! Module: workflow::rpc::request::handler::nonroot_cycles
//!
//! Responsibility: authorize and execute replay-protected non-root cycles requests.
//! Does not own: endpoint auth, stable replay receipts, or management-call primitives.
//! Boundary: RPC request handler calls this for root and non-root cycles funding paths.

use super::{RootCapability, RootContext, replay};
use crate::{
    InternalError,
    cdk::types::Principal,
    domain::policy::pure::cycles_funding::{FundingPolicyViolation, evaluate},
    dto::rpc::{CyclesFundingPreflightResponse, CyclesRequest, CyclesResponse},
    ids::CanisterRole,
    log,
    log::Topic,
    model::replay::{
        CommandKind, ExternalEffectDescriptor, OperationId, RecoveryReason, ReplayActor,
    },
    ops::{
        config::ConfigOps,
        cost_guard::{CostGuardPermit, CostGuardRequest},
        ic::{IcOps, mgmt::MgmtOps},
        replay::{self as replay_ops, guard::ReplayPending},
        runtime::{
            cycles_funding::CyclesFundingLedgerOps,
            env::EnvOps,
            metrics::{
                cycles_funding::{CyclesFundingDeniedReason, CyclesFundingMetrics},
                root_capability::{
                    RootCapabilityMetricKey, RootCapabilityMetricOutcome, RootCapabilityMetrics,
                },
            },
        },
        storage::{children::CanisterChildrenOps, replay::ReplayReceiptOps},
    },
    replay_policy::CostClass,
    workflow::{
        cost_guard::{CostGuardWorkflow, map_cost_guard_reserve_error},
        replay::mark_recovery_required_after_failure,
        rpc::{RootCapabilityAuthority, RpcWorkflowError},
    },
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

pub(in crate::workflow::rpc) struct NonrootCyclesCapabilityWorkflow;

///
/// AuthorizedCyclesGrant
///
/// Approved cycle transfer amount after authorization and policy checks.
/// Owned by RPC workflow and passed into execution helpers.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct AuthorizedCyclesGrant {
    approved_cycles: u128,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum CyclesAuthorization {
    Grant(AuthorizedCyclesGrant),
    Preflight(CyclesFundingPreflightResponse),
}

#[derive(Debug)]
enum CyclesAuthorizationError {
    Internal(InternalError),
    Preflight(CyclesFundingPreflightResponse),
}

impl From<InternalError> for CyclesAuthorizationError {
    fn from(error: InternalError) -> Self {
        Self::Internal(error)
    }
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
    pub(in crate::workflow::rpc) async fn response_replay_first(
        req: CyclesRequest,
    ) -> Result<CyclesResponse, InternalError> {
        let ctx = extract_cycles_context(false)?;
        let child = direct_child_record(ctx.caller);
        response_replay_first_with_child(ctx, req, child).await
    }
}

pub(super) async fn response_replay_first_root(
    req: CyclesRequest,
    authority: &RootCapabilityAuthority,
) -> Result<CyclesResponse, InternalError> {
    let ctx = extract_cycles_context(true)?;
    let child = component_registry_child_record(&ctx, authority);
    response_replay_first_with_child(ctx, req, child).await
}

async fn response_replay_first_with_child(
    ctx: RootContext,
    req: CyclesRequest,
    child: Option<ResolvedCyclesChild>,
) -> Result<CyclesResponse, InternalError> {
    let capability = RootCapability::RequestCycles(req.clone());
    let pending = match replay::check_replay(&ctx, &capability)? {
        replay::ReplayPreflight::Fresh(pending) => pending,
        replay::ReplayPreflight::Cached(crate::dto::rpc::Response::Cycles(response)) => {
            return Ok(response);
        }
        replay::ReplayPreflight::Cached(_) => {
            return Err(InternalError::invariant());
        }
    };

    let grant = match authorize_request_cycles_with_child(&ctx, &req, child) {
        Ok(grant) => grant,
        Err(CyclesAuthorizationError::Internal(err)) => {
            return Err(replay::abort_replay_after_failure(pending, err));
        }
        Err(CyclesAuthorizationError::Preflight(preflight)) => {
            replay::abort_replay(pending)?;
            RootCapabilityMetrics::record_execution(
                RootCapabilityMetricKey::RequestCycles,
                RootCapabilityMetricOutcome::Success,
            );
            return Ok(CyclesResponse::PreflightRejected(preflight));
        }
    };

    let response = match execute_authorized_request_cycles(&ctx, &pending, grant).await {
        Ok(response) => response,
        Err(err) => {
            let err = replay::abort_replay_after_failure(pending, err);
            RootCapabilityMetrics::record_execution(
                RootCapabilityMetricKey::RequestCycles,
                RootCapabilityMetricOutcome::Error,
            );
            return Err(err);
        }
    };

    if let Err(err) = replay::commit_replay(&pending) {
        if let Err(_recovery_err) =
            replay::mark_recovery_required(&pending, RecoveryReason::ResponseCommitFailed)
        {
            return Err(err);
        }
        return Err(err);
    }

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
    authority: &RootCapabilityAuthority,
) -> Result<(), InternalError> {
    authorize_root_request_cycles_plan(ctx, req, authority).map(|_| ())
}

/// Resolve an approved non-root cycles grant in one authorization pass.
pub(super) fn authorize_request_cycles_plan(
    ctx: &RootContext,
    req: &CyclesRequest,
) -> Result<CyclesAuthorization, InternalError> {
    authorize_request_cycles_plan_with_child(ctx, req, direct_child_record(ctx.caller))
}

/// Resolve an approved root cycles grant in one authorization pass.
pub(super) fn authorize_root_request_cycles_plan(
    ctx: &RootContext,
    req: &CyclesRequest,
    authority: &RootCapabilityAuthority,
) -> Result<CyclesAuthorization, InternalError> {
    authorize_request_cycles_plan_with_child(
        ctx,
        req,
        component_registry_child_record(ctx, authority),
    )
}

fn authorize_request_cycles_plan_with_child(
    ctx: &RootContext,
    req: &CyclesRequest,
    child: Option<ResolvedCyclesChild>,
) -> Result<CyclesAuthorization, InternalError> {
    match authorize_request_cycles_with_child(ctx, req, child) {
        Ok(grant) => Ok(CyclesAuthorization::Grant(grant)),
        Err(CyclesAuthorizationError::Preflight(preflight)) => {
            Ok(CyclesAuthorization::Preflight(preflight))
        }
        Err(CyclesAuthorizationError::Internal(error)) => Err(error),
    }
}

fn authorize_request_cycles_with_child(
    ctx: &RootContext,
    req: &CyclesRequest,
    child: Option<ResolvedCyclesChild>,
) -> Result<AuthorizedCyclesGrant, CyclesAuthorizationError> {
    let decision = authorize_request_cycles_inner(ctx, req, child);

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
        Err(CyclesAuthorizationError::Internal(err)) => {
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
        Err(CyclesAuthorizationError::Preflight(preflight)) => {
            RootCapabilityMetrics::record_authorization(
                RootCapabilityMetricKey::RequestCycles,
                RootCapabilityMetricOutcome::Denied,
            );
            log!(
                Topic::Rpc,
                Info,
                "capability preflight rejected (capability=RequestCycles, caller={}, subnet={}, now={}, outcome={preflight:?})",
                ctx.caller,
                ctx.subnet_id,
                ctx.now
            );
        }
    }

    decision
}

/// Apply the existing cycles funding policy and structural child checks.
fn authorize_request_cycles_inner(
    ctx: &RootContext,
    req: &CyclesRequest,
    child: Option<ResolvedCyclesChild>,
) -> Result<AuthorizedCyclesGrant, CyclesAuthorizationError> {
    CyclesFundingMetrics::record_requested(ctx.caller, req.cycles);

    let Some(child) = child else {
        CyclesFundingMetrics::record_denied(
            ctx.caller,
            req.cycles,
            CyclesFundingDeniedReason::ChildNotFound,
        );
        return Err(InternalError::from(RpcWorkflowError::ChildNotFound(ctx.caller)).into());
    };
    if child.parent_pid != Some(ctx.self_pid) {
        CyclesFundingMetrics::record_denied(
            ctx.caller,
            req.cycles,
            CyclesFundingDeniedReason::NotDirectChild,
        );
        return Err(InternalError::from(RpcWorkflowError::NotChildOfCaller(
            ctx.caller,
            ctx.self_pid,
        ))
        .into());
    }

    if !crate::ops::storage::state::fleet::FleetStateOps::cycles_funding_enabled() {
        CyclesFundingMetrics::record_denied(
            ctx.caller,
            req.cycles,
            CyclesFundingDeniedReason::KillSwitchDisabled,
        );
        return Err(InternalError::from(RpcWorkflowError::CyclesFundingDisabled).into());
    }

    reject_competing_funding_operation(ctx, req)?;

    let limits = if ctx.is_root_env {
        ConfigOps::cycles_funding_limits_for_root_child_role(&child.role)?
    } else {
        ConfigOps::cycles_funding_limits_for_component_child_role(&child.role)?
    };
    let ledger = CyclesFundingLedgerOps::snapshot(ctx.caller);
    let decision = match evaluate(limits, ledger, req.cycles, ctx.now) {
        Ok(decision) => decision,
        Err(violation) => {
            return Err(CyclesAuthorizationError::Preflight(
                map_funding_policy_violation(ctx, req.cycles, violation),
            ));
        }
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

    let available = IcOps::canister_cycle_balance().to_u128();
    if let Some(preflight) = parent_funding_preflight(decision.approved_cycles, available) {
        CyclesFundingMetrics::record_denied(
            ctx.caller,
            decision.approved_cycles,
            CyclesFundingDeniedReason::InsufficientCycles,
        );
        return Err(CyclesAuthorizationError::Preflight(preflight));
    }

    Ok(AuthorizedCyclesGrant {
        approved_cycles: decision.approved_cycles,
    })
}

fn reject_competing_funding_operation(
    ctx: &RootContext,
    req: &CyclesRequest,
) -> Result<(), InternalError> {
    // Replay is the durable in-flight authority. Excluding this request's own
    // operation keeps first admission valid while another pending operation
    // for the same child blocks stale whole-ledger rollback across an await.
    let Some(metadata) = req.metadata else {
        return Ok(());
    };
    let command_kind = root_request_cycles_command_kind()
        .expect("root request cycles command kind is a valid static label");
    let operation_id = OperationId::from_bytes(metadata.request_id);
    if !ReplayReceiptOps::has_pending_for_actor_command_excluding_operation(
        ReplayActor::direct_caller(ctx.caller),
        &command_kind,
        operation_id,
        replay_ops::guard::secs_to_ns(ctx.now),
    ) {
        return Ok(());
    }

    CyclesFundingMetrics::record_denied(
        ctx.caller,
        req.cycles,
        CyclesFundingDeniedReason::OperationInProgress,
    );
    Err(RpcWorkflowError::FundingOperationInProgress { child: ctx.caller }.into())
}

/// Execute an already-authorized cycles transfer.
async fn execute_authorized_request_cycles(
    ctx: &RootContext,
    pending: &ReplayPending,
    grant: AuthorizedCyclesGrant,
) -> Result<CyclesResponse, InternalError> {
    let cost_permit = reserve_request_cycles_cost_guard(ctx, grant.approved_cycles)?;
    mark_request_cycles_external_effect(pending, ctx, grant.approved_cycles, &cost_permit)?;
    let ledger_before_grant = CyclesFundingLedgerOps::snapshot(ctx.caller);
    CyclesFundingLedgerOps::record_child_grant(ctx.caller, grant.approved_cycles, ctx.now);

    if let Err(err) =
        MgmtOps::deposit_cycles_with_permit(&cost_permit, ctx.caller, grant.approved_cycles).await
    {
        CyclesFundingLedgerOps::restore_child_snapshot(ctx.caller, ledger_before_grant);
        let err = CostGuardWorkflow::recover_after_failure(&cost_permit, IcOps::now_secs(), err);
        let err =
            preserve_request_cycles_recovery_required(pending, ctx, grant.approved_cycles, err);
        CyclesFundingMetrics::record_denied(
            ctx.caller,
            grant.approved_cycles,
            CyclesFundingDeniedReason::ExecutionError,
        );
        return Err(err);
    }

    CyclesFundingMetrics::record_granted(ctx.caller, grant.approved_cycles);

    let response = CyclesResponse::Transferred {
        cycles_transferred: grant.approved_cycles,
    };
    if let Err(err) = replay::stage_response(pending, &crate::dto::rpc::Response::Cycles(response))
    {
        let reason = match CostGuardWorkflow::complete(&cost_permit, IcOps::now_secs()) {
            Ok(()) => RecoveryReason::ResponseCommitFailed,
            Err(_settlement_err) => RecoveryReason::CostSettlementFailed,
        };
        let _ = replay::mark_recovery_required(pending, reason);
        return Err(err);
    }

    if let Err(err) = CostGuardWorkflow::complete(&cost_permit, IcOps::now_secs()) {
        if let Err(_recovery_err) =
            replay::mark_recovery_required(pending, RecoveryReason::CostSettlementFailed)
        {
            return Err(err);
        }
        return Err(err);
    }

    Ok(response)
}

fn direct_child_record(pid: Principal) -> Option<ResolvedCyclesChild> {
    CanisterChildrenOps::role_parent(pid)
        .map(|(role, parent_pid)| ResolvedCyclesChild { role, parent_pid })
}

fn component_registry_child_record(
    ctx: &RootContext,
    authority: &RootCapabilityAuthority,
) -> Option<ResolvedCyclesChild> {
    if authority.caller_canister_id() != ctx.caller {
        return None;
    }
    Some(ResolvedCyclesChild {
        role: authority.caller_role()?.clone(),
        parent_pid: authority.caller_parent_canister_id(),
    })
}

pub(super) const fn parent_funding_preflight(
    approved_cycles: u128,
    available_cycles: u128,
) -> Option<CyclesFundingPreflightResponse> {
    if approved_cycles > available_cycles {
        Some(CyclesFundingPreflightResponse::ParentFundingUnavailable { approved_cycles })
    } else {
        None
    }
}

pub(super) fn map_funding_policy_violation(
    ctx: &RootContext,
    requested_cycles: u128,
    violation: FundingPolicyViolation,
) -> CyclesFundingPreflightResponse {
    match &violation {
        FundingPolicyViolation::MaxPerChild { .. } => {
            CyclesFundingMetrics::record_denied(
                ctx.caller,
                requested_cycles,
                CyclesFundingDeniedReason::MaxPerChildExceeded,
            );
        }
        FundingPolicyViolation::CooldownActive { .. } => {
            CyclesFundingMetrics::record_denied(
                ctx.caller,
                requested_cycles,
                CyclesFundingDeniedReason::CooldownActive,
            );
        }
    }
    funding_policy_preflight(violation)
}

pub(super) const fn funding_policy_preflight(
    violation: FundingPolicyViolation,
) -> CyclesFundingPreflightResponse {
    match violation {
        FundingPolicyViolation::MaxPerChild {
            max_per_child,
            remaining_budget,
        } => CyclesFundingPreflightResponse::ChildBudgetExhausted {
            remaining_child_budget: remaining_budget,
            max_per_child,
        },
        FundingPolicyViolation::CooldownActive { retry_after_secs } => {
            CyclesFundingPreflightResponse::CooldownActive { retry_after_secs }
        }
    }
}

fn reserve_request_cycles_cost_guard(
    ctx: &RootContext,
    approved_cycles: u128,
) -> Result<CostGuardPermit, InternalError> {
    CostGuardWorkflow::reserve(request_cycles_cost_guard_request(
        ctx,
        approved_cycles,
        IcOps::canister_cycle_balance().to_u128(),
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

fn root_request_cycles_command_kind() -> Result<CommandKind, crate::model::replay::CommandKindError>
{
    CommandKind::new(ROOT_REQUEST_CYCLES_COMMAND_KIND)
}

pub(super) fn mark_request_cycles_external_effect(
    pending: &ReplayPending,
    ctx: &RootContext,
    approved_cycles: u128,
    cost_permit: &CostGuardPermit,
) -> Result<(), InternalError> {
    if let Err(err) = replay_ops::mark_root_replay_costed_external_effect(
        pending,
        ExternalEffectDescriptor::ManagementCall {
            canister: ctx.caller,
            method: "deposit_cycles".to_string(),
        },
        cost_permit,
        replay_ops::guard::secs_to_ns(IcOps::now_secs()),
    )
    .map_err(replay::map_replay_store_error)
    {
        return Err(CostGuardWorkflow::recover_after_failure(
            cost_permit,
            IcOps::now_secs(),
            err,
        ));
    }
    log!(
        Topic::Rpc,
        Info,
        "request cycles replay effect marked effect=deposit_cycles command_kind={} caller={} approved_cycles={}",
        ROOT_REQUEST_CYCLES_COMMAND_KIND,
        ctx.caller,
        approved_cycles
    );
    Ok(())
}

fn preserve_request_cycles_recovery_required(
    pending: &ReplayPending,
    ctx: &RootContext,
    approved_cycles: u128,
    err: InternalError,
) -> InternalError {
    let diagnostic = err.code();
    let err = mark_recovery_required_after_failure(
        &pending.receipt_token,
        RecoveryReason::ExternalEffectStatusUnknown,
        replay_ops::guard::secs_to_ns(IcOps::now_secs()),
        err,
        "request cycles replay recovery marker failed",
    );
    log!(
        Topic::Rpc,
        Error,
        "request cycles replay recovery required effect=deposit_cycles command_kind={} caller={} approved_cycles={} diagnostic={}",
        ROOT_REQUEST_CYCLES_COMMAND_KIND,
        ctx.caller,
        approved_cycles,
        diagnostic
    );
    err
}
