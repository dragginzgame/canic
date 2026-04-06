use super::{
    DEFAULT_MAX_ROLE_ATTESTATION_TTL_SECONDS, RootCapability, RootContext, nonroot_cycles,
};
use crate::{
    InternalError,
    dto::auth::{DelegationRequest, RoleAttestationRequest},
    dto::rpc::UpgradeCanisterRequest,
    log,
    log::Topic,
    ops::{
        config::ConfigOps,
        runtime::env::EnvOps,
        runtime::metrics::root_capability::{RootCapabilityMetricOutcome, RootCapabilityMetrics},
        storage::registry::subnet::SubnetRegistryOps,
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
        RootCapability::RequestCycles(_) => unreachable!("handled before generic authorization"),
        RootCapability::IssueDelegation(req) => {
            authorize_root_only(ctx).and_then(|()| authorize_issue_delegation(ctx, req))
        }
        RootCapability::IssueRoleAttestation(req) => {
            authorize_root_only(ctx).and_then(|()| authorize_issue_role_attestation(ctx, req))
        }
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

fn authorize_issue_delegation(
    ctx: &RootContext,
    req: &DelegationRequest,
) -> Result<(), InternalError> {
    let cfg = ConfigOps::delegated_tokens_config()?;
    if !cfg.enabled {
        return Err(RpcWorkflowError::DelegatedTokensDisabled.into());
    }

    let root_pid = ctx.self_pid;

    if ctx.caller != req.shard_pid {
        return Err(
            RpcWorkflowError::DelegationCallerShardMismatch(ctx.caller, req.shard_pid).into(),
        );
    }

    if req.ttl_secs == 0 {
        return Err(RpcWorkflowError::DelegationInvalidTtl(req.ttl_secs).into());
    }

    if req.aud.is_empty() {
        return Err(RpcWorkflowError::DelegationAudienceEmpty.into());
    }

    if req.scopes.is_empty() {
        return Err(RpcWorkflowError::DelegationScopesEmpty.into());
    }

    if req.scopes.iter().any(String::is_empty) {
        return Err(RpcWorkflowError::DelegationScopeEmpty.into());
    }

    for target in &req.verifier_targets {
        if *target == req.shard_pid {
            return Err(RpcWorkflowError::DelegationVerifierTargetIncludesShard {
                target: *target,
                shard_pid: req.shard_pid,
            }
            .into());
        }

        if *target == root_pid {
            return Err(RpcWorkflowError::DelegationVerifierTargetIncludesRoot {
                target: *target,
                root_pid,
            }
            .into());
        }

        if !SubnetRegistryOps::is_registered(*target) {
            return Err(RpcWorkflowError::DelegationVerifierTargetNotRegistered {
                target: *target,
            }
            .into());
        }
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

    if req.audience.is_none() {
        return Err(RpcWorkflowError::RoleAttestationAudienceRequired.into());
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

pub(super) fn max_role_attestation_ttl_seconds() -> u64 {
    ConfigOps::role_attestation_config()
        .map(|cfg| cfg.max_ttl_secs)
        .unwrap_or(DEFAULT_MAX_ROLE_ATTESTATION_TTL_SECONDS)
}
