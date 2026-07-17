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
    dto::rpc::{
        CreateCanisterParent, CreateCanisterRequest, CreateCanisterResponse,
        RecycleCanisterRequest, RecycleCanisterResponse, Response, UpgradeCanisterRequest,
        UpgradeCanisterResponse,
    },
    log,
    log::Topic,
    model::replay::{CommandKind, ExternalEffectDescriptor, RecoveryReason},
    ops::{
        config::ConfigOps,
        cost_guard::{CostGuardOps, CostGuardPermit, CostGuardRequest},
        ic::IcOps,
        replay::guard::ReplayPending,
        storage::{index::subnet::SubnetIndexOps, registry::subnet::SubnetRegistryOps},
    },
    replay_policy::CostClass,
    workflow::{
        canister_lifecycle::{
            CanisterLifecycleEvent, CanisterLifecycleWorkflow, CanisterUpgradeCostContext,
        },
        cost_guard::map_cost_guard_reserve_error,
        pool::PoolWorkflow,
        rpc::RpcWorkflowError,
    },
};

const ROOT_PROVISION_COMMAND_KIND: &str = "root.provision.v1";
const ROOT_PROVISION_DEPLOYMENT_QUOTA_WINDOW_SECONDS: u64 = 60;
const MAX_ROOT_PROVISION_DEPLOYMENT_OPERATIONS_PER_WINDOW: u64 = 10;
const MIN_ROOT_PROVISION_CYCLES_AFTER_RESERVATION: u128 = TC;

pub(super) async fn execute_root_capability(
    ctx: &RootContext,
    pending: &ReplayPending,
    capability: RootCapability,
    authorized_cycles: Option<AuthorizedCyclesGrant>,
) -> Result<Response, InternalError> {
    let capability_name = capability.capability_name();

    let result = match capability {
        RootCapability::ProvisionCanister(req) => execute_provision(ctx, pending, &req).await,
        RootCapability::UpgradeCanister(req) => execute_upgrade(ctx, pending, &req).await,
        RootCapability::RecycleCanister(req) => execute_recycle(&req).await,
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

async fn execute_provision(
    ctx: &RootContext,
    pending: &ReplayPending,
    req: &CreateCanisterRequest,
) -> Result<Response, InternalError> {
    let parent_pid = resolve_provision_parent(ctx, req)?;
    preflight_provision_parent_registered(parent_pid)?;
    let reservation_cycles = root_provision_cycle_reservation_cycles(req)?;
    let cost_permit = reserve_root_provision_cost_guard(ctx, reservation_cycles)?;
    if let Err(err) =
        mark_root_provision_external_effect(pending, ctx, req, parent_pid, &cost_permit)
    {
        return Err(CostGuardOps::recover_after_failure(
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
            let err = CostGuardOps::recover_after_failure(&cost_permit, IcOps::now_secs(), err);
            mark_root_provision_recovery_required(pending, ctx, req, parent_pid, &err)?;
            return Err(err);
        }
    };
    let Some(new_canister_pid) = lifecycle_result.new_canister_pid else {
        let err: InternalError = RpcWorkflowError::MissingNewCanisterPid.into();
        let err = CostGuardOps::recover_after_failure(&cost_permit, IcOps::now_secs(), err);
        mark_root_provision_recovery_required(pending, ctx, req, parent_pid, &err)?;
        return Err(err);
    };

    let response = Response::CreateCanister(CreateCanisterResponse { new_canister_pid });
    if let Err(err) = replay::stage_response(pending, &response) {
        let err = CostGuardOps::recover_after_failure(&cost_permit, IcOps::now_secs(), err);
        replay::mark_recovery_required(pending, RecoveryReason::ResponseCommitFailed)?;
        return Err(err);
    }
    if let Err(err) = CostGuardOps::complete(&cost_permit, IcOps::now_secs()) {
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
) -> Result<CostGuardPermit, InternalError> {
    CostGuardOps::reserve(root_provision_cost_guard_request(
        ctx,
        reservation_cycles,
        IcOps::canister_cycle_balance().to_u128(),
    ))
    .map_err(map_cost_guard_reserve_error)
}

pub(super) fn root_provision_cost_guard_request(
    ctx: &RootContext,
    reservation_cycles: u128,
    current_cycle_balance: u128,
) -> CostGuardRequest {
    CostGuardRequest {
        cost_class: CostClass::ManagementDeployment,
        command_kind: root_provision_command_kind(),
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

fn root_provision_command_kind() -> CommandKind {
    CommandKind::new(ROOT_PROVISION_COMMAND_KIND)
        .expect("root provision command kind is a valid static label")
}

pub(super) fn mark_root_provision_external_effect(
    pending: &ReplayPending,
    ctx: &RootContext,
    req: &CreateCanisterRequest,
    parent_pid: Principal,
    cost_permit: &CostGuardPermit,
) -> Result<(), InternalError> {
    replay::mark_costed_external_effect_in_flight(
        pending,
        ExternalEffectDescriptor::ManagementCreateCanister {
            command_kind: root_provision_command_kind(),
        },
        cost_permit,
    )?;
    log!(
        Topic::Rpc,
        Info,
        "root provision replay effect marked effect=provision_canister command_kind={} caller={} role={} parent={}",
        ROOT_PROVISION_COMMAND_KIND,
        ctx.caller,
        req.canister_role,
        parent_pid
    );
    Ok(())
}

fn mark_root_provision_recovery_required(
    pending: &ReplayPending,
    ctx: &RootContext,
    req: &CreateCanisterRequest,
    parent_pid: Principal,
    err: &InternalError,
) -> Result<(), InternalError> {
    let (error_class, error_origin) = err.log_fields();
    replay::mark_recovery_required(pending, RecoveryReason::ExternalEffectStatusUnknown)?;
    log!(
        Topic::Rpc,
        Error,
        "root provision replay recovery required effect=provision_canister command_kind={} caller={} role={} parent={} error_class={} error_origin={}",
        ROOT_PROVISION_COMMAND_KIND,
        ctx.caller,
        req.canister_role,
        parent_pid,
        error_class,
        error_origin
    );
    Ok(())
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

async fn execute_recycle(req: &RecycleCanisterRequest) -> Result<Response, InternalError> {
    PoolWorkflow::pool_recycle_canister(req.canister_pid).await?;

    Ok(Response::RecycleCanister(RecycleCanisterResponse {}))
}
