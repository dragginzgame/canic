//! Module: workflow::rpc::request::handler::authorize
//!
//! Responsibility: authorize root-capability requests before execution.
//! Does not own: replay reservation, capability execution, or request mapping.
//! Boundary: reads workflow context and storage ops after endpoints authenticate input.

use super::{RootCapability, RootContext, nonroot_cycles};
use crate::{
    InternalError,
    dto::rpc::{CreateCanisterParent, CreateCanisterRequest, RecycleCanisterRequest},
    log,
    log::Topic,
    ops::{
        runtime::env::EnvOps,
        runtime::metrics::root_capability::{RootCapabilityMetricOutcome, RootCapabilityMetrics},
    },
    workflow::rpc::{RootCapabilityAuthority, RootCapabilityMemberLifecycle, RpcWorkflowError},
};

/// authorize
///
/// Apply capability-specific authorization and record root-capability metrics.
pub(super) fn authorize(
    ctx: &RootContext,
    capability: &RootCapability,
    authority: &RootCapabilityAuthority,
) -> Result<(), InternalError> {
    require_exact_caller_authority(ctx, authority)?;

    // RequestCycles already owns its authorization metrics/logging in the
    // shared cycles helper so root and non-root paths stay aligned.
    if let RootCapability::RequestCycles(req) = capability {
        return if ctx.is_root_env {
            nonroot_cycles::authorize_root_request_cycles(ctx, req, authority)
        } else {
            nonroot_cycles::authorize_request_cycles(ctx, req)
        };
    }

    let descriptor = capability.descriptor();
    if authority.caller_member_lifecycle() == Some(RootCapabilityMemberLifecycle::Prepared)
        && !matches!(capability, RootCapability::AllocatePlacementChild(_))
    {
        return Err(InternalError::public(
            crate::diagnostics::codes::AUTHORITY_UNAUTHORIZED,
        ));
    }
    let decision = match capability {
        RootCapability::AcknowledgePlacementReceipt(_) => {
            authorize_placement_receipt_acknowledgement(ctx)
        }
        RootCapability::AllocatePlacementChild(req) | RootCapability::ProvisionCanister(req) => {
            authorize_provision(ctx, req, authority)
        }
        RootCapability::RecycleCanister(req) => {
            authorize_root_only(ctx).and_then(|()| authorize_recycle(ctx, req, authority))
        }
        RootCapability::RequestCycles(_) => unreachable!("handled before generic authorization"),
    };

    match &decision {
        Ok(()) => {
            RootCapabilityMetrics::record_authorization(
                descriptor.key,
                RootCapabilityMetricOutcome::Accepted,
            );
            log!(
                Topic::Rpc,
                Info,
                "capability authorized (capability={}, caller={}, subnet={}, now={})",
                descriptor.name,
                ctx.caller,
                ctx.subnet_id,
                ctx.now
            );
        }
        Err(err) => {
            RootCapabilityMetrics::record_authorization(
                descriptor.key,
                RootCapabilityMetricOutcome::Denied,
            );
            log!(
                Topic::Rpc,
                Warn,
                "capability denied (capability={}, caller={}, subnet={}, now={}): {err}",
                descriptor.name,
                ctx.caller,
                ctx.subnet_id,
                ctx.now
            );
        }
    }

    decision
}

fn authorize_placement_receipt_acknowledgement(ctx: &RootContext) -> Result<(), InternalError> {
    if ctx.caller == ctx.self_pid {
        return Ok(());
    }

    if !ctx.is_root_env {
        return EnvOps::require_root();
    }

    Ok(())
}

fn authorize_provision(
    ctx: &RootContext,
    req: &CreateCanisterRequest,
    authority: &RootCapabilityAuthority,
) -> Result<(), InternalError> {
    if !ctx.is_root_env {
        return EnvOps::require_root();
    }

    if !matches!(&req.parent, CreateCanisterParent::ThisCanister) {
        return Err(InternalError::public(
            crate::diagnostics::codes::AUTHORITY_UNAUTHORIZED,
        ));
    }

    if authority.provision_parent_canister_id() != Some(ctx.caller) {
        return Err(InternalError::public(
            crate::diagnostics::codes::AUTHORITY_UNAUTHORIZED,
        ));
    }

    Ok(())
}

fn authorize_root_only(ctx: &RootContext) -> Result<(), InternalError> {
    if ctx.is_root_env {
        Ok(())
    } else {
        EnvOps::require_root()
    }
}

fn authorize_recycle(
    ctx: &RootContext,
    req: &RecycleCanisterRequest,
    authority: &RootCapabilityAuthority,
) -> Result<(), InternalError> {
    if authority.caller_component().is_none() {
        return Err(InternalError::public(
            crate::diagnostics::codes::AUTHORITY_UNAUTHORIZED,
        ));
    }
    require_exact_target(req.canister_pid, authority)?;
    if authority.target_parent_canister_id() != Some(ctx.caller) {
        return Err(RpcWorkflowError::NotChildOfCaller(req.canister_pid, ctx.caller).into());
    }

    Ok(())
}

fn require_exact_caller_authority(
    ctx: &RootContext,
    authority: &RootCapabilityAuthority,
) -> Result<(), InternalError> {
    if authority.caller_canister_id() != ctx.caller {
        return Err(InternalError::public(
            crate::diagnostics::codes::AUTHORITY_UNAUTHORIZED,
        ));
    }
    if ctx.caller == ctx.self_pid && !authority.caller_is_fleet_subnet_root() {
        return Err(InternalError::public(
            crate::diagnostics::codes::AUTHORITY_UNAUTHORIZED,
        ));
    }
    if ctx.caller != ctx.self_pid && authority.caller_is_fleet_subnet_root() {
        return Err(InternalError::public(
            crate::diagnostics::codes::AUTHORITY_UNAUTHORIZED,
        ));
    }
    Ok(())
}

fn require_exact_target(
    target: crate::cdk::types::Principal,
    authority: &RootCapabilityAuthority,
) -> Result<(), InternalError> {
    if authority.target_canister_id() != Some(target) {
        return Err(RpcWorkflowError::ChildNotFound(target).into());
    }
    Ok(())
}
