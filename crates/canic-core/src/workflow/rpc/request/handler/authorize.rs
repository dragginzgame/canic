use super::{
    DEFAULT_MAX_ROLE_ATTESTATION_TTL_SECONDS, RootCapability, RootContext,
    funding::{self, FundingDecision, FundingPolicyViolation},
};
use crate::{
    InternalError,
    dto::auth::{DelegationRequest, RoleAttestationRequest},
    dto::rpc::{CyclesRequest, UpgradeCanisterRequest},
    ids::CanisterRole,
    log,
    log::Topic,
    ops::{
        config::ConfigOps,
        ic::{IcOps, mgmt::MgmtOps},
        runtime::env::EnvOps,
        runtime::metrics::cycles_funding::{CyclesFundingDeniedReason, CyclesFundingMetrics},
        runtime::metrics::root_capability::{
            RootCapabilityAuthorizationOutcome, RootCapabilityMetrics,
        },
        storage::{registry::subnet::SubnetRegistryOps, state::app::AppStateOps},
    },
    workflow::rpc::RpcWorkflowError,
};

pub(super) fn authorize(
    ctx: &RootContext,
    capability: &RootCapability,
) -> Result<(), InternalError> {
    let capability_key = capability.metric_key();
    let capability_name = capability.capability_name();
    let decision = match capability {
        RootCapability::Provision(_req) => authorize_root_only(ctx),
        RootCapability::Upgrade(req) => {
            authorize_root_only(ctx).and_then(|()| authorize_upgrade(ctx, req))
        }
        RootCapability::RequestCycles(req) => authorize_request_cycles(ctx, req),
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
                capability_key,
                RootCapabilityAuthorizationOutcome::Accepted,
            );
            log!(
                Topic::Rpc,
                Info,
                "capability authorized (capability={capability_name}, caller={}, subnet={}, now={})",
                ctx.caller,
                ctx.subnet_id,
                ctx.now
            );
        }
        Err(err) => {
            RootCapabilityMetrics::record_authorization(
                capability_key,
                RootCapabilityAuthorizationOutcome::Denied,
            );
            log!(
                Topic::Rpc,
                Warn,
                "capability denied (capability={capability_name}, caller={}, subnet={}, now={}): {err}",
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

fn authorize_request_cycles(ctx: &RootContext, req: &CyclesRequest) -> Result<(), InternalError> {
    CyclesFundingMetrics::record_requested(ctx.caller, req.cycles);

    let Some(child) = SubnetRegistryOps::get(ctx.caller) else {
        CyclesFundingMetrics::record_denied(
            ctx.caller,
            req.cycles,
            CyclesFundingDeniedReason::ChildNotFound,
        );
        return Err(RpcWorkflowError::ChildNotFound(ctx.caller).into());
    };
    let root_self_request = ctx.is_root_env
        && ctx.caller == ctx.self_pid
        && child.role == CanisterRole::ROOT
        && child.parent_pid.is_none();
    if child.parent_pid != Some(ctx.self_pid) && !root_self_request {
        CyclesFundingMetrics::record_denied(
            ctx.caller,
            req.cycles,
            CyclesFundingDeniedReason::NotDirectChild,
        );
        return Err(RpcWorkflowError::NotChildOfCaller(ctx.caller, ctx.self_pid).into());
    }

    if !AppStateOps::cycles_funding_enabled() {
        CyclesFundingMetrics::record_denied(
            ctx.caller,
            req.cycles,
            CyclesFundingDeniedReason::KillSwitchDisabled,
        );
        return Err(RpcWorkflowError::CyclesFundingDisabled.into());
    }

    let policy = funding::policy_for_child_role(&child.role);
    let decision = match policy.evaluate(ctx.caller, req.cycles, ctx.now) {
        Ok(decision) => decision,
        Err(violation) => {
            return match violation {
                FundingPolicyViolation::MaxPerChild {
                    requested,
                    max_per_child,
                    remaining_budget,
                } => {
                    CyclesFundingMetrics::record_denied(
                        ctx.caller,
                        req.cycles,
                        CyclesFundingDeniedReason::MaxPerChildExceeded,
                    );
                    Err(RpcWorkflowError::FundingRequestExceedsChildBudget {
                        requested,
                        remaining_budget,
                        max_per_child,
                    }
                    .into())
                }
                FundingPolicyViolation::CooldownActive { retry_after_secs } => {
                    CyclesFundingMetrics::record_denied(
                        ctx.caller,
                        req.cycles,
                        CyclesFundingDeniedReason::CooldownActive,
                    );
                    Err(RpcWorkflowError::FundingCooldownActive { retry_after_secs }.into())
                }
            };
        }
    };

    log_clamped_cycles_request(ctx, req, decision);

    let available = MgmtOps::canister_cycle_balance().to_u128();
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

    Ok(())
}

fn log_clamped_cycles_request(ctx: &RootContext, req: &CyclesRequest, decision: FundingDecision) {
    if !decision.clamped_max_per_request && !decision.clamped_max_per_child {
        return;
    }

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
