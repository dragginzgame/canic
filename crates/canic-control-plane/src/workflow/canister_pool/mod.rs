//! Root-owned maintenance for prepaid empty Canisters on one physical Subnet.

use crate::{
    ops::{
        canister_pool::CanisterPoolOps, component_registry::ComponentRegistryOps,
        storage::state::root_wasm_store::RootWasmStoreStateOps,
    },
    workflow::deployment,
};
use canic_core::{
    api::timer::{TimerApi, TimerHandle},
    cdk::types::{Cycles, Principal},
    control_plane_support::{
        error::InternalError,
        ops::ic::{
            IcOps,
            build_network::BuildNetworkOps,
            mgmt::{CanisterSettings, MgmtOps, UpdateSettingsArgs},
            nns::NnsRegistryOps,
        },
        workflow::cost_guard::CostGuardWorkflow,
        workflow::runtime::fleet_activation::FleetActivationWorkflow,
    },
    dto::{
        fleet_activation::FleetActivationPhase,
        pool::{
            CanisterPoolResponse, CanisterPoolStatusRequest, PoolAdminCommand, PoolAdminResponse,
        },
    },
    ids::{BuildNetwork, FleetSubnetCanisterPoolConfig, SubnetId},
};
use sha2::{Digest, Sha256};
use std::{cell::RefCell, time::Duration};

const MAINTENANCE_INTERVAL: Duration = Duration::from_secs(30);
const CREATION_OPERATION_DOMAIN: &[u8] = b"canic.root.canister_pool.creation.v1";
const MAX_STATUS_PAGE_ENTRIES: u16 = 256;

thread_local! {
    static MAINTENANCE_TIMER: RefCell<Option<TimerHandle>> = const { RefCell::new(None) };
}

/// Start one non-overlapping root-owned maintenance loop.
pub fn start() {
    MAINTENANCE_TIMER.with_borrow_mut(|current| {
        if let Some(existing) = current.take() {
            let _ = TimerApi::cancel(existing);
        }
        *current = Some(TimerApi::set_interval(
            MAINTENANCE_INTERVAL,
            "canic:canister_pool:maintain",
            || async {
                let _ = maintain_once().await;
            },
        ));
    });
    TimerApi::defer_lifecycle(
        Duration::ZERO,
        "canic:canister_pool:maintain_initial",
        async {
            let _ = maintain_once().await;
        },
    );
}

/// Stop proactive maintenance once root draining has fenced new allocations.
pub fn stop() {
    MAINTENANCE_TIMER.with_borrow_mut(|current| {
        if let Some(existing) = current.take() {
            let _ = TimerApi::cancel(existing);
        }
    });
}

/// Return the exact immutable policy and durable asset inventory.
pub fn status(request: CanisterPoolStatusRequest) -> Result<CanisterPoolResponse, InternalError> {
    if request.limit == 0 || request.limit > MAX_STATUS_PAGE_ENTRIES {
        return Err(InternalError::invalid_input(format!(
            "Canister pool status limit must be between 1 and {MAX_STATUS_PAGE_ENTRIES}",
        )));
    }
    Ok(CanisterPoolOps::response(
        pool_config()?,
        request.start_after,
        usize::from(request.limit),
    ))
}

/// Execute one controller-authorized maintenance command.
pub async fn admin(command: PoolAdminCommand) -> Result<PoolAdminResponse, InternalError> {
    match command {
        PoolAdminCommand::Maintain => maintain_once().await,
        PoolAdminCommand::Import { canister_id } => import(canister_id).await,
        PoolAdminCommand::RetryReset { canister_id } => {
            CanisterPoolOps::retry_reset(canister_id, IcOps::now_nanos())?;
            Ok(PoolAdminResponse::ResetQueued { canister_id })
        }
        PoolAdminCommand::Handoff {
            canister_id,
            recipient,
        } => handoff(canister_id, recipient).await,
    }
}

/// Reconcile one bounded unit of reset or refill work.
pub async fn maintain_once() -> Result<PoolAdminResponse, InternalError> {
    let status = FleetActivationWorkflow::status()?;
    if !matches!(
        status.phase,
        FleetActivationPhase::Prepared | FleetActivationPhase::Active
    ) {
        return Err(InternalError::unavailable(
            "Canister pool maintenance requires a Prepared or Active Fleet Subnet Root",
        ));
    }
    let config = pool_config()?;

    if CanisterPoolOps::pending_handoff().is_some() {
        return Ok(PoolAdminResponse::MaintenancePaused {
            reason: "Canister pool asset handoff is pending".to_string(),
        });
    }

    if let Some(canister_id) = CanisterPoolOps::pending_reset_canisters()
        .into_iter()
        .next()
    {
        return Ok(reset_admin_response(
            canister_id,
            reset_asset(canister_id, &config).await?,
        ));
    }

    if let Some(created) = reconcile_recorded_creation(&config)? {
        return Ok(PoolAdminResponse::Created {
            canister_id: created,
        });
    }
    if root_is_draining()? {
        return Ok(PoolAdminResponse::MaintenancePaused {
            reason: "Fleet Subnet Root draining has fenced proactive pool refill".to_string(),
        });
    }
    if CanisterPoolOps::ready_count() >= config.minimum_size {
        return Ok(PoolAdminResponse::Maintained);
    }
    if CanisterPoolOps::asset_count() >= config.maximum_size {
        return Err(InternalError::resource_exhausted(
            "Canister pool is below minimum_size but maximum_size is exhausted",
        ));
    }
    require_managed_creation_capacity()?;

    create_empty_asset(&config).await
}

async fn import(canister_id: Principal) -> Result<PoolAdminResponse, InternalError> {
    if root_is_draining()? {
        return Err(InternalError::conflict(
            "Canister pool import is fenced while the Fleet Subnet Root is draining",
        ));
    }
    require_import_candidate(canister_id)?;
    require_ic_import_on_root_subnet(canister_id).await?;
    let config = pool_config()?;
    if !CanisterPoolOps::contains_asset(canister_id) {
        require_managed_creation_capacity()?;
    }
    CanisterPoolOps::initialize_imports(&config, &[canister_id], IcOps::now_nanos())?;
    match reset_asset(canister_id, &config).await? {
        ResetAssetOutcome::Ready => Ok(PoolAdminResponse::Imported { canister_id }),
        ResetAssetOutcome::Underfunded { reason } => Ok(PoolAdminResponse::ResetFailed {
            canister_id,
            reason,
        }),
    }
}

async fn handoff(
    canister_id: Principal,
    recipient: Principal,
) -> Result<PoolAdminResponse, InternalError> {
    if !root_is_draining()? {
        return Err(InternalError::conflict(
            "Canister pool assets may be handed off only while the Fleet Subnet Root is draining",
        ));
    }
    let root = IcOps::canister_self();
    if recipient == Principal::anonymous()
        || recipient == Principal::management_canister()
        || recipient == root
        || recipient == canister_id
    {
        return Err(InternalError::invalid_input(
            "Canister pool handoff recipient must be distinct non-reserved replacement authority",
        ));
    }
    if let Some(existing) = CanisterPoolOps::completed_handoff_recipient(canister_id) {
        if existing == recipient {
            return Ok(PoolAdminResponse::HandedOff {
                canister_id,
                recipient,
            });
        }
        return Err(InternalError::conflict(
            "Canister pool asset was already handed to different replacement authority",
        ));
    }
    CanisterPoolOps::begin_handoff(canister_id, recipient, IcOps::now_nanos())?;
    MgmtOps::update_settings(&UpdateSettingsArgs {
        canister_id,
        settings: CanisterSettings {
            controllers: Some(vec![root, recipient]),
            ..CanisterSettings::default()
        },
        sender_canister_version: None,
    })
    .await?;
    CanisterPoolOps::complete_handoff(canister_id, recipient, IcOps::now_nanos())?;
    Ok(PoolAdminResponse::HandedOff {
        canister_id,
        recipient,
    })
}

fn require_import_candidate(canister_id: Principal) -> Result<(), InternalError> {
    let root = FleetActivationWorkflow::root_authority()?.binding;
    if canister_id == root.fleet_subnet_root
        || canister_id == root.authority.binding.coordinator
        || RootWasmStoreStateOps::wasm_stores()
            .iter()
            .any(|store| store.pid == canister_id)
    {
        return Err(InternalError::conflict(
            "Fleet infrastructure cannot be imported into the Canister pool",
        ));
    }
    if ComponentRegistryOps::component_for_principal(canister_id).is_some() {
        return Err(InternalError::conflict(
            "a registered Component-tree member cannot be imported into the Canister pool",
        ));
    }
    Ok(())
}

/// Return a stopped Component Canister to durable local prepaid inventory.
pub async fn recycle(canister_id: Principal) -> Result<(), InternalError> {
    let config = pool_config()?;
    CanisterPoolOps::register_recycled_pending(canister_id, IcOps::now_nanos())?;
    if CanisterPoolOps::asset_is_ready(canister_id)? {
        return Ok(());
    }
    let _ = reset_asset(canister_id, &config).await?;
    Ok(())
}

/// Reject a new physical Canister when current root-owned assets exhaust the protected ceiling.
pub fn require_managed_creation_capacity() -> Result<(), InternalError> {
    let root = FleetActivationWorkflow::root_authority()?.binding;
    let registry = ComponentRegistryOps::current().ok_or_else(|| {
        InternalError::unavailable("root Component Registry authority has not been prepared")
    })?;
    let managed = 1_u32
        .checked_add(registry.known_created_component_canisters)
        .and_then(|count| count.checked_add(CanisterPoolOps::asset_count()))
        .ok_or_else(|| InternalError::resource_exhausted("root managed-Canister count overflow"))?;
    if managed >= root.limits.maximum_managed_canisters {
        return Err(InternalError::resource_exhausted(
            "root managed-Canister capacity is exhausted",
        ));
    }
    Ok(())
}

fn root_is_draining() -> Result<bool, InternalError> {
    Ok(ComponentRegistryOps::current()
        .ok_or_else(|| {
            InternalError::unavailable("root Component Registry authority has not been prepared")
        })?
        .root_draining
        .is_some())
}

enum ResetAssetOutcome {
    Ready,
    Underfunded { reason: String },
}

async fn reset_asset(
    canister_id: Principal,
    config: &FleetSubnetCanisterPoolConfig,
) -> Result<ResetAssetOutcome, InternalError> {
    if CanisterPoolOps::asset_is_ready(canister_id)? {
        return Ok(ResetAssetOutcome::Ready);
    }
    let root = IcOps::canister_self();
    let result: Result<Cycles, InternalError> = async {
        MgmtOps::update_settings(&UpdateSettingsArgs {
            canister_id,
            settings: CanisterSettings {
                controllers: Some(vec![root]),
                ..CanisterSettings::default()
            },
            sender_canister_version: None,
        })
        .await?;
        MgmtOps::uninstall_code(canister_id).await?;
        let cycles = MgmtOps::get_cycles(canister_id).await?;
        Ok(cycles)
    }
    .await;

    match result {
        Ok(cycles) if cycles >= config.canister_cycles => {
            CanisterPoolOps::mark_ready(canister_id, cycles, IcOps::now_nanos())?;
            Ok(ResetAssetOutcome::Ready)
        }
        Ok(cycles) => {
            let reason = format!(
                "Canister pool asset {canister_id} has {cycles}, below configured {}",
                config.canister_cycles
            );
            CanisterPoolOps::mark_failed(
                canister_id,
                Some(cycles),
                reason.clone(),
                IcOps::now_nanos(),
            )?;
            Ok(ResetAssetOutcome::Underfunded { reason })
        }
        Err(error) => {
            CanisterPoolOps::mark_failed(canister_id, None, error.to_string(), IcOps::now_nanos())?;
            Err(error)
        }
    }
}

async fn create_empty_asset(
    config: &FleetSubnetCanisterPoolConfig,
) -> Result<PoolAdminResponse, InternalError> {
    let root = IcOps::canister_self();
    let operation_id = creation_operation_id(root, CanisterPoolOps::next_creation_sequence());
    let permit = deployment::reserve_canister_pool_creation_cost_guard(&config.canister_cycles)?;
    CanisterPoolOps::begin_creation(
        operation_id,
        config.canister_cycles.clone(),
        permit.replay_settlement(),
        IcOps::now_nanos(),
    )
    .map_err(|error| CostGuardWorkflow::recover_after_failure(&permit, IcOps::now_secs(), error))?;

    let canister_id =
        MgmtOps::create_canister_with_permit(&permit, vec![root], config.canister_cycles.clone())
            .await
            .map_err(|error| {
                CostGuardWorkflow::recover_after_failure(&permit, IcOps::now_secs(), error)
            })?;
    CanisterPoolOps::mark_creation_created(operation_id, canister_id).map_err(|error| {
        CostGuardWorkflow::complete_after_failure(&permit, IcOps::now_secs(), error)
    })?;
    CanisterPoolOps::register_created_ready(
        config,
        canister_id,
        config.canister_cycles.clone(),
        IcOps::now_nanos(),
    )
    .map_err(|error| {
        CostGuardWorkflow::complete_after_failure(&permit, IcOps::now_secs(), error)
    })?;
    CostGuardWorkflow::complete(&permit, IcOps::now_secs())?;
    CanisterPoolOps::commit_creation(operation_id)?;
    Ok(PoolAdminResponse::Created { canister_id })
}

fn reconcile_recorded_creation(
    config: &FleetSubnetCanisterPoolConfig,
) -> Result<Option<Principal>, InternalError> {
    let Some(creation) = CanisterPoolOps::pending_creation() else {
        return Ok(None);
    };
    let Some(canister_id) = creation.canister_id else {
        return Err(InternalError::unavailable(
            "Canister pool creation effect is unresolved; inspect pool status before retrying",
        ));
    };
    CanisterPoolOps::register_created_ready(
        config,
        canister_id,
        creation.canister_cycles,
        IcOps::now_nanos(),
    )?;
    CostGuardWorkflow::complete_replay_settlement(
        &creation.cost_guard_settlement,
        IcOps::now_secs(),
    )?;
    CanisterPoolOps::commit_creation(creation.operation_id)?;
    Ok(Some(canister_id))
}

fn reset_admin_response(canister_id: Principal, outcome: ResetAssetOutcome) -> PoolAdminResponse {
    match outcome {
        ResetAssetOutcome::Ready => PoolAdminResponse::ResetReady { canister_id },
        ResetAssetOutcome::Underfunded { reason } => PoolAdminResponse::ResetFailed {
            canister_id,
            reason,
        },
    }
}

async fn require_ic_import_on_root_subnet(canister_id: Principal) -> Result<(), InternalError> {
    if BuildNetworkOps::build_network() != Some(BuildNetwork::Ic) {
        return Ok(());
    }
    let expected = FleetActivationWorkflow::root_authority()?
        .binding
        .placement_subnet;
    let actual = NnsRegistryOps::get_subnet_for_canister(canister_id).await?;
    validate_import_subnet(canister_id, expected, actual)
}

fn validate_import_subnet(
    canister_id: Principal,
    expected: SubnetId,
    actual: Option<Principal>,
) -> Result<(), InternalError> {
    let actual = actual.ok_or_else(|| {
        InternalError::unavailable(format!(
            "NNS Registry has no Subnet route for Canister pool import {canister_id}"
        ))
    })?;
    if actual != expected.into_principal() {
        return Err(InternalError::conflict(format!(
            "Canister pool import {canister_id} is routed to Subnet {actual}, not root Subnet {expected}"
        )));
    }
    Ok(())
}

fn pool_config() -> Result<FleetSubnetCanisterPoolConfig, InternalError> {
    Ok(FleetActivationWorkflow::root_authority()?
        .binding
        .limits
        .canister_pool)
}

fn creation_operation_id(root: Principal, sequence: u64) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(CREATION_OPERATION_DOMAIN);
    hasher.update(root.as_slice());
    hasher.update(sequence.to_be_bytes());
    hasher.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creation_operation_identity_binds_root_and_sequence() {
        let root = Principal::from_slice(&[1; 29]);
        assert_eq!(
            creation_operation_id(root, 7),
            creation_operation_id(root, 7)
        );
        assert_ne!(
            creation_operation_id(root, 7),
            creation_operation_id(root, 8)
        );
        assert_ne!(
            creation_operation_id(root, 7),
            creation_operation_id(Principal::from_slice(&[2; 29]), 7)
        );
    }

    #[test]
    fn import_subnet_requires_exact_nns_routing_evidence() {
        let canister_id = Principal::from_slice(&[3; 29]);
        let expected = SubnetId::from_principal(Principal::from_slice(&[4; 29]));
        assert!(
            validate_import_subnet(canister_id, expected, Some(expected.into_principal())).is_ok()
        );
        assert!(validate_import_subnet(canister_id, expected, None).is_err());
        assert!(
            validate_import_subnet(canister_id, expected, Some(Principal::from_slice(&[5; 29])))
                .is_err()
        );
    }
}
