use super::{DEFAULT_MAX_ROLE_ATTESTATION_TTL_SECONDS, RootCapability, RootContext};
use crate::{
    InternalError,
    dto::auth::{DelegationRequest, RoleAttestationRequest},
    dto::rpc::{CyclesRequest, UpgradeCanisterRequest},
    log,
    log::Topic,
    ops::{
        config::ConfigOps,
        ic::{IcOps, mgmt::MgmtOps},
        runtime::env::EnvOps,
        runtime::metrics::root_capability::{
            RootCapabilityMetricEventType, RootCapabilityMetricOutcome, RootCapabilityMetrics,
        },
        storage::registry::subnet::SubnetRegistryOps,
    },
    workflow::rpc::RpcWorkflowError,
};

pub(super) fn authorize(
    ctx: &RootContext,
    capability: &RootCapability,
) -> Result<(), InternalError> {
    if !ctx.is_root_env {
        RootCapabilityMetrics::record(
            capability.metric_key(),
            RootCapabilityMetricEventType::Authorization,
            RootCapabilityMetricOutcome::Denied,
        );
        return EnvOps::require_root();
    }

    let capability_key = capability.metric_key();
    let capability_name = capability.capability_name();
    let decision = match capability {
        RootCapability::Provision(_req) => Ok(()),
        RootCapability::Upgrade(req) => authorize_upgrade(ctx, req),
        RootCapability::MintCycles(req) => authorize_mint_cycles(ctx, req),
        RootCapability::IssueDelegation(req) => authorize_issue_delegation(ctx, req),
        RootCapability::IssueRoleAttestation(req) => authorize_issue_role_attestation(ctx, req),
    };

    match &decision {
        Ok(()) => {
            RootCapabilityMetrics::record(
                capability_key,
                RootCapabilityMetricEventType::Authorization,
                RootCapabilityMetricOutcome::Accepted,
            );
            log!(
                Topic::Rpc,
                Info,
                "root capability authorized (capability={capability_name}, caller={}, subnet={}, now={})",
                ctx.caller,
                ctx.subnet_id,
                ctx.now
            );
        }
        Err(err) => {
            RootCapabilityMetrics::record(
                capability_key,
                RootCapabilityMetricEventType::Authorization,
                RootCapabilityMetricOutcome::Denied,
            );
            log!(
                Topic::Rpc,
                Warn,
                "root capability denied (capability={capability_name}, caller={}, subnet={}, now={}): {err}",
                ctx.caller,
                ctx.subnet_id,
                ctx.now
            );
        }
    }

    decision
}

fn authorize_upgrade(ctx: &RootContext, req: &UpgradeCanisterRequest) -> Result<(), InternalError> {
    let registry_entry = SubnetRegistryOps::get(req.canister_pid)
        .ok_or(RpcWorkflowError::ChildNotFound(req.canister_pid))?;

    if registry_entry.parent_pid != Some(ctx.caller) {
        return Err(RpcWorkflowError::NotChildOfCaller(req.canister_pid, ctx.caller).into());
    }

    Ok(())
}

fn authorize_mint_cycles(_ctx: &RootContext, req: &CyclesRequest) -> Result<(), InternalError> {
    let available = MgmtOps::canister_cycle_balance().to_u128();
    if req.cycles > available {
        return Err(RpcWorkflowError::InsufficientRootCycles {
            requested: req.cycles,
            available,
        }
        .into());
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

    let root_pid = EnvOps::root_pid()?;
    if root_pid != IcOps::canister_self() {
        return Err(RpcWorkflowError::DelegationMustTargetRoot.into());
    }

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
