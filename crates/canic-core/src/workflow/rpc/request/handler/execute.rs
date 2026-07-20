//! Module: workflow::rpc::request::handler::execute
//!
//! Responsibility: execute authorized root capability requests.
//! Does not own: endpoint auth, replay guard classification, or storage schemas.
//! Boundary: RPC handler delegates capability side effects and response construction here.

use super::{
    RootCapability, RootContext, nonroot_cycles, nonroot_cycles::AuthorizedCyclesGrant, replay,
};
use crate::{
    InternalError,
    cdk::types::{Principal, TC},
    domain::policy::pure::topology::TopologyPolicyError,
    dto::error::Error,
    dto::rpc::{
        AcknowledgePlacementReceiptRequest, AcknowledgePlacementReceiptResponse,
        CreateCanisterParent, CreateCanisterRequest, CreateCanisterResponse,
        RecycleCanisterRequest, RecycleCanisterResponse, Response, UpgradeCanisterRequest,
        UpgradeCanisterResponse,
    },
    log,
    log::Topic,
    model::replay::{CommandKind, ExternalEffectDescriptor, OperationId, RecoveryReason},
    ops::{
        config::ConfigOps,
        cost_guard::{CostGuardPermit, CostGuardRequest},
        ic::IcOps,
        replay::{
            acknowledge_root_placement_receipt,
            guard::{ReplayPending, secs_to_ns},
            receipt::PlacementReceiptAcknowledgementDecision,
        },
        storage::{index::subnet::SubnetIndexOps, registry::subnet::SubnetRegistryOps},
    },
    replay_policy::CostClass,
    workflow::{
        canister_lifecycle::{
            CanisterLifecycleEvent, CanisterLifecycleWorkflow, CanisterUpgradeCostContext,
        },
        cost_guard::{CostGuardWorkflow, map_cost_guard_reserve_error},
        pool::PoolWorkflow,
        replay::mark_recovery_required_after_failure,
        rpc::RpcWorkflowError,
    },
};

const ROOT_PROVISION_DEPLOYMENT_QUOTA_WINDOW_SECONDS: u64 = 60;
const MAX_ROOT_PROVISION_DEPLOYMENT_OPERATIONS_PER_WINDOW: u64 = 10;
const MIN_ROOT_PROVISION_CYCLES_AFTER_RESERVATION: u128 = TC;

pub(super) async fn execute_root_capability(
    ctx: &RootContext,
    pending: &ReplayPending,
    capability: RootCapability,
    authorized_cycles: Option<AuthorizedCyclesGrant>,
) -> Result<Response, InternalError> {
    let descriptor = capability.descriptor();
    let capability_name = descriptor.name;

    let result = match capability {
        RootCapability::AcknowledgePlacementReceipt(_) => {
            unreachable!("receipt acknowledgement bypasses replay execution")
        }
        RootCapability::AllocatePlacementChild(req) | RootCapability::ProvisionCanister(req) => {
            execute_provision(ctx, pending, &req, descriptor.command_kind).await
        }
        RootCapability::UpgradeCanister(req) => execute_upgrade(ctx, pending, &req).await,
        RootCapability::RecycleCanister(req) => execute_recycle(pending, &req).await,
        RootCapability::RequestCycles(req) => {
            let response = if let Some(grant) = authorized_cycles {
                nonroot_cycles::execute_authorized_request_cycles(ctx, pending, grant).await
            } else if ctx.is_root_env {
                nonroot_cycles::execute_root_request_cycles(ctx, pending, &req).await
            } else {
                nonroot_cycles::execute_request_cycles(ctx, pending, &req).await
            }?;
            Ok(Response::Cycles(response))
        }
    };

    if let Err(err) = &result {
        log!(
            Topic::Rpc,
            Warn,
            "execute_root_capability failed (capability={capability_name}, caller={}, subnet={}, now={}): {err}",
            ctx.caller,
            ctx.subnet_id,
            ctx.now
        );
    }

    result
}

pub(super) fn execute_placement_receipt_acknowledgement(
    ctx: &RootContext,
    req: &AcknowledgePlacementReceiptRequest,
) -> Result<Response, InternalError> {
    let operation_id = OperationId::from_bytes(req.operation_id);
    match acknowledge_root_placement_receipt(operation_id, ctx.caller)
        .map_err(replay::map_replay_store_error)?
    {
        PlacementReceiptAcknowledgementDecision::Acknowledged
        | PlacementReceiptAcknowledgementDecision::AlreadyAbsent => {}
        PlacementReceiptAcknowledgementDecision::ActorMismatch => {
            return Err(InternalError::public(Error::forbidden(format!(
                "placement receipt {operation_id} is not owned by caller",
            ))));
        }
        PlacementReceiptAcknowledgementDecision::NotCommitted => {
            return Err(InternalError::public(Error::conflict(format!(
                "placement receipt {operation_id} is not committed",
            ))));
        }
        PlacementReceiptAcknowledgementDecision::NotPlacementEffect => {
            return Err(InternalError::public(Error::conflict(format!(
                "placement receipt {operation_id} does not contain a placement-child effect",
            ))));
        }
    }

    let response = Response::AcknowledgePlacementReceipt(AcknowledgePlacementReceiptResponse {});
    Ok(response)
}

async fn execute_provision(
    ctx: &RootContext,
    pending: &ReplayPending,
    req: &CreateCanisterRequest,
    command_kind: &'static str,
) -> Result<Response, InternalError> {
    let parent_pid = resolve_provision_parent(ctx, req)?;
    preflight_provision_parent_registered(parent_pid)?;
    let reservation_cycles = root_provision_cycle_reservation_cycles(req)?;
    let cost_permit = reserve_root_provision_cost_guard(ctx, reservation_cycles, command_kind)?;
    if let Err(err) = mark_root_provision_external_effect(
        pending,
        ctx,
        req,
        parent_pid,
        &cost_permit,
        command_kind,
    ) {
        return Err(CostGuardWorkflow::recover_after_failure(
            &cost_permit,
            IcOps::now_secs(),
            err,
        ));
    }

    let event = CanisterLifecycleEvent::Create {
        deployment_permit: &cost_permit,
        role: req.canister_role.clone(),
        parent: parent_pid,
        extra_arg: req.extra_arg.clone(),
    };

    let lifecycle_result = match CanisterLifecycleWorkflow::apply(event).await {
        Ok(result) => result,
        Err(err) => {
            let err =
                CostGuardWorkflow::recover_after_failure(&cost_permit, IcOps::now_secs(), err);
            return Err(preserve_root_provision_recovery_required(
                pending,
                ctx,
                req,
                parent_pid,
                err,
                command_kind,
            ));
        }
    };
    let Some(new_canister_pid) = lifecycle_result.new_canister_pid else {
        let err: InternalError = RpcWorkflowError::MissingNewCanisterPid.into();
        let err = CostGuardWorkflow::recover_after_failure(&cost_permit, IcOps::now_secs(), err);
        return Err(preserve_root_provision_recovery_required(
            pending,
            ctx,
            req,
            parent_pid,
            err,
            command_kind,
        ));
    };

    let response = Response::CreateCanister(CreateCanisterResponse { new_canister_pid });
    if let Err(err) = replay::stage_response(pending, &response) {
        let mut err = err;
        let reason = match CostGuardWorkflow::complete(&cost_permit, IcOps::now_secs()) {
            Ok(()) => RecoveryReason::ResponseCommitFailed,
            Err(settlement_err) => {
                err = err.with_diagnostic_context(format!(
                    "root provision cost settlement also failed: {settlement_err}"
                ));
                RecoveryReason::CostSettlementFailed
            }
        };
        if let Err(recovery_err) = replay::mark_recovery_required(pending, reason) {
            err = err.with_diagnostic_context(format!(
                "root provision replay recovery marker failed: {recovery_err}"
            ));
        }
        return Err(err);
    }
    if let Err(err) = CostGuardWorkflow::complete(&cost_permit, IcOps::now_secs()) {
        if let Err(recovery_err) =
            replay::mark_recovery_required(pending, RecoveryReason::CostSettlementFailed)
        {
            return Err(err.with_diagnostic_context(format!(
                "root provision replay recovery marker failed: {recovery_err}"
            )));
        }
        return Err(err);
    }

    Ok(response)
}

fn resolve_provision_parent(
    ctx: &RootContext,
    req: &CreateCanisterRequest,
) -> Result<Principal, InternalError> {
    match &req.parent {
        CreateCanisterParent::Canister(pid) => Ok(*pid),
        CreateCanisterParent::Root => Ok(IcOps::canister_self()),
        CreateCanisterParent::ThisCanister => Ok(ctx.caller),
        CreateCanisterParent::Parent => SubnetRegistryOps::get_parent(ctx.caller)
            .ok_or_else(|| RpcWorkflowError::ParentNotFound(ctx.caller).into()),
        CreateCanisterParent::Index(role) => SubnetIndexOps::get(role)
            .ok_or_else(|| RpcWorkflowError::CanisterRoleNotFound(role.clone()).into()),
    }
}

fn preflight_provision_parent_registered(parent_pid: Principal) -> Result<(), InternalError> {
    if SubnetRegistryOps::is_registered(parent_pid) {
        Ok(())
    } else {
        Err(TopologyPolicyError::ParentNotFound(parent_pid).into())
    }
}

fn root_provision_cycle_reservation_cycles(
    req: &CreateCanisterRequest,
) -> Result<u128, InternalError> {
    Ok(ConfigOps::current_subnet_canister(&req.canister_role)?
        .initial_cycles
        .to_u128())
}

fn reserve_root_provision_cost_guard(
    ctx: &RootContext,
    reservation_cycles: u128,
    command_kind: &'static str,
) -> Result<CostGuardPermit, InternalError> {
    CostGuardWorkflow::reserve(root_provision_cost_guard_request(
        ctx,
        reservation_cycles,
        IcOps::canister_cycle_balance().to_u128(),
        command_kind,
    ))
    .map_err(map_cost_guard_reserve_error)
}

pub(super) fn root_provision_cost_guard_request(
    ctx: &RootContext,
    reservation_cycles: u128,
    current_cycle_balance: u128,
    command_kind: &'static str,
) -> CostGuardRequest {
    CostGuardRequest {
        cost_class: CostClass::ManagementDeployment,
        command_kind: root_provision_command_kind(command_kind),
        quota_subject: ctx.caller,
        payer: ctx.self_pid,
        now_secs: ctx.now,
        quota_window_secs: ROOT_PROVISION_DEPLOYMENT_QUOTA_WINDOW_SECONDS,
        max_operations_per_window: MAX_ROOT_PROVISION_DEPLOYMENT_OPERATIONS_PER_WINDOW,
        current_cycle_balance,
        cycle_reservation_cycles: reservation_cycles,
        min_cycles_after_reservation: MIN_ROOT_PROVISION_CYCLES_AFTER_RESERVATION,
    }
}

fn root_provision_command_kind(command_kind: &'static str) -> CommandKind {
    CommandKind::new(command_kind).expect("root provision command kind is a valid static label")
}

pub(super) fn mark_root_provision_external_effect(
    pending: &ReplayPending,
    ctx: &RootContext,
    req: &CreateCanisterRequest,
    parent_pid: Principal,
    cost_permit: &CostGuardPermit,
    command_kind: &'static str,
) -> Result<(), InternalError> {
    replay::mark_costed_external_effect_in_flight(
        pending,
        ExternalEffectDescriptor::ManagementCreateCanister {
            command_kind: root_provision_command_kind(command_kind),
        },
        cost_permit,
    )?;
    log!(
        Topic::Rpc,
        Info,
        "root provision replay effect marked effect=provision_canister command_kind={} caller={} role={} parent={}",
        command_kind,
        ctx.caller,
        req.canister_role,
        parent_pid
    );
    Ok(())
}

fn preserve_root_provision_recovery_required(
    pending: &ReplayPending,
    ctx: &RootContext,
    req: &CreateCanisterRequest,
    parent_pid: Principal,
    err: InternalError,
    command_kind: &'static str,
) -> InternalError {
    let (error_class, error_origin) = err.log_fields();
    let err = mark_recovery_required_after_failure(
        &pending.receipt_token,
        RecoveryReason::ExternalEffectStatusUnknown,
        secs_to_ns(IcOps::now_secs()),
        err,
        "root provision replay recovery marker failed",
    );
    log!(
        Topic::Rpc,
        Error,
        "root provision replay recovery required effect=provision_canister command_kind={} caller={} role={} parent={} error_class={} error_origin={}",
        command_kind,
        ctx.caller,
        req.canister_role,
        parent_pid,
        error_class,
        error_origin
    );
    err
}

async fn execute_upgrade(
    ctx: &RootContext,
    pending: &ReplayPending,
    req: &UpgradeCanisterRequest,
) -> Result<Response, InternalError> {
    let response = Response::UpgradeCanister(UpgradeCanisterResponse {});
    replay::stage_response(pending, &response)?;
    let event = CanisterLifecycleEvent::Upgrade {
        cost_context: CanisterUpgradeCostContext {
            quota_subject: ctx.caller,
            payer: ctx.self_pid,
            now_secs: ctx.now,
        },
        pid: req.canister_pid,
        replay_pending: pending,
    };

    CanisterLifecycleWorkflow::apply(event).await?;

    Ok(response)
}

async fn execute_recycle(
    pending: &ReplayPending,
    req: &RecycleCanisterRequest,
) -> Result<Response, InternalError> {
    let response = Response::RecycleCanister(RecycleCanisterResponse {});
    replay::stage_response(pending, &response)?;
    PoolWorkflow::pool_recycle_canister(req.canister_pid).await?;

    Ok(response)
}
