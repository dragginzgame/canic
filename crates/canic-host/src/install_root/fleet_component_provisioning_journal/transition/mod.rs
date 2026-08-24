//! Module: install_root::fleet_component_provisioning_journal::transition
//!
//! Responsibility: apply the typed host phase machine for fresh Component provisioning.
//! Does not own: document encoding, filesystem mechanics, authority rules, or remote effects.
//! Boundary: each next phase is validated and durably replaced before callers invoke its effect.

use super::{
    model::{
        FleetComponentProvisioningInstallJournalError, FleetComponentProvisioningInstallPhase,
        PlanFleetComponentProvisioningInstallRequest, ResolvedFleetComponentProvisioningInstall,
    },
    persistence::{create_or_load, journal_path, replace_exact},
    validation::{
        advance_request, invalid, planned_journal, validate_advance_request,
        validate_catalog_entry, validate_journal, validate_status_identity,
        validate_status_progress, validate_terminal_status,
    },
};
use crate::fleet_catalog::{CommittedFleetCatalog, FleetCatalogEntryV1};

use canic_core::dto::component_provisioning::{
    FleetComponentProvisioningPhase, FleetComponentProvisioningStatusResponse,
};

pub(in crate::install_root) fn plan_fleet_component_provisioning_install(
    request: PlanFleetComponentProvisioningInstallRequest<'_>,
) -> Result<ResolvedFleetComponentProvisioningInstall, FleetComponentProvisioningInstallJournalError>
{
    let path = journal_path(&request.fleet_install_plan.path);
    let expected = planned_journal(&path, request)?;
    create_or_load(path, expected)
}

pub(in crate::install_root) fn begin_component_provisioning_preparation(
    current: &ResolvedFleetComponentProvisioningInstall,
) -> Result<ResolvedFleetComponentProvisioningInstall, FleetComponentProvisioningInstallJournalError>
{
    advance_without_evidence(
        current,
        FleetComponentProvisioningInstallPhase::Planned,
        FleetComponentProvisioningInstallPhase::PreparationInFlight,
    )
}

pub(in crate::install_root) fn record_component_provisioning_observed(
    current: &ResolvedFleetComponentProvisioningInstall,
    status: FleetComponentProvisioningStatusResponse,
) -> Result<ResolvedFleetComponentProvisioningInstall, FleetComponentProvisioningInstallJournalError>
{
    validate_status_identity(&current.path, &current.journal, &status)?;
    let next_phase = if status.phase == FleetComponentProvisioningPhase::RuntimesActivated {
        validate_terminal_status(&current.path, &current.journal, &status)?;
        FleetComponentProvisioningInstallPhase::RuntimesActivated
    } else {
        FleetComponentProvisioningInstallPhase::Prepared
    };
    transition(
        current,
        FleetComponentProvisioningInstallPhase::PreparationInFlight,
        next_phase,
        |next| next.last_status = Some(status),
    )
}

pub(in crate::install_root) fn begin_component_provisioning_advance(
    current: &ResolvedFleetComponentProvisioningInstall,
) -> Result<ResolvedFleetComponentProvisioningInstall, FleetComponentProvisioningInstallJournalError>
{
    let status = current
        .journal
        .last_status
        .as_ref()
        .ok_or_else(|| invalid(&current.path, "Prepared journal has no Coordinator status"))?;
    if status.phase == FleetComponentProvisioningPhase::RuntimesActivated {
        return Err(invalid(
            &current.path,
            "terminal Coordinator status cannot begin another provisioning advance",
        ));
    }
    let request = advance_request(status);
    transition(
        current,
        FleetComponentProvisioningInstallPhase::Prepared,
        FleetComponentProvisioningInstallPhase::AdvanceInFlight,
        |next| next.advance_request = Some(request),
    )
}

pub(in crate::install_root) fn record_component_provisioning_advanced(
    current: &ResolvedFleetComponentProvisioningInstall,
    status: FleetComponentProvisioningStatusResponse,
) -> Result<ResolvedFleetComponentProvisioningInstall, FleetComponentProvisioningInstallJournalError>
{
    let previous = current
        .journal
        .last_status
        .as_ref()
        .ok_or_else(|| invalid(&current.path, "advance intent has no preceding status"))?;
    let request = current
        .journal
        .advance_request
        .as_ref()
        .ok_or_else(|| invalid(&current.path, "advance intent has no exact request"))?;
    validate_advance_request(&current.path, request, previous)?;
    validate_status_progress(&current.path, &current.journal, previous, &status)?;
    let next_phase = if status.phase == FleetComponentProvisioningPhase::RuntimesActivated {
        validate_terminal_status(&current.path, &current.journal, &status)?;
        FleetComponentProvisioningInstallPhase::RuntimesActivated
    } else {
        FleetComponentProvisioningInstallPhase::Prepared
    };
    transition(
        current,
        FleetComponentProvisioningInstallPhase::AdvanceInFlight,
        next_phase,
        |next| {
            next.last_status = Some(status);
            next.advance_request = None;
        },
    )
}

pub(in crate::install_root) fn begin_fleet_catalog_publication(
    current: &ResolvedFleetComponentProvisioningInstall,
    entry: FleetCatalogEntryV1,
) -> Result<ResolvedFleetComponentProvisioningInstall, FleetComponentProvisioningInstallJournalError>
{
    validate_catalog_entry(&current.path, &current.journal, &entry)?;
    transition(
        current,
        FleetComponentProvisioningInstallPhase::RuntimesActivated,
        FleetComponentProvisioningInstallPhase::CatalogPublicationInFlight,
        |next| next.catalog_entry = Some(entry),
    )
}

pub(in crate::install_root) fn record_fleet_catalog_published(
    current: &ResolvedFleetComponentProvisioningInstall,
    committed: CommittedFleetCatalog,
) -> Result<ResolvedFleetComponentProvisioningInstall, FleetComponentProvisioningInstallJournalError>
{
    if current.journal.phase != FleetComponentProvisioningInstallPhase::CatalogPublicationInFlight {
        return Err(
            FleetComponentProvisioningInstallJournalError::InvalidTransition {
                observed: current.journal.phase,
                requested: FleetComponentProvisioningInstallPhase::CatalogPublished,
            },
        );
    }
    let expected_entry = current.journal.catalog_entry.as_ref().ok_or_else(|| {
        invalid(
            &current.path,
            "catalog publication intent has no frozen row",
        )
    })?;
    if &committed.entry != expected_entry {
        return Err(invalid(
            &current.path,
            "committed Fleet catalog entry differs from the frozen publication row",
        ));
    }
    transition(
        current,
        FleetComponentProvisioningInstallPhase::CatalogPublicationInFlight,
        FleetComponentProvisioningInstallPhase::CatalogPublished,
        |next| next.catalog_hash = Some(committed.catalog_hash),
    )
}

pub(in crate::install_root) fn complete_fleet_component_provisioning_install(
    current: &ResolvedFleetComponentProvisioningInstall,
) -> Result<ResolvedFleetComponentProvisioningInstall, FleetComponentProvisioningInstallJournalError>
{
    advance_without_evidence(
        current,
        FleetComponentProvisioningInstallPhase::CatalogPublished,
        FleetComponentProvisioningInstallPhase::Complete,
    )
}

fn advance_without_evidence(
    current: &ResolvedFleetComponentProvisioningInstall,
    expected: FleetComponentProvisioningInstallPhase,
    requested: FleetComponentProvisioningInstallPhase,
) -> Result<ResolvedFleetComponentProvisioningInstall, FleetComponentProvisioningInstallJournalError>
{
    if current.journal.phase == requested {
        return Ok(ResolvedFleetComponentProvisioningInstall {
            journal: current.journal.clone(),
            path: current.path.clone(),
        });
    }
    transition(current, expected, requested, |_| {})
}

fn transition(
    current: &ResolvedFleetComponentProvisioningInstall,
    expected: FleetComponentProvisioningInstallPhase,
    requested: FleetComponentProvisioningInstallPhase,
    apply: impl FnOnce(&mut super::model::FleetComponentProvisioningInstallJournal),
) -> Result<ResolvedFleetComponentProvisioningInstall, FleetComponentProvisioningInstallJournalError>
{
    if current.journal.phase != expected {
        return Err(
            FleetComponentProvisioningInstallJournalError::InvalidTransition {
                observed: current.journal.phase,
                requested,
            },
        );
    }
    let mut next = current.journal.clone();
    next.sequence = next
        .sequence
        .checked_add(1)
        .ok_or_else(|| invalid(&current.path, "journal sequence exhausted"))?;
    next.phase = requested;
    apply(&mut next);
    validate_journal(&current.path, &next)?;
    replace_exact(current, next)
}
