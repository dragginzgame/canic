//! Read-only Root acceptance and provisioning progress reconstruction and classification.
//!
//! Boundary: retained Coordinator state and Root responses are classified without committing a
//! Root call or mutating the Coordinator journal.

use super::*;

#[derive(Clone, Copy, Eq, PartialEq)]
pub(super) enum RootAcceptanceAdvance {
    Begin,
    Reconcile,
    Current,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub(super) enum RootProvisionAdvance {
    Begin,
    Reconcile,
    Current,
    Publish,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub(super) struct RootProvisioningCounts {
    reserved: u32,
    claimed: u32,
    installed: u32,
    registry_committed: u32,
}

impl RootProvisioningCounts {
    pub(super) const fn from_response(response: &RootComponentProvisioningStatusResponse) -> Self {
        Self {
            reserved: response.reserved_component_count,
            claimed: response.claimed_component_count,
            installed: response.installed_component_count,
            registry_committed: response.registry_committed_component_count,
        }
    }

    pub(super) const fn from_progress(progress: FleetComponentProvisioningRootProgress) -> Self {
        Self {
            reserved: progress.reserved_component_count,
            claimed: progress.claimed_component_count,
            installed: progress.installed_component_count,
            registry_committed: progress.registry_committed_component_count,
        }
    }

    pub(super) fn is_canonical(self, component_count: u32) -> bool {
        let counts_are_bounded = [
            self.reserved <= component_count,
            self.claimed <= component_count,
            self.installed <= component_count,
            self.registry_committed <= component_count,
        ]
        .into_iter()
        .all(|fact| fact);
        let phases_are_ordered = [
            stage_follows_complete_predecessor(self.claimed, self.reserved, component_count),
            stage_follows_complete_predecessor(self.installed, self.claimed, component_count),
            stage_follows_complete_predecessor(
                self.registry_committed,
                self.installed,
                component_count,
            ),
        ]
        .into_iter()
        .all(|fact| fact);
        counts_are_bounded && phases_are_ordered
    }

    pub(super) fn advances_one_step_to(self, next: Self, component_count: u32) -> bool {
        let states_are_canonical = [
            self.is_canonical(component_count),
            next.is_canonical(component_count),
        ]
        .into_iter()
        .all(|fact| fact);
        if !states_are_canonical {
            return false;
        }
        let reservation_advances = [
            self.claimed == 0,
            self.installed == 0,
            self.registry_committed == 0,
            next.claimed == 0,
            next.installed == 0,
            next.registry_committed == 0,
            self.reserved.checked_add(1) == Some(next.reserved),
        ]
        .into_iter()
        .all(|fact| fact);
        let claim_advances = [
            self.reserved == next.reserved,
            self.installed == 0,
            self.registry_committed == 0,
            next.installed == 0,
            next.registry_committed == 0,
            self.claimed.checked_add(1) == Some(next.claimed),
        ]
        .into_iter()
        .all(|fact| fact);
        let install_advances = [
            self.reserved == next.reserved,
            self.claimed == next.claimed,
            self.registry_committed == 0,
            next.registry_committed == 0,
            self.installed.checked_add(1) == Some(next.installed),
        ]
        .into_iter()
        .all(|fact| fact);
        let registry_advances = [
            self.reserved == next.reserved,
            self.claimed == next.claimed,
            self.installed == next.installed,
            self.registry_committed.checked_add(1) == Some(next.registry_committed),
        ]
        .into_iter()
        .all(|fact| fact);
        [
            reservation_advances,
            claim_advances,
            install_advances,
            registry_advances,
        ]
        .into_iter()
        .any(|advances| advances)
    }
}

const fn stage_follows_complete_predecessor(
    stage: u32,
    predecessor: u32,
    component_count: u32,
) -> bool {
    stage == 0 || predecessor == component_count
}

#[derive(Clone)]
pub(super) struct FleetComponentProvisioningRootAcceptanceProgress {
    pub(super) planned_at_ns: u64,
    pub(super) phase: FleetComponentProvisioningPhase,
    pub(super) acceptances: Vec<FleetComponentProvisioningRootAcceptanceRecord>,
    pub(super) accepted_root_count: u32,
    pub(super) root_batch_count: u32,
    pub(super) in_flight: Option<FleetComponentProvisioningRootAcceptanceIntentRecord>,
    pub(super) roots_accepted_at_ns: Option<u64>,
}

#[derive(Clone)]
pub(super) struct FleetComponentProvisioningRootProvisionProgress {
    pub(super) provisions: Vec<FleetComponentProvisioningRootProvisionRecord>,
    pub(super) provisioned_root_count: u32,
    pub(super) current: Option<FleetComponentProvisioningRootProvisionRecord>,
    pub(super) current_response: Option<RootComponentProvisioningStatusResponse>,
    pub(super) in_flight: Option<FleetComponentProvisioningRootProvisionIntentRecord>,
    pub(super) roots_accepted_at_ns: Option<u64>,
    pub(super) components_provisioned_at_ns: Option<u64>,
    pub(super) published_fleet_registry: Option<FleetRegistryVersion>,
    pub(super) service_topology_published_at_ns: Option<u64>,
}

pub(super) fn component_provisioning_root_acceptance_progress(
    record: &FleetComponentProvisioningRecord,
) -> Result<FleetComponentProvisioningRootAcceptanceProgress, InternalError> {
    let root_batch_count = u32::try_from(record.plan.batches.len())
        .map_err(|_| receipt_invariant("root batch count does not fit u32"))?;
    match &record.state {
        FleetComponentProvisioningStateRecord::Planned { planned_at_ns } => {
            planned_root_acceptance_progress(*planned_at_ns, root_batch_count)
        }
        FleetComponentProvisioningStateRecord::AcceptingRoots {
            planned_at_ns,
            acceptances,
            in_flight,
        } => root_acceptance_progress_from_parts(
            *planned_at_ns,
            FleetComponentProvisioningPhase::AcceptingRoots,
            acceptances,
            *in_flight,
            None,
            root_batch_count,
        ),
        state => {
            let authority = post_acceptance_authority(state);
            root_acceptance_progress_from_parts(
                authority.planned_at_ns,
                authority.phase,
                authority.acceptances,
                None,
                Some(authority.roots_accepted_at_ns),
                root_batch_count,
            )
        }
    }
}

struct PostAcceptanceAuthority<'a> {
    planned_at_ns: u64,
    phase: FleetComponentProvisioningPhase,
    acceptances: &'a [FleetComponentProvisioningRootAcceptanceRecord],
    roots_accepted_at_ns: u64,
}

fn post_acceptance_authority(
    state: &FleetComponentProvisioningStateRecord,
) -> PostAcceptanceAuthority<'_> {
    let (planned_at_ns, acceptances, roots_accepted_at_ns) = match state {
        FleetComponentProvisioningStateRecord::RootsAccepted {
            planned_at_ns,
            acceptances,
            roots_accepted_at_ns,
        }
        | FleetComponentProvisioningStateRecord::ProvisioningRoots {
            planned_at_ns,
            acceptances,
            roots_accepted_at_ns,
            ..
        }
        | FleetComponentProvisioningStateRecord::ComponentsProvisioned {
            planned_at_ns,
            acceptances,
            roots_accepted_at_ns,
            ..
        }
        | FleetComponentProvisioningStateRecord::ServiceTopologyPublished {
            planned_at_ns,
            acceptances,
            roots_accepted_at_ns,
            ..
        }
        | FleetComponentProvisioningStateRecord::ConfirmingDirectories {
            planned_at_ns,
            acceptances,
            roots_accepted_at_ns,
            ..
        }
        | FleetComponentProvisioningStateRecord::DirectoriesConfirmed {
            planned_at_ns,
            acceptances,
            roots_accepted_at_ns,
            ..
        }
        | FleetComponentProvisioningStateRecord::ActivatingRuntimes {
            planned_at_ns,
            acceptances,
            roots_accepted_at_ns,
            ..
        }
        | FleetComponentProvisioningStateRecord::RuntimesActivated {
            planned_at_ns,
            acceptances,
            roots_accepted_at_ns,
            ..
        } => (
            *planned_at_ns,
            acceptances.as_slice(),
            *roots_accepted_at_ns,
        ),
        _ => unreachable!("pre-acceptance states are handled by the caller"),
    };
    let phase = match state {
        FleetComponentProvisioningStateRecord::RootsAccepted { .. } => {
            FleetComponentProvisioningPhase::RootsAccepted
        }
        FleetComponentProvisioningStateRecord::ProvisioningRoots { .. } => {
            FleetComponentProvisioningPhase::ProvisioningRoots
        }
        FleetComponentProvisioningStateRecord::ComponentsProvisioned { .. } => {
            FleetComponentProvisioningPhase::ComponentsProvisioned
        }
        FleetComponentProvisioningStateRecord::ServiceTopologyPublished { .. } => {
            FleetComponentProvisioningPhase::ServiceTopologyPublished
        }
        FleetComponentProvisioningStateRecord::ConfirmingDirectories { .. } => {
            FleetComponentProvisioningPhase::ConfirmingDirectories
        }
        FleetComponentProvisioningStateRecord::DirectoriesConfirmed { .. } => {
            FleetComponentProvisioningPhase::DirectoriesConfirmed
        }
        FleetComponentProvisioningStateRecord::ActivatingRuntimes { .. } => {
            FleetComponentProvisioningPhase::ActivatingRuntimes
        }
        FleetComponentProvisioningStateRecord::RuntimesActivated { .. } => {
            FleetComponentProvisioningPhase::RuntimesActivated
        }
        _ => unreachable!("pre-acceptance states are handled by the caller"),
    };
    PostAcceptanceAuthority {
        planned_at_ns,
        phase,
        acceptances,
        roots_accepted_at_ns,
    }
}

fn root_acceptance_progress_from_parts(
    planned_at_ns: u64,
    phase: FleetComponentProvisioningPhase,
    acceptances: &[FleetComponentProvisioningRootAcceptanceRecord],
    in_flight: Option<FleetComponentProvisioningRootAcceptanceIntentRecord>,
    roots_accepted_at_ns: Option<u64>,
    root_batch_count: u32,
) -> Result<FleetComponentProvisioningRootAcceptanceProgress, InternalError> {
    let accepted_root_count = u32::try_from(acceptances.len())
        .map_err(|_| receipt_invariant("accepted root count does not fit u32"))?;
    Ok(FleetComponentProvisioningRootAcceptanceProgress {
        planned_at_ns,
        phase,
        acceptances: acceptances.to_vec(),
        accepted_root_count,
        root_batch_count,
        in_flight,
        roots_accepted_at_ns,
    })
}

fn planned_root_acceptance_progress(
    planned_at_ns: u64,
    root_batch_count: u32,
) -> Result<FleetComponentProvisioningRootAcceptanceProgress, InternalError> {
    root_acceptance_progress_from_parts(
        planned_at_ns,
        FleetComponentProvisioningPhase::Planned,
        &[],
        None,
        None,
        root_batch_count,
    )
}

pub(super) fn component_provisioning_root_provision_progress(
    record: &FleetComponentProvisioningRecord,
) -> Result<FleetComponentProvisioningRootProvisionProgress, InternalError> {
    match &record.state {
        FleetComponentProvisioningStateRecord::Planned { .. }
        | FleetComponentProvisioningStateRecord::AcceptingRoots { .. } => {
            Ok(empty_root_provision_progress())
        }
        FleetComponentProvisioningStateRecord::RootsAccepted {
            acceptances,
            roots_accepted_at_ns,
            ..
        } => Ok(accepted_root_provision_progress(
            acceptances,
            *roots_accepted_at_ns,
        )),
        FleetComponentProvisioningStateRecord::ProvisioningRoots {
            acceptances,
            roots_accepted_at_ns,
            provisions,
            current,
            in_flight,
            ..
        } => active_root_provision_progress(
            acceptances,
            *roots_accepted_at_ns,
            provisions,
            current.as_deref(),
            in_flight.as_ref(),
        ),
        FleetComponentProvisioningStateRecord::ComponentsProvisioned {
            roots_accepted_at_ns,
            provisions,
            components_provisioned_at_ns,
            ..
        } => terminal_root_provision_progress(
            provisions,
            *roots_accepted_at_ns,
            *components_provisioned_at_ns,
            None,
        ),
        FleetComponentProvisioningStateRecord::ServiceTopologyPublished {
            roots_accepted_at_ns,
            provisions,
            components_provisioned_at_ns,
            published_fleet_registry,
            service_topology_published_at_ns,
            ..
        }
        | FleetComponentProvisioningStateRecord::ConfirmingDirectories {
            roots_accepted_at_ns,
            provisions,
            components_provisioned_at_ns,
            published_fleet_registry,
            service_topology_published_at_ns,
            ..
        }
        | FleetComponentProvisioningStateRecord::DirectoriesConfirmed {
            roots_accepted_at_ns,
            provisions,
            components_provisioned_at_ns,
            published_fleet_registry,
            service_topology_published_at_ns,
            ..
        }
        | FleetComponentProvisioningStateRecord::ActivatingRuntimes {
            roots_accepted_at_ns,
            provisions,
            components_provisioned_at_ns,
            published_fleet_registry,
            service_topology_published_at_ns,
            ..
        }
        | FleetComponentProvisioningStateRecord::RuntimesActivated {
            roots_accepted_at_ns,
            provisions,
            components_provisioned_at_ns,
            published_fleet_registry,
            service_topology_published_at_ns,
            ..
        } => terminal_root_provision_progress(
            provisions,
            *roots_accepted_at_ns,
            *components_provisioned_at_ns,
            Some((published_fleet_registry, *service_topology_published_at_ns)),
        ),
    }
}

pub(super) fn classify_root_provision_advance(
    request: &FleetComponentProvisioningAdvanceRequest,
    progress: &FleetComponentProvisioningRootProvisionProgress,
) -> Result<RootProvisionAdvance, InternalError> {
    if request.expected_provisioned_root_count == progress.provisioned_root_count {
        let Some(current) = progress.current_response.as_ref() else {
            if request.expected_current_root.is_none()
                && progress.components_provisioned_at_ns.is_some()
            {
                return Ok(if progress.service_topology_published_at_ns.is_some() {
                    RootProvisionAdvance::Current
                } else {
                    RootProvisionAdvance::Publish
                });
            }
            return Err(InternalError::conflict());
        };
        let actual = root_provisioning_progress(current);
        if request.expected_current_root.as_ref() == Some(&actual) {
            return Ok(if progress.in_flight.is_some() {
                RootProvisionAdvance::Reconcile
            } else {
                RootProvisionAdvance::Begin
            });
        }
        if let Some(expected) = request.expected_current_root
            && expected.fleet_subnet_root == actual.fleet_subnet_root
            && expected.component_count == actual.component_count
            && RootProvisioningCounts::from_progress(expected).advances_one_step_to(
                RootProvisioningCounts::from_progress(actual),
                actual.component_count,
            )
        {
            return Ok(RootProvisionAdvance::Current);
        }
        return Err(InternalError::conflict());
    }
    if request.expected_provisioned_root_count.checked_add(1)
        == Some(progress.provisioned_root_count)
    {
        let index = usize::try_from(request.expected_provisioned_root_count)
            .map_err(|_| InternalError::resource_exhausted())?;
        let provision = progress.provisions.get(index).ok_or_else(|| {
            receipt_invariant("terminal root provisioning receipt is absent at its cursor")
        })?;
        if request.expected_current_root.as_ref()
            == Some(&root_provisioning_progress(&provision.response))
        {
            return Ok(RootProvisionAdvance::Current);
        }
    }
    Err(InternalError::conflict())
}

pub(super) fn classify_root_acceptance_advance(
    request: &FleetComponentProvisioningAdvanceRequest,
    progress: &FleetComponentProvisioningRootAcceptanceProgress,
) -> Result<RootAcceptanceAdvance, InternalError> {
    if request.expected_accepted_root_count == progress.accepted_root_count {
        if progress.accepted_root_count == progress.root_batch_count
            && progress.roots_accepted_at_ns.is_some()
        {
            return Ok(RootAcceptanceAdvance::Current);
        }
        return Ok(if progress.in_flight.is_some() {
            RootAcceptanceAdvance::Reconcile
        } else {
            RootAcceptanceAdvance::Begin
        });
    }
    if request.expected_accepted_root_count.checked_add(1) == Some(progress.accepted_root_count) {
        return Ok(RootAcceptanceAdvance::Current);
    }
    Err(InternalError::conflict())
}

const fn empty_root_provision_progress() -> FleetComponentProvisioningRootProvisionProgress {
    FleetComponentProvisioningRootProvisionProgress {
        provisions: Vec::new(),
        provisioned_root_count: 0,
        current: None,
        current_response: None,
        in_flight: None,
        roots_accepted_at_ns: None,
        components_provisioned_at_ns: None,
        published_fleet_registry: None,
        service_topology_published_at_ns: None,
    }
}

fn accepted_root_provision_progress(
    acceptances: &[FleetComponentProvisioningRootAcceptanceRecord],
    roots_accepted_at_ns: u64,
) -> FleetComponentProvisioningRootProvisionProgress {
    FleetComponentProvisioningRootProvisionProgress {
        current_response: acceptances.first().map(|record| record.response.clone()),
        roots_accepted_at_ns: Some(roots_accepted_at_ns),
        ..empty_root_provision_progress()
    }
}

fn active_root_provision_progress(
    acceptances: &[FleetComponentProvisioningRootAcceptanceRecord],
    roots_accepted_at_ns: u64,
    provisions: &[FleetComponentProvisioningRootProvisionRecord],
    current: Option<&FleetComponentProvisioningRootProvisionRecord>,
    in_flight: Option<&FleetComponentProvisioningRootProvisionIntentRecord>,
) -> Result<FleetComponentProvisioningRootProvisionProgress, InternalError> {
    let provisioned_root_count = u32::try_from(provisions.len())
        .map_err(|_| receipt_invariant("provisioned root count does not fit u32"))?;
    let current_response = current.map_or_else(
        || {
            acceptances
                .get(provisions.len())
                .map(|record| record.response.clone())
        },
        |record| Some(record.response.clone()),
    );
    Ok(FleetComponentProvisioningRootProvisionProgress {
        provisions: provisions.to_vec(),
        provisioned_root_count,
        current: current.cloned(),
        current_response,
        in_flight: in_flight.cloned(),
        roots_accepted_at_ns: Some(roots_accepted_at_ns),
        components_provisioned_at_ns: None,
        published_fleet_registry: None,
        service_topology_published_at_ns: None,
    })
}

fn terminal_root_provision_progress(
    provisions: &[FleetComponentProvisioningRootProvisionRecord],
    roots_accepted_at_ns: u64,
    components_provisioned_at_ns: u64,
    publication: Option<(&FleetRegistryVersion, u64)>,
) -> Result<FleetComponentProvisioningRootProvisionProgress, InternalError> {
    let provisioned_root_count = u32::try_from(provisions.len())
        .map_err(|_| receipt_invariant("provisioned root count does not fit u32"))?;
    Ok(FleetComponentProvisioningRootProvisionProgress {
        provisions: provisions.to_vec(),
        provisioned_root_count,
        current: None,
        current_response: None,
        in_flight: None,
        roots_accepted_at_ns: Some(roots_accepted_at_ns),
        components_provisioned_at_ns: Some(components_provisioned_at_ns),
        published_fleet_registry: publication.map(|(version, _)| version.clone()),
        service_topology_published_at_ns: publication.map(|(_, published_at_ns)| published_at_ns),
    })
}
