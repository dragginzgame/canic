//! Module: fleet_ensure::workflow::reinstall::terminal
//!
//! Responsibility: assess completed source conservation before a separately reviewed preparation.
//! Does not own: source decoding, archive mutation, record conversion or remote effects.
//! Boundary: read-only source observations precede the existing journaled reinstall owner.

use super::*;
use crate::fleet_ensure::view::terminal_source::TerminalSourceView;

pub(super) fn plan_preparation<P: EnsurePlatform>(
    root: &Path,
    paths: &EnsurePaths,
    desired: &DesiredFleet,
    digest: &str,
    time: u64,
    state: &FleetEnsureStateRecord,
    platform: &mut P,
) -> Result<FleetEnsureReport, EnsureWorkflowError<P::Error>> {
    let source = capture::terminal::read(paths, &desired.environment, &desired.fleet)?;
    let authority = capture::capture_source(root, source.reviewed_desired.desired())?;
    let conservation = verify(desired, &authority, &source, state, platform)?;
    let authority = capture::terminal::capture(root, &source, conservation)?;
    let operation = canic_core::cdk::utils::hash::sha256_hex(
        format!(
            "canic-fleet-reinstall:{digest}:{}:{time}",
            source.documents.operation_id
        )
        .as_bytes(),
    );
    let plan = preparation(
        root,
        desired,
        digest,
        &source.documents.operation_id,
        &authority,
        &operation,
        time,
        platform,
        state,
    )?;
    capture::adoption::stage(paths, &plan)?;
    Ok(report(plan))
}

pub(super) fn verify<P: EnsurePlatform>(
    target: &DesiredFleet,
    authority: &FleetReinstallSourceRecord,
    source: &TerminalSourceView,
    state: &FleetEnsureStateRecord,
    platform: &mut P,
) -> Result<crate::fleet_ensure::model::ActualCycleConservation, EnsureWorkflowError<P::Error>> {
    let desired = source.reviewed_desired.desired();
    if desired.operator != target.operator || desired.cycles_ledger != target.cycles_ledger {
        return Err(EnsureWorkflowError::ReinstallConflict);
    }
    let inventory = observe_source(authority, target, platform, |platform| {
        platform.terminal_inventory(&source.documents.operation_id, state)
    })?;
    if inventory.active_registry != state.active_registry
        || !policy::terminal::inventory_matches(state, &inventory.entries)
    {
        return Err(EnsureWorkflowError::ConvergenceDrift);
    }
    let cycles = inventory.controlled_cycles_by_principal;
    let mut observation = observe_source(authority, target, platform, |platform| {
        platform.observe(&source.documents.operation_id, state)
    })?;
    attach_terminal_cycles(&mut observation, cycles)?;
    policy::terminal::conservation(source, &observation, controlled_cycles(&observation)?)
        .ok_or_else(|| EnsureWorkflowError::Conservation(
            "completed-source receipts and fresh controlled cycles do not reconcile within the original bounds; preserve source evidence".into()))
}
