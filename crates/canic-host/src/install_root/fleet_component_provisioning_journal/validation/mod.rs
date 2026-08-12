//! Module: install_root::fleet_component_provisioning_journal::validation
//!
//! Responsibility: validate frozen provisioning authority, progress and terminal evidence.
//! Does not own: document schema, filesystem persistence, phase mutation, or remote effects.
//! Boundary: every loaded or proposed journal must satisfy these checks before it is trusted.

use super::model::{
    FleetComponentProvisioningInstallJournal, FleetComponentProvisioningInstallJournalError,
    FleetComponentProvisioningInstallPhase, JOURNAL_SCHEMA_VERSION,
    PlanFleetComponentProvisioningInstallRequest,
};
use crate::fleet_catalog::FleetCatalogEntryV1;
use std::path::Path;

use candid::Principal;
use canic_core::dto::component_provisioning::{
    FleetComponentProvisioningAdvanceRequest, FleetComponentProvisioningPhase,
    FleetComponentProvisioningStatusResponse,
};

pub(super) fn planned_journal(
    path: &Path,
    request: PlanFleetComponentProvisioningInstallRequest<'_>,
) -> Result<FleetComponentProvisioningInstallJournal, FleetComponentProvisioningInstallJournalError>
{
    if request.coordinator == Principal::anonymous()
        || request.coordinator == Principal::management_canister()
    {
        return Err(invalid(path, "Coordinator principal is invalid"));
    }
    if request.environment.is_empty() {
        return Err(invalid(path, "Fleet installation environment is empty"));
    }
    let journal = FleetComponentProvisioningInstallJournal {
        schema_version: JOURNAL_SCHEMA_VERSION,
        sequence: 0,
        phase: FleetComponentProvisioningInstallPhase::Planned,
        fleet_install_plan_digest: request.fleet_install_plan.digest,
        coordinator: request.coordinator,
        fleet_name: request.fleet_name,
        environment: request.environment,
        prepare_request: request.compiled.prepare_request,
        plan_hash: request.compiled.plan_hash,
        last_status: None,
        advance_request: None,
        catalog_entry: None,
        catalog_hash: None,
    };
    validate_journal(path, &journal)?;
    Ok(journal)
}

pub(super) const fn advance_request(
    status: &FleetComponentProvisioningStatusResponse,
) -> FleetComponentProvisioningAdvanceRequest {
    FleetComponentProvisioningAdvanceRequest {
        operation_id: status.operation_id,
        plan_hash: status.plan_hash,
        expected_phase: status.phase,
        expected_accepted_root_count: status.accepted_root_count,
        expected_provisioned_root_count: status.provisioned_root_count,
        expected_current_root: status.current_root,
        expected_directory_confirmed_root_count: status.directory_confirmed_root_count,
        expected_current_synchronization: status.current_synchronization,
        expected_current_publication: status.current_publication,
        expected_runtime_activated_root_count: status.runtime_activated_root_count,
        expected_current_activation: status.current_activation,
    }
}

pub(super) fn validate_advance_request(
    path: &Path,
    request: &FleetComponentProvisioningAdvanceRequest,
    status: &FleetComponentProvisioningStatusResponse,
) -> Result<(), FleetComponentProvisioningInstallJournalError> {
    if request != &advance_request(status) {
        return Err(invalid(
            path,
            "durable provisioning advance request differs from the preceding Coordinator status",
        ));
    }
    Ok(())
}

pub(super) fn validate_status_progress(
    path: &Path,
    journal: &FleetComponentProvisioningInstallJournal,
    previous: &FleetComponentProvisioningStatusResponse,
    status: &FleetComponentProvisioningStatusResponse,
) -> Result<(), FleetComponentProvisioningInstallJournalError> {
    validate_status_identity(path, journal, status)?;
    let previous_progress = StatusProgress::from_status(previous);
    let observed_progress = StatusProgress::from_status(status);
    if observed_progress.has_regressed_from(previous_progress) {
        return Err(invalid(path, "Coordinator provisioning status regressed"));
    }
    Ok(())
}

pub(super) fn validate_status_identity(
    path: &Path,
    journal: &FleetComponentProvisioningInstallJournal,
    status: &FleetComponentProvisioningStatusResponse,
) -> Result<(), FleetComponentProvisioningInstallJournalError> {
    let plan = &journal.prepare_request.plan;
    let expected_authority = ProvisioningStatusAuthority::from_journal(journal);
    let observed_authority = ProvisioningStatusAuthority::from_status(status);
    let identity_matches = observed_authority == expected_authority;
    let observed_cardinality = StatusCardinality::from_status(status);
    let expected_cardinality = StatusCardinality::from_plan(path, plan)?;
    let cardinality_matches = observed_cardinality == expected_cardinality;
    let counts_are_bounded = [
        status.accepted_root_count <= status.root_batch_count,
        status.provisioned_root_count <= status.root_batch_count,
        status.directory_confirmed_root_count <= status.directory_confirmation_root_count,
        status.runtime_activated_root_count <= status.directory_confirmation_root_count,
    ]
    .into_iter()
    .all(std::convert::identity);
    let status_is_valid = [identity_matches, cardinality_matches, counts_are_bounded]
        .into_iter()
        .all(std::convert::identity);
    if !status_is_valid {
        return Err(invalid(
            path,
            "Coordinator status differs from frozen provisioning authority",
        ));
    }
    Ok(())
}

pub(super) fn validate_terminal_status(
    path: &Path,
    journal: &FleetComponentProvisioningInstallJournal,
    status: &FleetComponentProvisioningStatusResponse,
) -> Result<(), FleetComponentProvisioningInstallJournalError> {
    validate_status_identity(path, journal, status)?;
    let root_work_complete = [
        status.accepted_root_count == status.root_batch_count,
        status.provisioned_root_count == status.root_batch_count,
    ]
    .into_iter()
    .all(std::convert::identity);
    let fleet_barriers_complete = [
        status.directory_confirmed_root_count == status.directory_confirmation_root_count,
        status.runtime_activated_root_count == status.directory_confirmation_root_count,
    ]
    .into_iter()
    .all(std::convert::identity);
    let no_effect_is_in_flight = [
        status.acceptance_in_flight_root,
        status.provisioning_in_flight_root,
        status.publication_in_flight_root,
        status.activation_in_flight_root,
    ]
    .into_iter()
    .all(|root| root.is_none());
    let no_cursor_remains = [
        status.current_root.is_none(),
        status.current_synchronization.is_none(),
        status.current_publication.is_none(),
        status.current_activation.is_none(),
    ]
    .into_iter()
    .all(std::convert::identity);
    let terminal_times_exist = [
        status.roots_accepted_at_ns,
        status.components_provisioned_at_ns,
        status.service_topology_published_at_ns,
        status.directories_confirmed_at_ns,
        status.runtimes_activated_at_ns,
    ]
    .into_iter()
    .all(|time| time.is_some());
    let terminal_evidence_exists = [
        terminal_times_exist,
        status.published_fleet_registry.is_some(),
    ]
    .into_iter()
    .all(std::convert::identity);
    let terminal_is_complete = [
        root_work_complete,
        fleet_barriers_complete,
        no_effect_is_in_flight,
        no_cursor_remains,
        terminal_evidence_exists,
    ]
    .into_iter()
    .all(std::convert::identity);
    if !terminal_is_complete {
        return Err(invalid(path, "Coordinator terminal evidence is incomplete"));
    }
    Ok(())
}

pub(super) fn validate_journal(
    path: &Path,
    journal: &FleetComponentProvisioningInstallJournal,
) -> Result<(), FleetComponentProvisioningInstallJournalError> {
    if journal.schema_version != JOURNAL_SCHEMA_VERSION {
        return Err(invalid(path, "unsupported journal schema version"));
    }
    if journal.coordinator == Principal::anonymous()
        || journal.coordinator == Principal::management_canister()
    {
        return Err(invalid(path, "Coordinator principal is invalid"));
    }
    validate_phase_evidence(path, journal)?;
    if let Some(status) = &journal.last_status {
        validate_status_identity(path, journal, status)?;
    }
    if let Some(request) = &journal.advance_request {
        let status = journal.last_status.as_ref().ok_or_else(|| {
            invalid(
                path,
                "advance request exists without preceding Coordinator status",
            )
        })?;
        validate_advance_request(path, request, status)?;
    }
    if let Some(entry) = &journal.catalog_entry {
        validate_catalog_entry(path, journal, entry)?;
    }
    if terminal_phase(journal.phase) {
        let status = journal
            .last_status
            .as_ref()
            .ok_or_else(|| invalid(path, "terminal host phase has no Coordinator status"))?;
        if status.phase != FleetComponentProvisioningPhase::RuntimesActivated {
            return Err(invalid(
                path,
                "terminal host phase precedes runtime activation",
            ));
        }
        validate_terminal_status(path, journal, status)?;
    }
    Ok(())
}

pub(super) fn validate_catalog_entry(
    path: &Path,
    journal: &FleetComponentProvisioningInstallJournal,
    entry: &FleetCatalogEntryV1,
) -> Result<(), FleetComponentProvisioningInstallJournalError> {
    let fleet = &journal.prepare_request.plan.fleet;
    let expected_authority = CatalogEntryAuthority {
        canonical_network_id: fleet.fleet.canonical_network_id,
        fleet_id: fleet.fleet.fleet_id,
        app: &fleet.app,
        coordinator_principal: journal.coordinator.to_text(),
        fleet_name: &journal.fleet_name,
        environment: &journal.environment,
    };
    let observed_authority = CatalogEntryAuthority {
        canonical_network_id: entry.canonical_network_id,
        fleet_id: entry.fleet_id,
        app: &entry.app,
        coordinator_principal: entry.coordinator_principal.clone(),
        fleet_name: &entry.fleet_name,
        environment: &entry.environment,
    };
    let row_is_bound = [
        observed_authority == expected_authority,
        entry.deployed_at_unix_secs > 0,
    ]
    .into_iter()
    .all(std::convert::identity);
    if !row_is_bound {
        return Err(invalid(
            path,
            "Fleet catalog row differs from the frozen provisioning authority",
        ));
    }
    Ok(())
}

pub(super) fn same_immutable_authority(
    observed: &FleetComponentProvisioningInstallJournal,
    expected: &FleetComponentProvisioningInstallJournal,
) -> bool {
    JournalImmutableAuthority::from_journal(observed)
        == JournalImmutableAuthority::from_journal(expected)
}

pub(super) fn invalid(
    path: &Path,
    reason: impl Into<String>,
) -> FleetComponentProvisioningInstallJournalError {
    FleetComponentProvisioningInstallJournalError::InvalidDocument {
        path: path.to_path_buf(),
        reason: reason.into(),
    }
}

fn validate_phase_evidence(
    path: &Path,
    journal: &FleetComponentProvisioningInstallJournal,
) -> Result<(), FleetComponentProvisioningInstallJournalError> {
    let expects_status = !matches!(
        journal.phase,
        FleetComponentProvisioningInstallPhase::Planned
            | FleetComponentProvisioningInstallPhase::PreparationInFlight
    );
    let expects_advance = journal.phase == FleetComponentProvisioningInstallPhase::AdvanceInFlight;
    let expects_catalog_entry = matches!(
        journal.phase,
        FleetComponentProvisioningInstallPhase::CatalogPublicationInFlight
            | FleetComponentProvisioningInstallPhase::CatalogPublished
            | FleetComponentProvisioningInstallPhase::Complete
    );
    let expects_catalog_hash = matches!(
        journal.phase,
        FleetComponentProvisioningInstallPhase::CatalogPublished
            | FleetComponentProvisioningInstallPhase::Complete
    );
    let status_present = journal.last_status.is_some();
    let advance_present = journal.advance_request.is_some();
    let catalog_entry_present = journal.catalog_entry.is_some();
    let catalog_hash_present = journal.catalog_hash.is_some();
    let evidence_matches_phase = [
        status_present == expects_status,
        advance_present == expects_advance,
        catalog_entry_present == expects_catalog_entry,
        catalog_hash_present == expects_catalog_hash,
    ]
    .into_iter()
    .all(std::convert::identity);
    if !evidence_matches_phase {
        return Err(invalid(path, "journal evidence does not match its phase"));
    }
    Ok(())
}

const fn terminal_phase(phase: FleetComponentProvisioningInstallPhase) -> bool {
    matches!(
        phase,
        FleetComponentProvisioningInstallPhase::RuntimesActivated
            | FleetComponentProvisioningInstallPhase::CatalogPublicationInFlight
            | FleetComponentProvisioningInstallPhase::CatalogPublished
            | FleetComponentProvisioningInstallPhase::Complete
    )
}

const fn provisioning_phase_rank(phase: FleetComponentProvisioningPhase) -> u8 {
    match phase {
        FleetComponentProvisioningPhase::Planned => 0,
        FleetComponentProvisioningPhase::AcceptingRoots => 1,
        FleetComponentProvisioningPhase::RootsAccepted => 2,
        FleetComponentProvisioningPhase::ProvisioningRoots => 3,
        FleetComponentProvisioningPhase::ComponentsProvisioned => 4,
        FleetComponentProvisioningPhase::ServiceTopologyPublished => 5,
        FleetComponentProvisioningPhase::ConfirmingDirectories => 6,
        FleetComponentProvisioningPhase::DirectoriesConfirmed => 7,
        FleetComponentProvisioningPhase::ActivatingRuntimes => 8,
        FleetComponentProvisioningPhase::RuntimesActivated => 9,
    }
}

#[derive(Eq, PartialEq)]
struct ProvisioningStatusAuthority<'a> {
    operation_id: &'a [u8; 32],
    plan_hash: &'a [u8; 32],
    fleet_registry: &'a canic_core::dto::fleet_registry::FleetRegistryVersion,
    configuration_digest: &'a canic_core::ids::ComponentDeploymentConfigurationDigest,
    operation: &'a canic_core::dto::component_provisioning::FleetComponentProvisioningOperation,
}

impl<'a> ProvisioningStatusAuthority<'a> {
    const fn from_journal(journal: &'a FleetComponentProvisioningInstallJournal) -> Self {
        Self {
            operation_id: &journal.prepare_request.operation_id,
            plan_hash: &journal.plan_hash,
            fleet_registry: &journal.prepare_request.plan.fleet_registry,
            configuration_digest: &journal.prepare_request.plan.configuration_digest,
            operation: &journal.prepare_request.plan.operation,
        }
    }

    const fn from_status(status: &'a FleetComponentProvisioningStatusResponse) -> Self {
        Self {
            operation_id: &status.operation_id,
            plan_hash: &status.plan_hash,
            fleet_registry: &status.fleet_registry,
            configuration_digest: &status.configuration_digest,
            operation: &status.operation,
        }
    }
}

#[derive(Eq, PartialEq)]
struct StatusCardinality {
    root_batches: u32,
    directory_confirmation_roots: u32,
    group_placements: u32,
    components: u32,
}

impl StatusCardinality {
    fn from_plan(
        path: &Path,
        plan: &canic_core::dto::component_provisioning::FleetComponentProvisioningPlan,
    ) -> Result<Self, FleetComponentProvisioningInstallJournalError> {
        let group_placements = plan.batches.iter().try_fold(0_usize, |total, batch| {
            total.checked_add(batch.placements.len())
        });
        let components = plan
            .batches
            .iter()
            .flat_map(|batch| &batch.placements)
            .try_fold(0_usize, |total, placement| {
                total.checked_add(placement.entries.len())
            });
        Ok(Self {
            root_batches: bounded_count(path, plan.batches.len(), "provisioning batch")?,
            directory_confirmation_roots: bounded_count(
                path,
                plan.directory_confirmation_roots.len(),
                "Directory confirmation root",
            )?,
            group_placements: bounded_count(
                path,
                group_placements
                    .ok_or_else(|| invalid(path, "group placement cardinality overflowed usize"))?,
                "group placement",
            )?,
            components: bounded_count(
                path,
                components
                    .ok_or_else(|| invalid(path, "Component cardinality overflowed usize"))?,
                "Component",
            )?,
        })
    }

    const fn from_status(status: &FleetComponentProvisioningStatusResponse) -> Self {
        Self {
            root_batches: status.root_batch_count,
            directory_confirmation_roots: status.directory_confirmation_root_count,
            group_placements: status.group_placement_count,
            components: status.component_count,
        }
    }
}

#[derive(Clone, Copy)]
struct StatusProgress {
    phase_rank: u8,
    accepted_roots: u32,
    provisioned_roots: u32,
    directory_confirmed_roots: u32,
    runtime_activated_roots: u32,
}

impl StatusProgress {
    const fn from_status(status: &FleetComponentProvisioningStatusResponse) -> Self {
        Self {
            phase_rank: provisioning_phase_rank(status.phase),
            accepted_roots: status.accepted_root_count,
            provisioned_roots: status.provisioned_root_count,
            directory_confirmed_roots: status.directory_confirmed_root_count,
            runtime_activated_roots: status.runtime_activated_root_count,
        }
    }

    fn has_regressed_from(self, previous: Self) -> bool {
        [
            self.phase_rank < previous.phase_rank,
            self.accepted_roots < previous.accepted_roots,
            self.provisioned_roots < previous.provisioned_roots,
            self.directory_confirmed_roots < previous.directory_confirmed_roots,
            self.runtime_activated_roots < previous.runtime_activated_roots,
        ]
        .into_iter()
        .any(std::convert::identity)
    }
}

#[derive(Eq, PartialEq)]
struct JournalImmutableAuthority<'a> {
    schema_version: u32,
    fleet_install_plan_digest: &'a [u8; 32],
    coordinator: Principal,
    fleet_name: &'a canic_core::ids::FleetName,
    environment: &'a str,
    prepare_request:
        &'a canic_core::dto::component_provisioning::FleetComponentProvisioningPrepareRequest,
    plan_hash: &'a [u8; 32],
}

impl<'a> JournalImmutableAuthority<'a> {
    fn from_journal(journal: &'a FleetComponentProvisioningInstallJournal) -> Self {
        Self {
            schema_version: journal.schema_version,
            fleet_install_plan_digest: &journal.fleet_install_plan_digest,
            coordinator: journal.coordinator,
            fleet_name: &journal.fleet_name,
            environment: &journal.environment,
            prepare_request: &journal.prepare_request,
            plan_hash: &journal.plan_hash,
        }
    }
}

#[derive(Eq, PartialEq)]
struct CatalogEntryAuthority<'a> {
    canonical_network_id: canic_core::ids::CanonicalNetworkId,
    fleet_id: canic_core::ids::FleetId,
    app: &'a canic_core::ids::AppId,
    coordinator_principal: String,
    fleet_name: &'a canic_core::ids::FleetName,
    environment: &'a str,
}

fn bounded_count(
    path: &Path,
    count: usize,
    subject: &'static str,
) -> Result<u32, FleetComponentProvisioningInstallJournalError> {
    u32::try_from(count).map_err(|_| invalid(path, format!("{subject} count does not fit u32")))
}
