use crate::{
    InternalError,
    ops::{
        config::ConfigOps,
        ic::{
            IcOps,
            mgmt::{CanisterSettings, MgmtOps, UpdateSettingsArgs},
        },
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
    parent_pid: Principal,
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

    if let Some(allocation) = try_allocate_from_pool(role, parent_pid, target.clone()).await? {
        return Ok(allocation);
    }

    let pid = match create_canister_with_configured_controllers(role, parent_pid, target).await {
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
    parent_pid: Principal,
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

    configure_pool_allocation_controllers(role, pid, parent_pid).await?;

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

// Ensure a reused pool canister follows the active parent-controller rule.
async fn configure_pool_allocation_controllers(
    role: &CanisterRole,
    pid: Principal,
    parent_pid: Principal,
) -> Result<(), InternalError> {
    let controllers = child_canister_controllers(parent_pid)?;
    if let Err(err) = MgmtOps::update_settings(&UpdateSettingsArgs {
        canister_id: pid,
        settings: CanisterSettings {
            controllers: Some(controllers),
            ..Default::default()
        },
        sender_canister_version: None,
    })
    .await
    {
        record_canister_op(
            role,
            CanisterOpsMetricOperation::Create,
            CanisterOpsMetricOutcome::Failed,
            CanisterOpsMetricReason::ManagementCall,
        );
        record_provisioning(
            role,
            ProvisioningMetricOperation::Allocate,
            ProvisioningMetricOutcome::Failed,
            ProvisioningMetricReason::ManagementCall,
        );
        return Err(err);
    }

    Ok(())
}

/// Create a fresh canister on the IC with the configured controllers.
async fn create_canister_with_configured_controllers(
    role: &CanisterRole,
    parent_pid: Principal,
    cycles: Cycles,
) -> Result<Principal, InternalError> {
    let controllers = child_canister_controllers(parent_pid)?;

    let pid = MgmtOps::create_canister(controllers, cycles.clone()).await?;

    log!(
        Topic::CanisterLifecycle,
        Ok,
        "⚡ create_canister: {pid} role={role} cycles={cycles} source=new (pool empty)"
    );

    Ok(pid)
}

// Controller rule for non-root managed canisters: configured controllers + root + direct parent.
fn child_canister_controllers(parent_pid: Principal) -> Result<Vec<Principal>, InternalError> {
    Ok(child_canister_controllers_from_config(
        ConfigOps::controllers()?,
        IcOps::canister_self(),
        parent_pid,
    ))
}

fn child_canister_controllers_from_config(
    mut controllers: Vec<Principal>,
    root: Principal,
    parent_pid: Principal,
) -> Vec<Principal> {
    push_unique_controller(&mut controllers, root);
    push_unique_controller(&mut controllers, parent_pid);
    controllers
}

fn push_unique_controller(controllers: &mut Vec<Principal>, controller: Principal) {
    if !controllers.contains(&controller) {
        controllers.push(controller);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(byte: u8) -> Principal {
        Principal::from_slice(&[byte])
    }

    // Keep controller construction deduplicated while preserving configured order.
    #[test]
    fn push_unique_controller_deduplicates_existing_entries() {
        let mut controllers = vec![p(1), p(2)];

        push_unique_controller(&mut controllers, p(2));
        push_unique_controller(&mut controllers, p(3));

        assert_eq!(controllers, vec![p(1), p(2), p(3)]);
    }

    // Enforce the 0.35.1 child-controller rule without disturbing configured order.
    #[test]
    fn child_canister_controllers_include_root_and_direct_parent() {
        let controllers = child_canister_controllers_from_config(vec![p(7), p(2)], p(2), p(3));

        assert_eq!(controllers, vec![p(7), p(2), p(3)]);
    }
}
