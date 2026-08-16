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
    cdk::types::Principal,
    dto::rpc::{
        AcknowledgePlacementReceiptRequest, CreateCanisterRequest, CreateCanisterResponse,
        RecycleCanisterRequest, Response,
    },
    log,
    log::Topic,
    model::replay::{CommandKind, ExternalEffectDescriptor, OperationId, RecoveryReason},
    ops::{
        ic::IcOps,
        replay::{
            acknowledge_root_placement_receipt,
            guard::{ReplayPending, secs_to_ns},
            receipt::PlacementReceiptAcknowledgementDecision,
        },
    },
    workflow::{
        replay::mark_recovery_required_after_failure,
        rpc::{
            RootCapabilityAuthority, RootCapabilityLifecycleExecutor,
            RootComponentChildProvisionRequest, RootComponentChildRecycleOutcome,
            RootComponentChildRecycleRequest,
        },
    },
};

pub(super) async fn execute_root_capability(
    ctx: &RootContext,
    pending: &ReplayPending,
    capability: RootCapability,
    authorized_cycles: Option<AuthorizedCyclesGrant>,
    authority: &RootCapabilityAuthority,
    lifecycle: &dyn RootCapabilityLifecycleExecutor,
) -> Result<Response, InternalError> {
    let descriptor = capability.descriptor();
    let capability_name = descriptor.name;

    let result = match capability {
        RootCapability::AcknowledgePlacementReceipt(_) => {
            unreachable!("receipt acknowledgement bypasses replay execution")
        }
        RootCapability::AllocatePlacementChild(req) | RootCapability::ProvisionCanister(req) => {
            execute_provision(
                ctx,
                pending,
                &req,
                descriptor.command_kind,
                authority,
                lifecycle,
            )
            .await
        }
        RootCapability::RecycleCanister(req) => {
            execute_recycle(ctx, pending, &req, authority, lifecycle).await
        }
        RootCapability::RequestCycles(req) => {
            let response = if let Some(grant) = authorized_cycles {
                nonroot_cycles::execute_authorized_request_cycles(ctx, pending, grant).await
            } else if ctx.is_root_env {
                nonroot_cycles::execute_root_request_cycles(ctx, pending, &req, authority).await
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
            return Err(InternalError::public(
                crate::diagnostics::codes::AUTHORITY_UNAUTHORIZED,
            ));
        }
        PlacementReceiptAcknowledgementDecision::NotCommitted => {
            return Err(InternalError::public(
                crate::diagnostics::codes::STATE_CONFLICT,
            ));
        }
        PlacementReceiptAcknowledgementDecision::NotPlacementEffect => {
            return Err(InternalError::public(
                crate::diagnostics::codes::STATE_CONFLICT,
            ));
        }
    }

    let response = Response::AcknowledgePlacementReceipt;
    Ok(response)
}

async fn execute_provision(
    ctx: &RootContext,
    pending: &ReplayPending,
    req: &CreateCanisterRequest,
    command_kind: &'static str,
    authority: &RootCapabilityAuthority,
    lifecycle: &dyn RootCapabilityLifecycleExecutor,
) -> Result<Response, InternalError> {
    let parent_pid = resolve_provision_parent(authority)?;
    mark_root_provision_external_effect(pending, ctx, req, parent_pid, command_kind)?;
    let provision = component_child_provision_request(pending, req, authority)?;
    let new_canister_pid = match lifecycle.provision_component_child(provision).await {
        Ok(pid) => pid,
        Err(err) => {
            return Err(preserve_root_provision_recovery_required(
                pending,
                ctx,
                req,
                parent_pid,
                err,
                command_kind,
                RecoveryReason::ComponentChildLifecycleInterrupted,
            ));
        }
    };

    let response = Response::CreateCanister(CreateCanisterResponse { new_canister_pid });
    if let Err(err) = replay::stage_response(pending, &response) {
        return Err(preserve_root_provision_recovery_required(
            pending,
            ctx,
            req,
            parent_pid,
            err,
            command_kind,
            RecoveryReason::ResponseCommitFailed,
        ));
    }

    Ok(response)
}

fn component_child_provision_request(
    pending: &ReplayPending,
    req: &CreateCanisterRequest,
    authority: &RootCapabilityAuthority,
) -> Result<RootComponentChildProvisionRequest, InternalError> {
    let component = authority
        .caller_component()
        .ok_or_else(|| InternalError::public(crate::diagnostics::codes::AUTHORITY_UNAUTHORIZED))?;
    let expected_registry = authority
        .caller_registry()
        .cloned()
        .ok_or_else(|| InternalError::invariant())?;
    Ok(RootComponentChildProvisionRequest {
        operation_id: pending.receipt_token.receipt().operation_id.into_bytes(),
        component,
        expected_registry,
        child_role: req.canister_role.clone(),
        application_init_args: req.extra_arg.clone(),
    })
}

fn resolve_provision_parent(
    authority: &RootCapabilityAuthority,
) -> Result<crate::cdk::types::Principal, InternalError> {
    authority
        .provision_parent_canister_id()
        .ok_or_else(|| InternalError::invariant())
}

fn root_provision_command_kind(command_kind: &'static str) -> CommandKind {
    CommandKind::new(command_kind).expect("root provision command kind is a valid static label")
}

pub(super) fn mark_root_provision_external_effect(
    pending: &ReplayPending,
    ctx: &RootContext,
    req: &CreateCanisterRequest,
    parent_pid: Principal,
    command_kind: &'static str,
) -> Result<(), InternalError> {
    replay::mark_external_effect_in_flight(
        pending,
        ExternalEffectDescriptor::RootCanisterProvision {
            command_kind: root_provision_command_kind(command_kind),
        },
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
    reason: RecoveryReason,
) -> InternalError {
    let diagnostic = err.code();
    let err = mark_recovery_required_after_failure(
        &pending.receipt_token,
        reason,
        secs_to_ns(IcOps::now_secs()),
        err,
        "root provision replay recovery marker failed",
    );
    log!(
        Topic::Rpc,
        Error,
        "root provision replay recovery required effect=provision_canister command_kind={} caller={} role={} parent={} diagnostic={}",
        command_kind,
        ctx.caller,
        req.canister_role,
        parent_pid,
        diagnostic
    );
    err
}

async fn execute_recycle(
    ctx: &RootContext,
    pending: &ReplayPending,
    req: &RecycleCanisterRequest,
    authority: &RootCapabilityAuthority,
    lifecycle: &dyn RootCapabilityLifecycleExecutor,
) -> Result<Response, InternalError> {
    let recycle = component_child_recycle_request(pending, req, authority)?;
    replay::mark_external_effect_in_flight(
        pending,
        ExternalEffectDescriptor::ManagementCall {
            canister: req.canister_pid,
            method: "component_subtree_removal".to_string(),
        },
    )?;
    match lifecycle.recycle_component_child(recycle).await {
        Ok(RootComponentChildRecycleOutcome::Completed) => {}
        Ok(RootComponentChildRecycleOutcome::InProgress) => {
            let error = InternalError::public(crate::diagnostics::codes::STATE_UNAVAILABLE);
            return Err(preserve_root_recycle_recovery_required(
                pending,
                ctx,
                req.canister_pid,
                error,
                RecoveryReason::ComponentChildLifecycleInterrupted,
            ));
        }
        Err(error) => {
            return Err(preserve_root_recycle_recovery_required(
                pending,
                ctx,
                req.canister_pid,
                error,
                RecoveryReason::ComponentChildLifecycleInterrupted,
            ));
        }
    }

    let response = Response::RecycleCanister;
    if let Err(error) = replay::stage_response(pending, &response) {
        return Err(preserve_root_recycle_recovery_required(
            pending,
            ctx,
            req.canister_pid,
            error,
            RecoveryReason::ResponseCommitFailed,
        ));
    }

    Ok(response)
}

fn component_child_recycle_request(
    pending: &ReplayPending,
    req: &RecycleCanisterRequest,
    authority: &RootCapabilityAuthority,
) -> Result<RootComponentChildRecycleRequest, InternalError> {
    let component = authority
        .caller_component()
        .ok_or_else(|| InternalError::public(crate::diagnostics::codes::AUTHORITY_UNAUTHORIZED))?;
    let expected_registry = authority
        .caller_registry()
        .cloned()
        .ok_or_else(|| InternalError::invariant())?;
    Ok(RootComponentChildRecycleRequest {
        operation_id: pending.receipt_token.receipt().operation_id.into_bytes(),
        component,
        expected_registry,
        target_canister_id: req.canister_pid,
    })
}

fn preserve_root_recycle_recovery_required(
    pending: &ReplayPending,
    ctx: &RootContext,
    target: Principal,
    error: InternalError,
    reason: RecoveryReason,
) -> InternalError {
    let diagnostic = error.code();
    let error = mark_recovery_required_after_failure(
        &pending.receipt_token,
        reason,
        secs_to_ns(IcOps::now_secs()),
        error,
        "root recycle replay recovery marker failed",
    );
    log!(
        Topic::Rpc,
        Error,
        "root recycle replay recovery required effect=component_subtree_removal caller={} target={} diagnostic={}",
        ctx.caller,
        target,
        diagnostic
    );
    error
}
