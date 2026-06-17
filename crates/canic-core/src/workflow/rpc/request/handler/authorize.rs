//! Module: workflow::rpc::request::handler::authorize
//!
//! Responsibility: authorize root-capability requests before execution.
//! Does not own: replay reservation, capability execution, or request mapping.
//! Boundary: reads workflow context and storage ops after endpoints authenticate input.

use super::{RootCapability, RootContext, nonroot_cycles};
use crate::{
    InternalError,
    dto::{
        error::Error,
        rpc::{
            CreateCanisterParent, CreateCanisterRequest, RecycleCanisterRequest,
            UpgradeCanisterRequest,
        },
    },
    log,
    log::Topic,
    ops::{
        runtime::env::EnvOps,
        runtime::metrics::root_capability::{RootCapabilityMetricOutcome, RootCapabilityMetrics},
        storage::registry::subnet::SubnetRegistryOps,
    },
    workflow::rpc::RpcWorkflowError,
};

/// authorize
///
/// Apply capability-specific authorization and record root-capability metrics.
pub(super) fn authorize(
    ctx: &RootContext,
    capability: &RootCapability,
) -> Result<(), InternalError> {
    // RequestCycles already owns its authorization metrics/logging in the
    // shared cycles helper so root and non-root paths stay aligned.
    if let RootCapability::RequestCycles(req) = capability {
        return if ctx.is_root_env {
            nonroot_cycles::authorize_root_request_cycles(ctx, req)
        } else {
            nonroot_cycles::authorize_request_cycles(ctx, req)
        };
    }

    let descriptor = capability.descriptor();
    let decision = match capability {
        RootCapability::Provision(req) => authorize_provision(ctx, req),
        RootCapability::Upgrade(req) => {
            authorize_root_only(ctx).and_then(|()| authorize_upgrade(ctx, req))
        }
        RootCapability::RecycleCanister(req) => {
            authorize_root_only(ctx).and_then(|()| authorize_recycle(ctx, req))
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

fn authorize_provision(
    ctx: &RootContext,
    req: &CreateCanisterRequest,
) -> Result<(), InternalError> {
    if ctx.caller == ctx.self_pid {
        return Ok(());
    }

    if !ctx.is_root_env {
        return EnvOps::require_root();
    }

    if !matches!(&req.parent, CreateCanisterParent::ThisCanister) {
        return Err(InternalError::public(Error::forbidden(
            "non-root structural provision requires parent=ThisCanister",
        )));
    }

    if !SubnetRegistryOps::is_registered(ctx.caller) {
        return Err(InternalError::public(Error::forbidden(
            "non-root structural provision requires caller to be registered in subnet registry",
        )));
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

fn authorize_upgrade(ctx: &RootContext, req: &UpgradeCanisterRequest) -> Result<(), InternalError> {
    let (_, parent_pid) = SubnetRegistryOps::role_parent(req.canister_pid)
        .ok_or(RpcWorkflowError::ChildNotFound(req.canister_pid))?;

    if parent_pid != Some(ctx.caller) {
        return Err(RpcWorkflowError::NotChildOfCaller(req.canister_pid, ctx.caller).into());
    }

    Ok(())
}

fn authorize_recycle(ctx: &RootContext, req: &RecycleCanisterRequest) -> Result<(), InternalError> {
    let Some((_, parent_pid)) = SubnetRegistryOps::role_parent(req.canister_pid) else {
        return Ok(());
    };

    if ctx.caller != ctx.self_pid && parent_pid != Some(ctx.caller) {
        return Err(RpcWorkflowError::NotChildOfCaller(req.canister_pid, ctx.caller).into());
    }

    Ok(())
}
