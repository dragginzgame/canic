use super::{
    DEFAULT_MAX_ROLE_ATTESTATION_TTL_SECONDS, RootCapability, RootContext, nonroot_cycles,
};
use crate::{
    InternalError,
    cdk::types::Principal,
    dto::auth::{InternalInvocationProofRequest, RoleAttestationRequest},
    dto::rpc::{RecycleCanisterRequest, UpgradeCanisterRequest},
    log,
    log::Topic,
    ops::{
        config::ConfigOps,
        runtime::env::EnvOps,
        runtime::metrics::root_capability::{RootCapabilityMetricOutcome, RootCapabilityMetrics},
        storage::{index::app::AppIndexOps, registry::subnet::SubnetRegistryOps},
    },
    workflow::rpc::RpcWorkflowError,
};

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
        RootCapability::Provision(_req) => authorize_root_only(ctx),
        RootCapability::Upgrade(req) => {
            authorize_root_only(ctx).and_then(|()| authorize_upgrade(ctx, req))
        }
        RootCapability::RecycleCanister(req) => {
            authorize_root_only(ctx).and_then(|()| authorize_recycle(ctx, req))
        }
        RootCapability::RequestCycles(_) => unreachable!("handled before generic authorization"),
        RootCapability::IssueRoleAttestation(req) => {
            authorize_root_only(ctx).and_then(|()| authorize_issue_role_attestation(ctx, req))
        }
        RootCapability::IssueInternalInvocationProof(req) => authorize_root_only(ctx)
            .and_then(|()| authorize_issue_internal_invocation_proof(ctx, req)),
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

fn authorize_root_only(ctx: &RootContext) -> Result<(), InternalError> {
    if ctx.is_root_env {
        Ok(())
    } else {
        EnvOps::require_root()
    }
}

fn authorize_upgrade(ctx: &RootContext, req: &UpgradeCanisterRequest) -> Result<(), InternalError> {
    let registry_entry = SubnetRegistryOps::get(req.canister_pid)
        .ok_or(RpcWorkflowError::ChildNotFound(req.canister_pid))?;

    if registry_entry.parent_pid != Some(ctx.caller) {
        return Err(RpcWorkflowError::NotChildOfCaller(req.canister_pid, ctx.caller).into());
    }

    Ok(())
}

fn authorize_recycle(ctx: &RootContext, req: &RecycleCanisterRequest) -> Result<(), InternalError> {
    let Some(registry_entry) = SubnetRegistryOps::get(req.canister_pid) else {
        return Ok(());
    };

    if ctx.caller != ctx.self_pid && registry_entry.parent_pid != Some(ctx.caller) {
        return Err(RpcWorkflowError::NotChildOfCaller(req.canister_pid, ctx.caller).into());
    }

    Ok(())
}

fn authorize_issue_role_attestation(
    ctx: &RootContext,
    req: &RoleAttestationRequest,
) -> Result<(), InternalError> {
    if req.subject != ctx.caller {
        return Err(RpcWorkflowError::RoleAttestationSubjectMismatch {
            caller: ctx.caller,
            subject: req.subject,
        }
        .into());
    }

    let registered = SubnetRegistryOps::get(req.subject).ok_or(
        RpcWorkflowError::RoleAttestationSubjectNotRegistered {
            subject: req.subject,
        },
    )?;

    if registered.role != req.role {
        return Err(RpcWorkflowError::RoleAttestationRoleMismatch {
            subject: req.subject,
            requested: req.role.clone(),
            registered: registered.role,
        }
        .into());
    }

    if let Some(requested_subnet) = req.subnet_id
        && requested_subnet != ctx.subnet_id
    {
        return Err(RpcWorkflowError::RoleAttestationSubnetMismatch {
            subject: req.subject,
            requested: requested_subnet,
            local: ctx.subnet_id,
        }
        .into());
    }

    let max_ttl_secs = max_role_attestation_ttl_seconds();
    if req.ttl_secs == 0 || req.ttl_secs > max_ttl_secs {
        return Err(RpcWorkflowError::RoleAttestationInvalidTtl {
            ttl_secs: req.ttl_secs,
            max_ttl_secs,
        }
        .into());
    }

    Ok(())
}

fn authorize_issue_internal_invocation_proof(
    ctx: &RootContext,
    req: &InternalInvocationProofRequest,
) -> Result<(), InternalError> {
    if req.subject != ctx.caller {
        return Err(RpcWorkflowError::RoleAttestationSubjectMismatch {
            caller: ctx.caller,
            subject: req.subject,
        }
        .into());
    }

    authorize_subject_role(req)?;

    if let Some(requested_subnet) = req.subnet_id
        && requested_subnet != ctx.subnet_id
    {
        return Err(RpcWorkflowError::RoleAttestationSubnetMismatch {
            subject: req.subject,
            requested: requested_subnet,
            local: ctx.subnet_id,
        }
        .into());
    }

    if req.audience_method.trim().is_empty() {
        return Err(RpcWorkflowError::InternalInvocationProofMethodEmpty.into());
    }

    if !is_known_audience(ctx, req.audience) {
        return Err(RpcWorkflowError::InternalInvocationProofAudienceUnknown {
            audience: req.audience,
        }
        .into());
    }

    let max_ttl_secs = max_role_attestation_ttl_seconds();
    if req.ttl_secs == 0 || req.ttl_secs > max_ttl_secs {
        return Err(RpcWorkflowError::RoleAttestationInvalidTtl {
            ttl_secs: req.ttl_secs,
            max_ttl_secs,
        }
        .into());
    }

    Ok(())
}

fn authorize_subject_role(req: &InternalInvocationProofRequest) -> Result<(), InternalError> {
    if AppIndexOps::get(&req.role) == Some(req.subject) {
        return Ok(());
    }

    let registered = SubnetRegistryOps::get(req.subject).ok_or(
        RpcWorkflowError::RoleAttestationSubjectNotRegistered {
            subject: req.subject,
        },
    )?;

    if registered.role != req.role {
        return Err(RpcWorkflowError::RoleAttestationRoleMismatch {
            subject: req.subject,
            requested: req.role.clone(),
            registered: registered.role,
        }
        .into());
    }

    Ok(())
}

fn is_known_audience(ctx: &RootContext, audience: Principal) -> bool {
    audience == ctx.self_pid
        || SubnetRegistryOps::is_registered(audience)
        || AppIndexOps::data()
            .entries
            .iter()
            .any(|(_, pid)| *pid == audience)
}

pub(super) fn max_role_attestation_ttl_seconds() -> u64 {
    ConfigOps::role_attestation_config().map_or(DEFAULT_MAX_ROLE_ATTESTATION_TTL_SECONDS, |cfg| {
        cfg.max_ttl_secs
    })
}
