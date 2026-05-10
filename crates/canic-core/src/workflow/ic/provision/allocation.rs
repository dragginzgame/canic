use crate::{
    InternalError,
    config::Config,
    ops::{
        config::ConfigOps,
        ic::{IcOps, mgmt::MgmtOps},
        runtime::metrics::{
            canister_ops::{
                CanisterOpsMetricOperation, CanisterOpsMetricOutcome, CanisterOpsMetricReason,
            },
            provisioning::{
                ProvisioningMetricOperation, ProvisioningMetricOutcome, ProvisioningMetricReason,
            },
        },
    },
    workflow::{
        ic::provision::metrics::{
            record_canister_op, record_provisioning, record_provisioning_failure,
        },
        pool::PoolWorkflow,
        prelude::*,
    },
};

///
/// AllocationSource
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum AllocationSource {
    Pool,
    New,
}

/// Allocate a canister ID and ensure it meets the initial cycle target.
///
/// Reuses a canister from the pool if available; otherwise creates a new one.
pub(super) async fn allocate_canister(
    role: &CanisterRole,
) -> Result<(Principal, AllocationSource), InternalError> {
    record_provisioning(
        role,
        ProvisioningMetricOperation::Allocate,
        ProvisioningMetricOutcome::Started,
        ProvisioningMetricReason::Ok,
    );
    let cfg = match ConfigOps::current_subnet_canister(role) {
        Ok(cfg) => cfg,
        Err(err) => {
            record_provisioning_failure(role, ProvisioningMetricOperation::Allocate, &err);
            return Err(err);
        }
    };
    let target = cfg.initial_cycles;

    if let Some(allocation) = try_allocate_from_pool(role, target.clone()).await? {
        return Ok(allocation);
    }

    let pid = match create_canister_with_configured_controllers(role, target).await {
        Ok(pid) => pid,
        Err(err) => {
            record_canister_op(
                role,
                CanisterOpsMetricOperation::Create,
                CanisterOpsMetricOutcome::Failed,
                CanisterOpsMetricReason::NewAllocation,
            );
            record_provisioning(
                role,
                ProvisioningMetricOperation::Allocate,
                ProvisioningMetricOutcome::Failed,
                ProvisioningMetricReason::NewAllocation,
            );
            return Err(err);
        }
    };

    record_canister_op(
        role,
        CanisterOpsMetricOperation::Create,
        CanisterOpsMetricOutcome::Completed,
        CanisterOpsMetricReason::NewAllocation,
    );
    record_provisioning(
        role,
        ProvisioningMetricOperation::Allocate,
        ProvisioningMetricOutcome::Completed,
        ProvisioningMetricReason::NewAllocation,
    );

    Ok((pid, AllocationSource::New))
}

// Reuse a ready pool canister when one is available.
async fn try_allocate_from_pool(
    role: &CanisterRole,
    target: Cycles,
) -> Result<Option<(Principal, AllocationSource)>, InternalError> {
    let Some(pid) = PoolWorkflow::pop_oldest_ready() else {
        return Ok(None);
    };

    let mut current = match MgmtOps::get_cycles(pid).await {
        Ok(current) => current,
        Err(err) => {
            record_provisioning_failure(role, ProvisioningMetricOperation::Allocate, &err);
            return Err(err);
        }
    };

    if current < target {
        current = topup_pool_allocation(role, pid, current, target).await?;
    }

    log!(
        Topic::CanisterPool,
        Ok,
        "⚡ allocate_canister: reusing {pid} role={role} from pool (current {current})"
    );
    record_canister_op(
        role,
        CanisterOpsMetricOperation::Create,
        CanisterOpsMetricOutcome::Completed,
        CanisterOpsMetricReason::PoolReuse,
    );
    record_provisioning(
        role,
        ProvisioningMetricOperation::Allocate,
        ProvisioningMetricOutcome::Completed,
        ProvisioningMetricReason::PoolReuse,
    );

    Ok(Some((pid, AllocationSource::Pool)))
}

// Top up a reused pool canister to the configured initial cycle target.
async fn topup_pool_allocation(
    role: &CanisterRole,
    pid: Principal,
    current: Cycles,
    target: Cycles,
) -> Result<Cycles, InternalError> {
    let missing = target.to_u128().saturating_sub(current.to_u128());
    if missing == 0 {
        return Ok(current);
    }

    if let Err(err) = MgmtOps::deposit_cycles(pid, missing).await {
        record_canister_op(
            role,
            CanisterOpsMetricOperation::Create,
            CanisterOpsMetricOutcome::Failed,
            CanisterOpsMetricReason::PoolTopup,
        );
        record_provisioning(
            role,
            ProvisioningMetricOperation::Allocate,
            ProvisioningMetricOutcome::Failed,
            ProvisioningMetricReason::PoolTopup,
        );
        return Err(err);
    }

    log!(
        Topic::CanisterPool,
        Ok,
        "⚡ allocate_canister: topped up {pid} by {} to meet target {}",
        Cycles::from(missing),
        target
    );
    Ok(Cycles::new(current.to_u128() + missing))
}

/// Create a fresh canister on the IC with the configured controllers.
async fn create_canister_with_configured_controllers(
    role: &CanisterRole,
    cycles: Cycles,
) -> Result<Principal, InternalError> {
    let root = IcOps::canister_self();
    let mut controllers = Config::get()?.controllers.clone();
    controllers.push(root);

    let pid = MgmtOps::create_canister(controllers, cycles.clone()).await?;

    log!(
        Topic::CanisterLifecycle,
        Ok,
        "⚡ create_canister: {pid} role={role} cycles={cycles} source=new (pool empty)"
    );

    Ok(pid)
}
