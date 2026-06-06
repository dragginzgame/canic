use super::{
    RootCapability, RootContext, attestation_expires_at, nonroot_cycles,
    nonroot_cycles::AuthorizedCyclesGrant, replay,
};
use crate::{
    InternalError,
    dto::auth::{
        InternalInvocationProofPayloadV1, InternalInvocationProofRequest, RoleAttestation,
        RoleAttestationRequest,
    },
    dto::rpc::{
        CreateCanisterParent, CreateCanisterRequest, CreateCanisterResponse,
        RecycleCanisterRequest, RecycleCanisterResponse, Response, UpgradeCanisterRequest,
        UpgradeCanisterResponse,
    },
    format::display_optional,
    log,
    log::Topic,
    ops::{
        auth::AuthOps,
        cost_guard::{CostGuardOps, CostGuardPermit, CostGuardRequest},
        ic::{IcOps, mgmt::MgmtOps},
        replay::{
            guard::ReplayPending,
            model::{CommandKind, RecoveryReason},
        },
        storage::{index::subnet::SubnetIndexOps, registry::subnet::SubnetRegistryOps},
    },
    replay_policy::CostClass,
    workflow::{
        canister_lifecycle::{CanisterLifecycleEvent, CanisterLifecycleWorkflow},
        pool::PoolWorkflow,
        rpc::RpcWorkflowError,
    },
};

const ROOT_AUTH_SIGNING_QUOTA_WINDOW_SECONDS: u64 = 60;
const MAX_ROOT_AUTH_SIGNING_OPERATIONS_PER_WINDOW: u64 = 60;
const ROOT_AUTH_SIGNING_CYCLE_RESERVATION_CYCLES: u128 = 1_000_000_000;
const MIN_ROOT_AUTH_SIGNING_CYCLES_AFTER_RESERVATION: u128 = 1_000_000_000;

pub(super) async fn execute_root_capability(
    ctx: &RootContext,
    pending: &ReplayPending,
    capability: RootCapability,
    authorized_cycles: Option<AuthorizedCyclesGrant>,
) -> Result<Response, InternalError> {
    let capability_name = capability.capability_name();

    let result = match capability {
        RootCapability::Provision(req) => execute_provision(ctx, &req).await,
        RootCapability::Upgrade(req) => execute_upgrade(&req).await,
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
        RootCapability::IssueRoleAttestation(req) => {
            execute_issue_role_attestation(ctx, pending, req).await
        }
        RootCapability::IssueInternalInvocationProof(req) => {
            execute_issue_internal_invocation_proof(ctx, pending, req).await
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

fn reserve_auth_material_signing_cost_guard(
    ctx: &RootContext,
    command_kind: &'static str,
    current_cycle_balance: u128,
) -> Result<CostGuardPermit, InternalError> {
    let command_kind =
        CommandKind::new(command_kind).expect("root auth signing command kind is valid");
    CostGuardOps::reserve(CostGuardRequest {
        cost_class: CostClass::ThresholdEcdsaSign,
        command_kind,
        quota_subject: ctx.caller,
        payer: ctx.self_pid,
        now_secs: IcOps::now_secs(),
        quota_window_secs: ROOT_AUTH_SIGNING_QUOTA_WINDOW_SECONDS,
        max_operations_per_window: MAX_ROOT_AUTH_SIGNING_OPERATIONS_PER_WINDOW,
        current_cycle_balance,
        cycle_reservation_cycles: ROOT_AUTH_SIGNING_CYCLE_RESERVATION_CYCLES,
        min_cycles_after_reservation: MIN_ROOT_AUTH_SIGNING_CYCLES_AFTER_RESERVATION,
    })
}

async fn execute_provision(
    ctx: &RootContext,
    req: &CreateCanisterRequest,
) -> Result<Response, InternalError> {
    let parent_pid = match &req.parent {
        CreateCanisterParent::Canister(pid) => *pid,
        CreateCanisterParent::Root => IcOps::canister_self(),
        CreateCanisterParent::ThisCanister => ctx.caller,
        CreateCanisterParent::Parent => SubnetRegistryOps::get_parent(ctx.caller)
            .ok_or(RpcWorkflowError::ParentNotFound(ctx.caller))?,
        CreateCanisterParent::Index(role) => SubnetIndexOps::get(role)
            .ok_or_else(|| RpcWorkflowError::CanisterRoleNotFound(role.clone()))?,
    };

    let event = CanisterLifecycleEvent::Create {
        role: req.canister_role.clone(),
        parent: parent_pid,
        extra_arg: req.extra_arg.clone(),
    };

    let lifecycle_result = CanisterLifecycleWorkflow::apply(event).await?;
    let new_canister_pid = lifecycle_result
        .new_canister_pid
        .ok_or(RpcWorkflowError::MissingNewCanisterPid)?;

    Ok(Response::CreateCanister(CreateCanisterResponse {
        new_canister_pid,
    }))
}

async fn execute_upgrade(req: &UpgradeCanisterRequest) -> Result<Response, InternalError> {
    let event = CanisterLifecycleEvent::Upgrade {
        pid: req.canister_pid,
    };

    CanisterLifecycleWorkflow::apply(event).await?;

    Ok(Response::UpgradeCanister(UpgradeCanisterResponse {}))
}

async fn execute_recycle(req: &RecycleCanisterRequest) -> Result<Response, InternalError> {
    PoolWorkflow::pool_recycle_canister(req.canister_pid).await?;

    Ok(Response::RecycleCanister(RecycleCanisterResponse {}))
}

async fn execute_issue_role_attestation(
    ctx: &RootContext,
    pending: &ReplayPending,
    req: RoleAttestationRequest,
) -> Result<Response, InternalError> {
    let payload = build_role_attestation(ctx, req)?;
    let prepared = AuthOps::prepare_role_attestation_signature(payload).await?;
    let cost_permit = reserve_auth_material_signing_cost_guard(
        ctx,
        "root.issue_role_attestation.v1",
        MgmtOps::canister_cycle_balance().to_u128(),
    )?;
    replay::mark_external_effect_in_flight(
        pending,
        AuthOps::role_attestation_signing_effect(&prepared),
    );
    let signed = match AuthOps::sign_prepared_role_attestation(&cost_permit, prepared).await {
        Ok(signed) => signed,
        Err(err) => {
            let _ = CostGuardOps::recover(&cost_permit, IcOps::now_secs());
            replay::mark_recovery_required(pending, RecoveryReason::ExternalEffectStatusUnknown);
            return Err(err);
        }
    };
    if let Err(err) = CostGuardOps::complete(&cost_permit, IcOps::now_secs()) {
        replay::mark_recovery_required(pending, RecoveryReason::ResponseCommitFailed);
        return Err(err);
    }
    log!(
        Topic::Auth,
        Info,
        "role attestation issued subject={} role={} audience={} subnet={} issued_at={} expires_at={} epoch={}",
        signed.payload.subject,
        signed.payload.role,
        signed.payload.audience,
        display_optional(signed.payload.subnet_id),
        signed.payload.issued_at,
        signed.payload.expires_at,
        signed.payload.epoch
    );
    Ok(Response::RoleAttestationIssued(signed))
}

async fn execute_issue_internal_invocation_proof(
    ctx: &RootContext,
    pending: &ReplayPending,
    req: InternalInvocationProofRequest,
) -> Result<Response, InternalError> {
    let payload = build_internal_invocation_proof(ctx, req)?;
    let prepared = AuthOps::prepare_internal_invocation_proof_signature(payload).await?;
    let cost_permit = reserve_auth_material_signing_cost_guard(
        ctx,
        "root.issue_internal_invocation_proof.v1",
        MgmtOps::canister_cycle_balance().to_u128(),
    )?;
    replay::mark_external_effect_in_flight(
        pending,
        AuthOps::internal_invocation_proof_signing_effect(&prepared),
    );
    let signed = match AuthOps::sign_prepared_internal_invocation_proof(&cost_permit, prepared)
        .await
    {
        Ok(signed) => signed,
        Err(err) => {
            let _ = CostGuardOps::recover(&cost_permit, IcOps::now_secs());
            replay::mark_recovery_required(pending, RecoveryReason::ExternalEffectStatusUnknown);
            return Err(err);
        }
    };
    if let Err(err) = CostGuardOps::complete(&cost_permit, IcOps::now_secs()) {
        replay::mark_recovery_required(pending, RecoveryReason::ResponseCommitFailed);
        return Err(err);
    }
    log!(
        Topic::Auth,
        Info,
        "internal invocation proof issued subject={} role={} audience={} method={} subnet={} issued_at={} expires_at={} epoch={}",
        signed.payload.subject,
        signed.payload.role,
        signed.payload.audience,
        signed.payload.audience_method,
        display_optional(signed.payload.subnet_id),
        signed.payload.issued_at,
        signed.payload.expires_at,
        signed.payload.epoch
    );
    Ok(Response::InternalInvocationProofIssued(signed))
}

pub(super) fn build_role_attestation(
    ctx: &RootContext,
    req: RoleAttestationRequest,
) -> Result<RoleAttestation, InternalError> {
    let expires_at = attestation_expires_at(ctx.now, req.ttl_secs)?;
    let epoch = AuthOps::current_role_epoch(&req.role)?;

    Ok(RoleAttestation {
        subject: req.subject,
        role: req.role,
        subnet_id: req.subnet_id,
        audience: req.audience,
        issued_at: ctx.now,
        expires_at,
        epoch,
    })
}

pub(super) fn build_internal_invocation_proof(
    ctx: &RootContext,
    req: InternalInvocationProofRequest,
) -> Result<InternalInvocationProofPayloadV1, InternalError> {
    if req.audience_method.trim().is_empty() {
        return Err(RpcWorkflowError::InternalInvocationProofMethodEmpty.into());
    }

    let expires_at = attestation_expires_at(ctx.now, req.ttl_secs)?;
    let epoch = AuthOps::current_role_epoch(&req.role)?;

    Ok(InternalInvocationProofPayloadV1 {
        subject: req.subject,
        role: req.role,
        subnet_id: req.subnet_id,
        audience: req.audience,
        audience_method: req.audience_method,
        issued_at: ctx.now,
        expires_at,
        epoch,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        cdk::types::Principal, ops::storage::intent::IntentStoreOps,
        storage::stable::intent::IntentStore,
    };

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn ctx() -> RootContext {
        RootContext {
            caller: p(3),
            self_pid: p(42),
            is_root_env: true,
            subnet_id: p(4),
            now: 2_000,
        }
    }

    #[test]
    fn auth_material_signing_cost_guard_rejects_low_cycle_reserve_before_recording_intents() {
        IntentStore::reset_for_tests();

        let err = reserve_auth_material_signing_cost_guard(
            &ctx(),
            "root.issue_role_attestation.v1",
            ROOT_AUTH_SIGNING_CYCLE_RESERVATION_CYCLES,
        )
        .expect_err("low cycle reserve rejected");

        assert!(err.to_string().contains("cycle reserve"));
        assert_eq!(IntentStoreOps::pending_total().expect("pending total"), 0);
    }

    #[test]
    fn auth_material_signing_cost_guard_reservation_completes() {
        IntentStore::reset_for_tests();

        let permit = reserve_auth_material_signing_cost_guard(
            &ctx(),
            "root.issue_internal_invocation_proof.v1",
            ROOT_AUTH_SIGNING_CYCLE_RESERVATION_CYCLES
                + MIN_ROOT_AUTH_SIGNING_CYCLES_AFTER_RESERVATION,
        )
        .expect("reservation");

        CostGuardOps::complete(&permit, ctx().now).expect("complete");
        assert_eq!(IntentStoreOps::pending_total().expect("pending total"), 0);
    }
}
