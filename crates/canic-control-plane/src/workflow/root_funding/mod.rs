//! Module: workflow::root_funding
//!
//! Responsibility: orchestrate Root request journaling and atomic attached-cycle acceptance.
//! Does not own: protected authority, Registry mirror storage, policy hashing, or timers.
//! Boundary: the endpoint authenticates first; this workflow validates, accepts once, and receipts.

use crate::{
    dto::root::RootFundingStatusResponse,
    ops::{
        component_registry::ComponentRegistryOps, fleet_registry_mirror::FleetRegistryMirrorOps,
        root_funding::RootFundingOps,
    },
    view::root_funding::{
        RootFundingAcceptanceDisposition, RootFundingAuthorityView, RootFundingScheduleView,
    },
    workflow::{
        fleet_coordinator_client, fleet_registry_mirror, root_authority::validated_root_authority,
    },
};
use candid::Principal;
use canic_core::{
    control_plane_support::{
        error::InternalError,
        ops::{ic::IcOps, icp_refill::IcpRefillStoreOps},
    },
    dto::fleet_funding::{
        FleetFundingPolicyRotationRootActivateRequest,
        FleetFundingPolicyRotationRootPrepareRequest, FleetFundingPolicyRotationRootReceipt,
        FleetRootFundingAcceptanceReceipt, FleetRootFundingAcceptanceRequest,
        FleetRootFundingRequest, FleetRootFundingResponse,
    },
    shared_support::fleet_funding_policy::fleet_subnet_root_funding_policy_hash,
};

/// Initialize the independent Root funding journal on fresh install.
pub fn initialize() -> Result<(), InternalError> {
    RootFundingOps::commit_genesis(RootFundingOps::compile_genesis()).map(|_| ())
}

/// Authenticate the public acceptance command against the exact protected Coordinator.
pub fn authorize_coordinator(caller: Principal) -> Result<(), InternalError> {
    let (protected, _) = validated_root_authority()?;
    if caller == Principal::anonymous() || caller != protected.binding.authority.binding.coordinator
    {
        return Err(InternalError::forbidden());
    }
    Ok(())
}

/// Persist or resume the exact next request before an outbound Coordinator call.
pub fn prepare_request() -> Result<FleetRootFundingRequest, InternalError> {
    let authority = funding_authority()?;
    RootFundingOps::prepare_request(
        &authority,
        IcOps::canister_cycle_balance().to_u128(),
        IcOps::now_nanos(),
    )
}

/// Return the validated durable request that must resume before any new work.
pub fn current_request() -> Result<Option<FleetRootFundingRequest>, InternalError> {
    RootFundingOps::current_request(&funding_authority()?)
}

/// Reject authority capture while a value-transfer workflow still requires reconciliation.
pub fn require_authority_snapshot_resumable() -> Result<(), InternalError> {
    if RootFundingOps::policy_rotation_in_progress()
        || current_request()?.is_some()
        || IcpRefillStoreOps::resumable_operation_count() != 0
    {
        return Err(InternalError::conflict());
    }
    Ok(())
}

/// Return the protected normal-threshold schedule for the sole Root timer owner.
pub fn schedule() -> Result<RootFundingScheduleView, InternalError> {
    if RootFundingOps::policy_rotation_in_progress() {
        return Err(InternalError::conflict());
    }
    let authority = funding_authority()?;
    Ok(RootFundingScheduleView {
        request_threshold: authority.funding.root_funding.request_threshold.to_u128(),
        cooldown_secs: authority.funding.root_funding.cooldown_secs,
    })
}

/// Fence this Root and retain one exact Coordinator-authored rotation request.
pub fn prepare_policy_rotation(
    request: FleetFundingPolicyRotationRootPrepareRequest,
) -> Result<FleetFundingPolicyRotationRootReceipt, InternalError> {
    let authority = funding_authority()?;
    if let Some(receipt) = RootFundingOps::policy_rotation_prepare_replay(&authority, &request)? {
        return Ok(receipt);
    }
    canic_core::api::runtime::root_funding::RootFundingTimerApi::prepare_policy_rotation_fence()?;
    let result = RootFundingOps::prepare_policy_rotation(&authority, request, IcOps::now_nanos());
    if result.is_err() {
        let _ = canic_core::api::runtime::root_funding::RootFundingTimerApi::reconcile();
    }
    result
}

/// Converge protected authority, Registry mirror and the existing Root journal.
pub async fn activate_policy_rotation(
    request: FleetFundingPolicyRotationRootActivateRequest,
) -> Result<FleetFundingPolicyRotationRootReceipt, InternalError> {
    let Ok(prepared) = RootFundingOps::prepared_policy_rotation() else {
        let authority = funding_authority()?;
        let receipt = RootFundingOps::completed_policy_rotation(&authority, &request)?
            .ok_or_else(InternalError::conflict)?;
        canic_core::api::runtime::root_funding::RootFundingTimerApi::reconcile()
            .map_err(|_| InternalError::unavailable())?;
        return Ok(receipt);
    };
    let plan = &prepared.request;
    if request.operation_id != plan.operation_id
        || request.plan_digest != plan.plan_digest
        || request.predecessor_registry != plan.predecessor_registry
        || request.predecessor_generation != plan.predecessor_generation
        || request.successor_generation != plan.successor_generation
        || request.fleet_subnet_root != plan.root.fleet_subnet_root
    {
        return Err(InternalError::conflict());
    }

    let (protected, root) = validated_root_authority()?;
    if root != request.fleet_subnet_root {
        return Err(InternalError::conflict());
    }
    let mut successor_funding = protected.binding.funding.clone();
    successor_funding.root_funding = plan.root.proposed_policy.clone();
    let current_hash = fleet_subnet_root_funding_policy_hash(&protected.binding.funding);
    let successor_hash = fleet_subnet_root_funding_policy_hash(&successor_funding);
    if current_hash == plan.root.predecessor_policy_hash {
        canic_core::control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow::replace_root_funding_authority(
            &protected.binding.funding,
            successor_funding,
        )?;
    } else if current_hash != successor_hash {
        return Err(InternalError::conflict());
    }

    fleet_registry_mirror::advance_for_funding_policy_rotation(
        request.predecessor_registry,
        request.successor_registry.clone(),
        plan.root.predecessor_policy_hash,
    )
    .await?;
    let authority = funding_authority()?;
    if authority.registry != request.successor_registry {
        return Err(InternalError::conflict());
    }
    let receipt = RootFundingOps::complete_policy_rotation(
        &authority,
        request.operation_id,
        request.plan_digest,
        IcOps::now_nanos(),
    )?;
    canic_core::api::runtime::root_funding::RootFundingTimerApi::reconcile()
        .map_err(|_| InternalError::unavailable())?;
    Ok(receipt)
}

/// Return the controller-only Root funding and emergency-refill projection.
pub fn status() -> Result<RootFundingStatusResponse, InternalError> {
    let authority = funding_authority()?;
    RootFundingOps::status(
        &authority,
        canic_core::api::state::FleetStateQuery::snapshot().cycles_funding_enabled,
        IcOps::canister_cycle_balance().to_u128(),
        IcOps::now_secs(),
    )
}

/// Invoke the exact retained request against its protected Coordinator.
pub async fn request_coordinator(
    request: FleetRootFundingRequest,
) -> Result<FleetRootFundingResponse, InternalError> {
    let current = current_request()?.ok_or_else(InternalError::conflict)?;
    if current != request {
        return Err(InternalError::conflict());
    }
    let coordinator = request.expected_registry.authority.binding.coordinator;
    fleet_coordinator_client::request_root_funding(coordinator, request).await
}

/// Atomically accept one exact attached grant or replay its receipt while accepting zero.
pub fn accept(
    request: FleetRootFundingAcceptanceRequest,
) -> Result<FleetRootFundingAcceptanceReceipt, InternalError> {
    let authority = funding_authority()?;
    let incoming_cycles = IcOps::msg_cycles_available();
    let current_balance = IcOps::canister_cycle_balance().to_u128();
    match RootFundingOps::prepare_acceptance(
        &authority,
        &request,
        incoming_cycles,
        current_balance,
    )? {
        RootFundingAcceptanceDisposition::Replay(receipt) => Ok(*receipt),
        RootFundingAcceptanceDisposition::Fresh => {
            let accepted = IcOps::msg_cycles_accept(request.granted_cycles.to_u128());
            if accepted != request.granted_cycles.to_u128() {
                ic_cdk::trap("Root funding acceptance did not accept the exact validated grant");
            }
            Ok(
                RootFundingOps::record_acceptance(&authority, &request, IcOps::now_nanos())
                    .unwrap_or_else(|error| {
                        ic_cdk::trap(format!(
                            "Root funding receipt commitment failed after cycle acceptance: {error}"
                        ))
                    }),
            )
        }
    }
}

/// Commit the exact Coordinator terminal response and release the current request slot.
pub fn record_response(
    response: FleetRootFundingResponse,
) -> Result<FleetRootFundingResponse, InternalError> {
    RootFundingOps::record_response(&funding_authority()?, response, IcOps::now_nanos())
}

fn funding_authority() -> Result<RootFundingAuthorityView, InternalError> {
    let (protected, root) = validated_root_authority()?;
    let mirror = FleetRegistryMirrorOps::validated_current(&protected, root)?;
    Ok(RootFundingAuthorityView {
        registry: mirror.active.snapshot.version,
        fleet_subnet_root: root,
        status: mirror.root_entry.status,
        funding_eligible: ComponentRegistryOps::root_funding_eligible(mirror.root_entry.status)?,
        funding: protected.binding.funding,
    })
}
