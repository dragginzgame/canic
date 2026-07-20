//! Module: workflow::rpc::request::handler::nonroot_cycles
//!
//! Responsibility: authorize and execute replay-protected non-root cycles requests.
//! Does not own: endpoint auth, stable replay receipts, or management-call primitives.
//! Boundary: RPC request handler calls this for root and non-root cycles funding paths.

use super::{RootCapability, RootContext, replay};
use crate::{
    InternalError, InternalErrorOrigin,
    cdk::types::Principal,
    domain::policy::pure::cycles_funding::{FundingPolicyViolation, evaluate},
    dto::rpc::{CyclesRequest, CyclesResponse},
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
        storage::{
            children::CanisterChildrenOps, registry::subnet::SubnetRegistryOps,
            replay::ReplayReceiptOps,
        },
    },
    replay_policy::CostClass,
    workflow::{
        cost_guard::{CostGuardWorkflow, map_cost_guard_reserve_error},
        replay::mark_recovery_required_after_failure,
        rpc::RpcWorkflowError,
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
    pub(in crate::workflow::rpc) async fn response_replay_first(
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

async fn response_replay_first_with_planner(
    ctx: RootContext,
    req: CyclesRequest,
    authorize_plan: fn(
        &RootContext,
        &CyclesRequest,
    ) -> Result<AuthorizedCyclesGrant, InternalError>,
) -> Result<CyclesResponse, InternalError> {
    let capability = RootCapability::RequestCycles(req.clone());
    let pending = match replay::check_replay(&ctx, &capability)? {
        replay::ReplayPreflight::Fresh(pending) => pending,
        replay::ReplayPreflight::Cached(crate::dto::rpc::Response::Cycles(response)) => {
            return Ok(response);
        }
        replay::ReplayPreflight::Cached(_) => {
            return Err(InternalError::invariant(
                InternalErrorOrigin::Workflow,
                "request cycles replay returned a non-cycles response",
            ));
        }
    };

    let grant = match authorize_plan(&ctx, &req) {
        Ok(grant) => grant,
        Err(err) => {
            return Err(replay::abort_replay_after_failure(pending, err));
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
        if let Err(recovery_err) =
            replay::mark_recovery_required(&pending, RecoveryReason::ResponseCommitFailed)
        {
            return Err(err.with_diagnostic_context(format!(
                "request cycles replay recovery marker failed: {recovery_err}"
            )));
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

    reject_competing_funding_operation(ctx, req)?;

    let limits = ConfigOps::cycles_funding_limits_for_child_role(&child.role)?;
    let ledger = CyclesFundingLedgerOps::snapshot(ctx.caller);
    let decision = match evaluate(limits, ledger, req.cycles, ctx.now) {
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

    let available = IcOps::canister_cycle_balance().to_u128();
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

    let response = CyclesResponse {
        cycles_transferred: grant.approved_cycles,
    };
    if let Err(err) = replay::stage_response(
        pending,
        &crate::dto::rpc::Response::Cycles(response.clone()),
    ) {
        let mut err = err;
        let reason = match CostGuardWorkflow::complete(&cost_permit, IcOps::now_secs()) {
            Ok(()) => RecoveryReason::ResponseCommitFailed,
            Err(settlement_err) => {
                err = err.with_diagnostic_context(format!(
                    "request cycles cost settlement also failed: {settlement_err}"
                ));
                RecoveryReason::CostSettlementFailed
            }
        };
        if let Err(recovery_err) = replay::mark_recovery_required(pending, reason) {
            err = err.with_diagnostic_context(format!(
                "request cycles replay recovery marker failed: {recovery_err}"
            ));
        }
        return Err(err);
    }

    if let Err(err) = CostGuardWorkflow::complete(&cost_permit, IcOps::now_secs()) {
        if let Err(recovery_err) =
            replay::mark_recovery_required(pending, RecoveryReason::CostSettlementFailed)
        {
            return Err(err.with_diagnostic_context(format!(
                "request cycles replay recovery marker failed: {recovery_err}"
            )));
        }
        return Err(err);
    }

    Ok(response)
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
    let (error_class, error_origin) = err.log_fields();
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
        "request cycles replay recovery required effect=deposit_cycles command_kind={} caller={} approved_cycles={} error_class={} error_origin={}",
        ROOT_REQUEST_CYCLES_COMMAND_KIND,
        ctx.caller,
        approved_cycles,
        error_class,
        error_origin
    );
    err
}
