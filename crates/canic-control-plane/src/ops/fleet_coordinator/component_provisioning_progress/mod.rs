//! Read-only Directory-confirmation and runtime-activation progress reconstruction.
//!
//! Boundary: retained Coordinator state is projected without choosing or committing an advance.

use super::*;

#[derive(Clone, Copy, Eq, PartialEq)]
pub(super) enum DirectoryConfirmationAdvance {
    Begin,
    Reconcile,
    Current,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub(super) enum RuntimeActivationAdvance {
    Begin,
    Reconcile,
    Current,
}

#[derive(Clone)]
pub(super) struct FleetComponentDirectoryConfirmationProgress {
    pub(super) planned_at_ns: u64,
    pub(super) acceptances: Vec<FleetComponentProvisioningRootAcceptanceRecord>,
    pub(super) roots_accepted_at_ns: u64,
    pub(super) provisions: Vec<FleetComponentProvisioningRootProvisionRecord>,
    pub(super) components_provisioned_at_ns: u64,
    pub(super) published_fleet_registry: FleetRegistryVersion,
    pub(super) service_topology_published_at_ns: u64,
    pub(super) confirmations: Vec<FleetComponentDirectoryConfirmationRecord>,
    pub(super) confirmed_root_count: u32,
    pub(super) confirmation_root_count: u32,
    pub(super) current: Option<FleetComponentDirectoryConfirmationRecord>,
    pub(super) in_flight: Option<FleetComponentDirectoryConfirmationIntentRecord>,
    pub(super) complete: bool,
}

#[derive(Clone)]
pub(super) struct FleetComponentRuntimeActivationProgress {
    pub(super) planned_at_ns: u64,
    pub(super) acceptances: Vec<FleetComponentProvisioningRootAcceptanceRecord>,
    pub(super) roots_accepted_at_ns: u64,
    pub(super) provisions: Vec<FleetComponentProvisioningRootProvisionRecord>,
    pub(super) components_provisioned_at_ns: u64,
    pub(super) published_fleet_registry: FleetRegistryVersion,
    pub(super) service_topology_published_at_ns: u64,
    pub(super) confirmations: Vec<FleetComponentDirectoryConfirmationRecord>,
    pub(super) directories_confirmed_at_ns: u64,
    pub(super) activations: Vec<FleetComponentRuntimeActivationRecord>,
    pub(super) activated_root_count: u32,
    pub(super) activation_root_count: u32,
    pub(super) current: Option<FleetComponentRuntimeActivationRecord>,
    pub(super) in_flight: Option<FleetComponentRuntimeActivationIntentRecord>,
    pub(super) runtimes_activated_at_ns: Option<u64>,
    pub(super) complete: bool,
}

pub(super) fn component_directory_confirmation_progress(
    record: &FleetComponentProvisioningRecord,
) -> Result<FleetComponentDirectoryConfirmationProgress, InternalError> {
    let confirmation_root_count = u32::try_from(record.plan.directory_confirmation_roots.len())
        .map_err(|_| receipt_invariant("Directory confirmation root count does not fit u32"))?;
    let progress = match &record.state {
        FleetComponentProvisioningStateRecord::ServiceTopologyPublished {
            planned_at_ns,
            acceptances,
            roots_accepted_at_ns,
            provisions,
            components_provisioned_at_ns,
            published_fleet_registry,
            service_topology_published_at_ns,
        } => FleetComponentDirectoryConfirmationProgress {
            planned_at_ns: *planned_at_ns,
            acceptances: acceptances.clone(),
            roots_accepted_at_ns: *roots_accepted_at_ns,
            provisions: provisions.clone(),
            components_provisioned_at_ns: *components_provisioned_at_ns,
            published_fleet_registry: published_fleet_registry.clone(),
            service_topology_published_at_ns: *service_topology_published_at_ns,
            confirmations: vec![],
            confirmed_root_count: 0,
            confirmation_root_count,
            current: None,
            in_flight: None,
            complete: false,
        },
        FleetComponentProvisioningStateRecord::ConfirmingDirectories {
            planned_at_ns,
            acceptances,
            roots_accepted_at_ns,
            provisions,
            components_provisioned_at_ns,
            published_fleet_registry,
            service_topology_published_at_ns,
            confirmations,
            current,
            in_flight,
        } => FleetComponentDirectoryConfirmationProgress {
            planned_at_ns: *planned_at_ns,
            acceptances: acceptances.clone(),
            roots_accepted_at_ns: *roots_accepted_at_ns,
            provisions: provisions.clone(),
            components_provisioned_at_ns: *components_provisioned_at_ns,
            published_fleet_registry: published_fleet_registry.clone(),
            service_topology_published_at_ns: *service_topology_published_at_ns,
            confirmations: confirmations.clone(),
            confirmed_root_count: u32::try_from(confirmations.len())
                .map_err(|_| receipt_invariant("Directory confirmation count does not fit u32"))?,
            confirmation_root_count,
            current: current.as_deref().cloned(),
            in_flight: in_flight.as_deref().cloned(),
            complete: false,
        },
        FleetComponentProvisioningStateRecord::DirectoriesConfirmed {
            planned_at_ns,
            acceptances,
            roots_accepted_at_ns,
            provisions,
            components_provisioned_at_ns,
            published_fleet_registry,
            service_topology_published_at_ns,
            confirmations,
            ..
        } => FleetComponentDirectoryConfirmationProgress {
            planned_at_ns: *planned_at_ns,
            acceptances: acceptances.clone(),
            roots_accepted_at_ns: *roots_accepted_at_ns,
            provisions: provisions.clone(),
            components_provisioned_at_ns: *components_provisioned_at_ns,
            published_fleet_registry: published_fleet_registry.clone(),
            service_topology_published_at_ns: *service_topology_published_at_ns,
            confirmations: confirmations.clone(),
            confirmed_root_count: u32::try_from(confirmations.len())
                .map_err(|_| receipt_invariant("Directory confirmation count does not fit u32"))?,
            confirmation_root_count,
            current: None,
            in_flight: None,
            complete: true,
        },
        state @ (FleetComponentProvisioningStateRecord::ActivatingRuntimes { .. }
        | FleetComponentProvisioningStateRecord::RuntimesActivated { .. }) => {
            terminal_downstream_directory_progress(state, confirmation_root_count)?
        }
        _ => {
            return Err(InternalError::conflict());
        }
    };
    Ok(progress)
}

fn terminal_downstream_directory_progress(
    state: &FleetComponentProvisioningStateRecord,
    confirmation_root_count: u32,
) -> Result<FleetComponentDirectoryConfirmationProgress, InternalError> {
    let (
        planned_at_ns,
        acceptances,
        roots_accepted_at_ns,
        provisions,
        components_provisioned_at_ns,
        published_fleet_registry,
        service_topology_published_at_ns,
        confirmations,
    ) = match state {
        FleetComponentProvisioningStateRecord::ActivatingRuntimes {
            planned_at_ns,
            acceptances,
            roots_accepted_at_ns,
            provisions,
            components_provisioned_at_ns,
            published_fleet_registry,
            service_topology_published_at_ns,
            confirmations,
            ..
        }
        | FleetComponentProvisioningStateRecord::RuntimesActivated {
            planned_at_ns,
            acceptances,
            roots_accepted_at_ns,
            provisions,
            components_provisioned_at_ns,
            published_fleet_registry,
            service_topology_published_at_ns,
            confirmations,
            ..
        } => (
            *planned_at_ns,
            acceptances,
            *roots_accepted_at_ns,
            provisions,
            *components_provisioned_at_ns,
            published_fleet_registry,
            *service_topology_published_at_ns,
            confirmations,
        ),
        _ => unreachable!("only downstream Directory states delegate here"),
    };
    Ok(FleetComponentDirectoryConfirmationProgress {
        planned_at_ns,
        acceptances: acceptances.clone(),
        roots_accepted_at_ns,
        provisions: provisions.clone(),
        components_provisioned_at_ns,
        published_fleet_registry: published_fleet_registry.clone(),
        service_topology_published_at_ns,
        confirmations: confirmations.clone(),
        confirmed_root_count: u32::try_from(confirmations.len())
            .map_err(|_| receipt_invariant("Directory confirmation count does not fit u32"))?,
        confirmation_root_count,
        current: None,
        in_flight: None,
        complete: true,
    })
}

struct RuntimeActivationAuthority {
    planned_at_ns: u64,
    acceptances: Vec<FleetComponentProvisioningRootAcceptanceRecord>,
    roots_accepted_at_ns: u64,
    provisions: Vec<FleetComponentProvisioningRootProvisionRecord>,
    components_provisioned_at_ns: u64,
    published_fleet_registry: FleetRegistryVersion,
    service_topology_published_at_ns: u64,
    confirmations: Vec<FleetComponentDirectoryConfirmationRecord>,
    directories_confirmed_at_ns: u64,
}

fn runtime_activation_authority(
    record: &FleetComponentProvisioningRecord,
) -> Result<RuntimeActivationAuthority, InternalError> {
    let (
        planned_at_ns,
        acceptances,
        roots_accepted_at_ns,
        provisions,
        components_provisioned_at_ns,
        published_fleet_registry,
        service_topology_published_at_ns,
        confirmations,
        directories_confirmed_at_ns,
    ) = match &record.state {
        FleetComponentProvisioningStateRecord::DirectoriesConfirmed {
            planned_at_ns,
            acceptances,
            roots_accepted_at_ns,
            provisions,
            components_provisioned_at_ns,
            published_fleet_registry,
            service_topology_published_at_ns,
            confirmations,
            directories_confirmed_at_ns,
        }
        | FleetComponentProvisioningStateRecord::ActivatingRuntimes {
            planned_at_ns,
            acceptances,
            roots_accepted_at_ns,
            provisions,
            components_provisioned_at_ns,
            published_fleet_registry,
            service_topology_published_at_ns,
            confirmations,
            directories_confirmed_at_ns,
            ..
        }
        | FleetComponentProvisioningStateRecord::RuntimesActivated {
            planned_at_ns,
            acceptances,
            roots_accepted_at_ns,
            provisions,
            components_provisioned_at_ns,
            published_fleet_registry,
            service_topology_published_at_ns,
            confirmations,
            directories_confirmed_at_ns,
            ..
        } => (
            *planned_at_ns,
            acceptances.clone(),
            *roots_accepted_at_ns,
            provisions.clone(),
            *components_provisioned_at_ns,
            published_fleet_registry.clone(),
            *service_topology_published_at_ns,
            confirmations.clone(),
            *directories_confirmed_at_ns,
        ),
        _ => {
            return Err(InternalError::conflict());
        }
    };
    Ok(RuntimeActivationAuthority {
        planned_at_ns,
        acceptances,
        roots_accepted_at_ns,
        provisions,
        components_provisioned_at_ns,
        published_fleet_registry,
        service_topology_published_at_ns,
        confirmations,
        directories_confirmed_at_ns,
    })
}

pub(super) fn component_runtime_activation_progress(
    record: &FleetComponentProvisioningRecord,
) -> Result<FleetComponentRuntimeActivationProgress, InternalError> {
    let authority = runtime_activation_authority(record)?;
    let activation_root_count = u32::try_from(record.plan.batches.len())
        .map_err(|_| receipt_invariant("runtime activation root count does not fit u32"))?;
    let (activations, current, in_flight, runtimes_activated_at_ns, complete) = match &record.state
    {
        FleetComponentProvisioningStateRecord::DirectoriesConfirmed { .. } => {
            (vec![], None, None, None, false)
        }
        FleetComponentProvisioningStateRecord::ActivatingRuntimes {
            activations,
            current,
            in_flight,
            ..
        } => (
            activations.clone(),
            current.as_deref().copied(),
            *in_flight,
            None,
            false,
        ),
        FleetComponentProvisioningStateRecord::RuntimesActivated {
            activations,
            runtimes_activated_at_ns,
            ..
        } => (
            activations.clone(),
            None,
            None,
            Some(*runtimes_activated_at_ns),
            true,
        ),
        _ => unreachable!("runtime activation authority rejected earlier phases"),
    };
    let activated_root_count = u32::try_from(activations.len())
        .map_err(|_| receipt_invariant("runtime-activated root count does not fit u32"))?;
    Ok(FleetComponentRuntimeActivationProgress {
        planned_at_ns: authority.planned_at_ns,
        acceptances: authority.acceptances,
        roots_accepted_at_ns: authority.roots_accepted_at_ns,
        provisions: authority.provisions,
        components_provisioned_at_ns: authority.components_provisioned_at_ns,
        published_fleet_registry: authority.published_fleet_registry,
        service_topology_published_at_ns: authority.service_topology_published_at_ns,
        confirmations: authority.confirmations,
        directories_confirmed_at_ns: authority.directories_confirmed_at_ns,
        activations,
        activated_root_count,
        activation_root_count,
        current,
        in_flight,
        runtimes_activated_at_ns,
        complete,
    })
}

pub(super) fn classify_runtime_activation_advance(
    request: &FleetComponentProvisioningAdvanceRequest,
    progress: &FleetComponentRuntimeActivationProgress,
) -> Result<RuntimeActivationAdvance, InternalError> {
    if progress.complete {
        return if runtime_activation_request_is_current(request, progress)
            || terminal_runtime_activation_replay(request, progress)?
        {
            Ok(RuntimeActivationAdvance::Current)
        } else {
            Err(InternalError::conflict())
        };
    }
    if request.expected_runtime_activated_root_count < progress.activated_root_count {
        return if terminal_runtime_activation_replay(request, progress)? {
            Ok(RuntimeActivationAdvance::Current)
        } else {
            Err(InternalError::conflict())
        };
    }
    if request.expected_runtime_activated_root_count != progress.activated_root_count {
        return Err(InternalError::conflict());
    }
    let actual = progress.current.map(|record| record.progress);
    if request.expected_current_activation != actual {
        let replays_last = request
            .expected_current_activation
            .zip(actual)
            .is_some_and(|(expected, actual)| activation_progress_advances(expected, actual));
        let replays_first = request.expected_current_activation.is_none()
            && actual.is_some_and(first_component_activation_progress);
        return if replays_last || replays_first {
            Ok(RuntimeActivationAdvance::Current)
        } else {
            Err(InternalError::conflict())
        };
    }
    if progress.in_flight.is_some() {
        Ok(RuntimeActivationAdvance::Reconcile)
    } else {
        Ok(RuntimeActivationAdvance::Begin)
    }
}

pub(super) fn first_component_activation_progress(
    actual: FleetComponentActivationRootProgress,
) -> bool {
    activation_progress_advances(
        FleetComponentActivationRootProgress {
            fleet_subnet_root: actual.fleet_subnet_root,
            component_count: actual.component_count,
            activated_component_count: 0,
            root_runtime_active: false,
        },
        actual,
    )
}

const fn runtime_activation_request_is_current(
    request: &FleetComponentProvisioningAdvanceRequest,
    progress: &FleetComponentRuntimeActivationProgress,
) -> bool {
    request.expected_runtime_activated_root_count == progress.activated_root_count
        && request.expected_current_activation.is_none()
}

fn terminal_runtime_activation_replay(
    request: &FleetComponentProvisioningAdvanceRequest,
    progress: &FleetComponentRuntimeActivationProgress,
) -> Result<bool, InternalError> {
    if request.expected_runtime_activated_root_count.checked_add(1)
        != Some(progress.activated_root_count)
    {
        return Ok(false);
    }
    let terminal = progress
        .activations
        .last()
        .ok_or_else(|| receipt_invariant("terminal runtime activation lacks a root receipt"))?;
    Ok(request.expected_current_activation.map_or_else(
        || first_component_activation_progress(terminal.progress),
        |expected| activation_progress_advances(expected, terminal.progress),
    ))
}

pub(super) fn activation_progress_advances(
    expected: FleetComponentActivationRootProgress,
    actual: FleetComponentActivationRootProgress,
) -> bool {
    let subject_is_exact = expected.fleet_subnet_root == actual.fleet_subnet_root
        && expected.component_count == actual.component_count;
    let shapes_are_valid = runtime_activation_progress_shape_is_valid(expected)
        && runtime_activation_progress_shape_is_valid(actual);
    if !subject_is_exact || !shapes_are_valid {
        return false;
    }
    let progression_is_monotonic = !expected.root_runtime_active
        && actual.activated_component_count >= expected.activated_component_count;
    if !progression_is_monotonic {
        return false;
    }
    let component_advances = actual.activated_component_count > expected.activated_component_count;
    let root_advances = actual.root_runtime_active;
    component_advances || root_advances
}

const fn runtime_activation_progress_shape_is_valid(
    progress: FleetComponentActivationRootProgress,
) -> bool {
    progress.activated_component_count <= progress.component_count
        && (!progress.root_runtime_active
            || progress.activated_component_count == progress.component_count)
}

pub(super) fn classify_directory_confirmation_advance(
    request: &FleetComponentProvisioningAdvanceRequest,
    progress: &FleetComponentDirectoryConfirmationProgress,
) -> Result<DirectoryConfirmationAdvance, InternalError> {
    if progress.complete {
        let current_is_exact = request.expected_directory_confirmed_root_count
            == progress.confirmed_root_count
            && request.expected_current_synchronization.is_none()
            && request.expected_current_publication.is_none();
        let replays_terminal_call = terminal_directory_confirmation_replay(request, progress)?;
        return if current_is_exact || replays_terminal_call {
            Ok(DirectoryConfirmationAdvance::Current)
        } else {
            Err(InternalError::conflict())
        };
    }
    if request.expected_directory_confirmed_root_count < progress.confirmed_root_count {
        return if request
            .expected_directory_confirmed_root_count
            .checked_add(1)
            == Some(progress.confirmed_root_count)
        {
            Ok(DirectoryConfirmationAdvance::Current)
        } else {
            Err(InternalError::conflict())
        };
    }
    if request.expected_directory_confirmed_root_count != progress.confirmed_root_count {
        return Err(InternalError::conflict());
    }
    let actual_synchronization = progress
        .current
        .as_ref()
        .and_then(confirmation_synchronization_progress);
    if request.expected_current_synchronization != actual_synchronization {
        if synchronization_progress_replays(
            request.expected_current_synchronization,
            actual_synchronization,
        ) {
            return Ok(DirectoryConfirmationAdvance::Current);
        }
        return Err(InternalError::conflict());
    }
    let actual_current = progress
        .current
        .as_ref()
        .and_then(confirmation_publication_response)
        .map(root_publication_progress);
    if request.expected_current_publication != actual_current {
        if publication_progress_replays(request.expected_current_publication, actual_current) {
            return Ok(DirectoryConfirmationAdvance::Current);
        }
        return Err(InternalError::conflict());
    }
    if progress.in_flight.is_some() {
        Ok(DirectoryConfirmationAdvance::Reconcile)
    } else {
        Ok(DirectoryConfirmationAdvance::Begin)
    }
}

fn terminal_directory_confirmation_replay(
    request: &FleetComponentProvisioningAdvanceRequest,
    progress: &FleetComponentDirectoryConfirmationProgress,
) -> Result<bool, InternalError> {
    if request
        .expected_directory_confirmed_root_count
        .checked_add(1)
        != Some(progress.confirmed_root_count)
    {
        return Ok(false);
    }
    let terminal = progress
        .confirmations
        .last()
        .ok_or_else(|| receipt_invariant("terminal Directory confirmation lacks a root receipt"))?;
    let terminal_progress =
        confirmation_publication_response(terminal).map(root_publication_progress);
    let terminal_synchronization = confirmation_synchronization_progress(terminal);
    let synchronization_replays = request.expected_current_synchronization
        == terminal_synchronization
        || synchronization_progress_replays(
            request.expected_current_synchronization,
            terminal_synchronization,
        );
    let publication_replays = request.expected_current_publication == terminal_progress
        || publication_progress_replays(request.expected_current_publication, terminal_progress);
    Ok(synchronization_replays && publication_replays)
}

const fn root_synchronization_progress(
    response: &RootComponentDirectorySynchronizationResponse,
) -> FleetComponentSynchronizationRootProgress {
    FleetComponentSynchronizationRootProgress {
        fleet_subnet_root: response.fleet_subnet_root,
        affected_component_count: response.affected_component_count,
        synchronized_component_count: response.synchronized_component_count,
        complete: response.complete,
    }
}

pub(super) fn confirmation_synchronization_progress(
    confirmation: &FleetComponentDirectoryConfirmationRecord,
) -> Option<FleetComponentSynchronizationRootProgress> {
    match confirmation {
        FleetComponentDirectoryConfirmationRecord::FreshPublication { .. } => None,
        FleetComponentDirectoryConfirmationRecord::ScaleOut {
            synchronization, ..
        } => Some(root_synchronization_progress(synchronization)),
    }
}

fn synchronization_progress_replays(
    expected: Option<FleetComponentSynchronizationRootProgress>,
    actual: Option<FleetComponentSynchronizationRootProgress>,
) -> bool {
    let Some(actual) = actual else {
        return false;
    };
    let Some(expected) = expected else {
        return actual.synchronized_component_count <= 1;
    };
    if expected.fleet_subnet_root != actual.fleet_subnet_root
        || expected.affected_component_count != actual.affected_component_count
    {
        return false;
    }
    let component_advances = !expected.complete
        && expected.synchronized_component_count.checked_add(1)
            == Some(actual.synchronized_component_count);
    let terminal_advances = !expected.complete
        && actual.complete
        && expected.synchronized_component_count == actual.synchronized_component_count;
    component_advances || terminal_advances
}

pub(super) fn publication_progress_replays(
    expected: Option<FleetComponentPublicationRootProgress>,
    actual: Option<FleetComponentPublicationRootProgress>,
) -> bool {
    let Some(actual) = actual else {
        return false;
    };
    expected.map_or_else(
        || publication_progress_shape_is_valid(actual),
        |expected| publication_progress_advances(expected, actual),
    )
}

pub(super) fn publication_progress_advances(
    expected: FleetComponentPublicationRootProgress,
    actual: FleetComponentPublicationRootProgress,
) -> bool {
    let subject_is_exact = expected.fleet_subnet_root == actual.fleet_subnet_root
        && expected.component_count == actual.component_count;
    let shapes_are_valid = publication_progress_shape_is_valid(expected)
        && publication_progress_shape_is_valid(actual);
    subject_is_exact
        && shapes_are_valid
        && actual.published_component_count > expected.published_component_count
}

const fn publication_progress_shape_is_valid(
    progress: FleetComponentPublicationRootProgress,
) -> bool {
    progress.published_component_count <= progress.component_count
}

pub(super) const fn root_publication_progress(
    response: &RootComponentProvisioningStatusResponse,
) -> FleetComponentPublicationRootProgress {
    FleetComponentPublicationRootProgress {
        fleet_subnet_root: response.fleet_subnet_root,
        component_count: response.component_count,
        published_component_count: response.published_component_count,
    }
}

pub(super) const fn root_provisioning_progress(
    response: &RootComponentProvisioningStatusResponse,
) -> FleetComponentProvisioningRootProgress {
    FleetComponentProvisioningRootProgress {
        fleet_subnet_root: response.fleet_subnet_root,
        component_count: response.component_count,
        reserved_component_count: response.reserved_component_count,
        claimed_component_count: response.claimed_component_count,
        installed_component_count: response.installed_component_count,
        registry_committed_component_count: response.registry_committed_component_count,
    }
}
