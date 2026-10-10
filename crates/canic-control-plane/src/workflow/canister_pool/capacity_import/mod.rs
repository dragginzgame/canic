//! Reviewed Root capacity import orchestration under durable pool ownership.
//!
//! Endpoint adapters authenticate controller authority; ops retains intent before every call.

use crate::{
    ops::canister_pool::capacity_import::CanisterPoolImportOps,
    view::canister_pool::{
        PoolImportCallBudgetView, PoolImportCallReservationView, PoolImportHistoryView,
        PoolImportObservationView,
    },
    workflow::canister_pool::require_import_candidate,
};
use canic_contracts::{
    dto::pool_import::{
        PoolImportCommand, PoolImportContext, PoolImportIdentity, PoolImportPhase,
        PoolImportReservation, PoolImportSourceProgress, PoolImportStatus,
    },
    ids::BuildNetwork,
};
use canic_core::{
    cdk::types::Principal,
    control_plane_support::{
        error::InternalError,
        ops::ic::{
            IcOps,
            build_network::BuildNetworkOps,
            mgmt::{CanisterSettings, MgmtOps, UpdateSettingsArgs},
            nns::NnsRegistryOps,
        },
        workflow::runtime::fleet_activation::FleetActivationWorkflow,
    },
};

/// Route authenticated current-contract import actions to the durable owner.

pub async fn command(request: PoolImportCommand) -> Result<PoolImportStatus, InternalError> {
    match request {
        PoolImportCommand::Advance {
            identity,
            canister_id,
        } => advance(identity, canister_id).await,
        PoolImportCommand::Release {
            identity,
            publication_sha256,
        } => release(identity, publication_sha256),
        PoolImportCommand::Reserve(request) => reserve(*request),
        PoolImportCommand::Settle(identity) => settle(identity).await,
    }
}

/// Return effect-free current authority for one initialized destination Root.
pub fn context() -> Result<PoolImportContext, InternalError> {
    let authority = FleetActivationWorkflow::root_authority()?;
    if crate::ops::canister_pool::capacity_import::bootstrap::pending() {
        let mirror = crate::ops::fleet_registry_mirror::FleetRegistryMirrorOps::validated_current(
            &authority,
            IcOps::canister_self(),
        )?;
        if mirror.root_entry.status
            != canic_contracts::dto::fleet_registry::FleetSubnetRootStatus::Active
        {
            return Err(InternalError::conflict());
        }
    } else {
        FleetActivationWorkflow::require_active()?;
    }
    CanisterPoolImportOps::require_no_competing_operation()?;
    let binding = authority.binding;
    let maximum_call_debit_cycles =
        crate::ops::canister_pool::capacity_import::quote::maximum_call_debit(
            binding
                .authority
                .binding
                .root_controllers(binding.fleet_subnet_root),
        )?;
    Ok(PoolImportContext {
        maximum_call_debit_cycles,
        bootstrap: crate::ops::canister_pool::capacity_import::bootstrap::context(),
        root_authority_sha256: CanisterPoolImportOps::authority_hash(&binding)?,
        next_sequence: CanisterPoolImportOps::next_sequence(),
        active_import: CanisterPoolImportOps::active_identity(),
        binding,
    })
}

/// Freeze the exact reservation before the operator grants Root controller access.
pub fn reserve(request: PoolImportReservation) -> Result<PoolImportStatus, InternalError> {
    require_destination(&request)?;
    let binding = FleetActivationWorkflow::root_authority()?.binding;
    let quote = crate::ops::canister_pool::capacity_import::quote::maximum_call_debit(
        request.final_controllers.clone(),
    )?;
    let minimum_calls = canic_core::control_plane_support::policy::pool_import::minimum_calls(
        request.sources.len(),
    )
    .ok_or_else(InternalError::invalid_input)?;
    let required = canic_core::control_plane_support::policy::pool_import::required_debit(
        quote,
        request.maximum_paid_calls,
    )
    .ok_or_else(InternalError::resource_exhausted)?;
    if request.maximum_paid_calls < minimum_calls || request.maximum_root_debit_cycles < required {
        return Err(InternalError::resource_exhausted());
    }
    for source in &request.sources {
        require_import_candidate(source.canister_id)?;
    }
    CanisterPoolImportOps::reserve(request, &binding.limits.canister_pool, IcOps::now_nanos())
}

/// Read retained evidence without causing a management observation or reset.
pub fn status(identity: PoolImportIdentity) -> Result<PoolImportStatus, InternalError> {
    CanisterPoolImportOps::status(identity)
}

/// Reconcile issued work and submit its next mutation from one fresh coherent observation.
/// Every mutation retains its own intent and allowance; a failed continuation resumes there.
pub async fn advance(
    identity: PoolImportIdentity,
    canister_id: Principal,
) -> Result<PoolImportStatus, InternalError> {
    let retained = CanisterPoolImportOps::status(identity)?;
    let index = retained
        .reservation
        .sources
        .iter()
        .position(|source| source.canister_id == canister_id)
        .ok_or_else(InternalError::invalid_input)?;
    let progress = &retained.progress[index];
    if matches!(progress, PoolImportSourceProgress::Ready(_)) {
        return Ok(retained);
    }
    let _execution = CanisterPoolImportOps::claim_execution(identity)?;
    require_destination(&retained.reservation)?;
    require_import_candidate(canister_id)?;
    // History precedes the final status sample. No await separates that sample's
    // validation from the next mutation's durable intent, including on recovery.
    let history = observe_issued_history(identity, canister_id, progress).await?;
    let observed = observe_source(identity, canister_id, &retained.reservation).await?;
    match progress {
        PoolImportSourceProgress::AwaitingHandoff
            if !retained.reservation.sources[index].stopped =>
        {
            let budget = budget(MgmtOps::stop_canister_call_cost(canister_id)?);
            let paid =
                CanisterPoolImportOps::issue_stop(identity, &observed, budget, IcOps::now_nanos())?;
            MgmtOps::stop_canister(canister_id).await?;
            complete_call(identity, paid)?;
        }
        PoolImportSourceProgress::AwaitingHandoff | PoolImportSourceProgress::Stopped => {
            issue_controllers(identity, &observed, &retained.reservation).await?;
        }
        PoolImportSourceProgress::ControllersIssued => {
            CanisterPoolImportOps::observe_controllers(
                identity,
                &observed,
                history.as_ref().ok_or_else(InternalError::invariant)?,
            )?;
            issue_uninstall(identity, &observed).await?;
        }
        PoolImportSourceProgress::StopIssued => {
            CanisterPoolImportOps::observe_stopped(identity, &observed)?;
            issue_controllers(identity, &observed, &retained.reservation).await?;
        }
        PoolImportSourceProgress::ControllersConfirmed => {
            issue_uninstall(identity, &observed).await?;
        }
        PoolImportSourceProgress::UninstallIssued => {
            CanisterPoolImportOps::observe_cleared(
                identity,
                &observed,
                history.as_ref().ok_or_else(InternalError::invariant)?,
                IcOps::now_nanos(),
            )?;
        }
        PoolImportSourceProgress::Ready(_) => return Err(InternalError::invariant()),
    }
    CanisterPoolImportOps::status(identity)
}

async fn observe_issued_history(
    identity: PoolImportIdentity,
    canister_id: Principal,
    progress: &PoolImportSourceProgress,
) -> Result<Option<PoolImportHistoryView>, InternalError> {
    if !matches!(
        progress,
        PoolImportSourceProgress::ControllersIssued | PoolImportSourceProgress::UninstallIssued
    ) {
        return Ok(None);
    }
    let paid = reserve_observation(identity, MgmtOps::canister_history_call_cost(canister_id)?)?;
    let history = CanisterPoolImportOps::observe_history(canister_id).await?;
    complete_call(identity, paid)?;
    Ok(Some(history))
}

async fn observe_source(
    identity: PoolImportIdentity,
    canister_id: Principal,
    reservation: &PoolImportReservation,
) -> Result<PoolImportObservationView, InternalError> {
    require_destination(reservation)?;
    if BuildNetworkOps::build_network() == Some(BuildNetwork::Ic) {
        let paid = reserve_observation(
            identity,
            NnsRegistryOps::subnet_lookup_call_cost(canister_id)?,
        )?;
        let subnet = NnsRegistryOps::get_subnet_for_canister(canister_id).await?;
        complete_call(identity, paid)?;
        if subnet != Some(reservation.subnet) {
            return Err(InternalError::conflict());
        }
    }
    let paid = reserve_observation(
        identity,
        MgmtOps::canister_inspection_reserve(canister_id)?.required_liquid_cycles,
    )?;
    let observed = CanisterPoolImportOps::observe_source(canister_id).await?;
    complete_call(identity, paid)?;
    require_destination(reservation)?;
    Ok(observed)
}

async fn issue_controllers(
    identity: PoolImportIdentity,
    observed: &PoolImportObservationView,
    reservation: &PoolImportReservation,
) -> Result<(), InternalError> {
    let args = UpdateSettingsArgs {
        canister_id: observed.canister_id,
        settings: CanisterSettings {
            controllers: Some(reservation.final_controllers.clone()),
            ..CanisterSettings::default()
        },
        sender_canister_version: None,
    };
    let budget = budget(MgmtOps::update_settings_call_cost(&args)?);
    let paid =
        CanisterPoolImportOps::issue_controllers(identity, observed, budget, IcOps::now_nanos())?;
    MgmtOps::update_settings(&args).await?;
    complete_call(identity, paid)
}

async fn issue_uninstall(
    identity: PoolImportIdentity,
    observed: &PoolImportObservationView,
) -> Result<(), InternalError> {
    let budget = budget(MgmtOps::uninstall_code_call_cost(observed.canister_id)?);
    let paid = CanisterPoolImportOps::issue_uninstall(identity, observed, budget)?;
    MgmtOps::uninstall_code(observed.canister_id).await?;
    complete_call(identity, paid)
}

/// Retain the final Root balance boundary before any host inventory publication.
pub async fn settle(identity: PoolImportIdentity) -> Result<PoolImportStatus, InternalError> {
    let retained = CanisterPoolImportOps::status(identity)?;
    if retained.root_receipt.is_some() {
        return Ok(retained);
    }
    if retained.phase != PoolImportPhase::Ready {
        return Err(InternalError::conflict());
    }
    let _execution = CanisterPoolImportOps::claim_execution(identity)?;
    require_destination(&retained.reservation)?;
    let root = IcOps::canister_self();
    let paid = reserve_observation(
        identity,
        MgmtOps::canister_inspection_reserve(root)?.required_liquid_cycles,
    )?;
    let observed = CanisterPoolImportOps::observe_source(root).await?;
    complete_call(identity, paid)?;
    require_destination(&retained.reservation)?;
    CanisterPoolImportOps::settle(identity, observed.cycles, observed.reserved_cycles)
}

/// Publish allocation release only after the host's durable local inventory handoff.
pub fn release(
    identity: PoolImportIdentity,
    publication_sha256: [u8; 32],
) -> Result<PoolImportStatus, InternalError> {
    let retained = CanisterPoolImportOps::status(identity)?;
    if !matches!(retained.phase, PoolImportPhase::Released { .. }) {
        require_destination(&retained.reservation)?;
    }
    CanisterPoolImportOps::release(identity, publication_sha256)
}

fn require_destination(request: &PoolImportReservation) -> Result<(), InternalError> {
    let current = context()?;
    let expected = (
        current.binding.fleet_subnet_root,
        current.binding.placement_subnet.into_principal(),
        current.root_authority_sha256,
        current
            .binding
            .authority
            .binding
            .root_controllers(current.binding.fleet_subnet_root),
    );
    let actual = (
        request.root,
        request.subnet,
        request.root_authority_sha256,
        request.final_controllers.clone(),
    );
    if actual != expected || request.root != IcOps::canister_self() {
        return Err(InternalError::conflict());
    }
    if request.minimum_root_cycles
        < current
            .binding
            .funding
            .root_funding
            .request_threshold
            .to_u128()
    {
        return Err(InternalError::resource_exhausted());
    }
    Ok(())
}

fn reserve_observation(
    identity: PoolImportIdentity,
    maximum_call_debit: u128,
) -> Result<PoolImportCallReservationView, InternalError> {
    CanisterPoolImportOps::reserve_paid_call(
        identity,
        maximum_call_debit,
        IcOps::canister_cycle_balance().to_u128(),
    )
}

fn budget(maximum_debit_cycles: u128) -> PoolImportCallBudgetView {
    PoolImportCallBudgetView {
        sender_canister_version: IcOps::canister_version(),
        maximum_debit_cycles,
        observed_root_cycles: IcOps::canister_cycle_balance().to_u128(),
    }
}

fn complete_call(
    identity: PoolImportIdentity,
    receipt: PoolImportCallReservationView,
) -> Result<(), InternalError> {
    CanisterPoolImportOps::complete_paid_call(
        identity,
        receipt,
        IcOps::canister_cycle_balance().to_u128(),
    )
}
