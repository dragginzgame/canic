//! Select a clean supplied-ID reset using current management authority only.

use crate::{
    fleet_ensure::{
        model::{
            DesiredFleet,
            infrastructure_bootstrap::{
                BootstrapCoordinatorSelection, InfrastructureBootstrapRecord,
            },
        },
        ops::{
            self, EnsurePaths,
            capacity_import::{
                admission::survey::CapacityImportSurveyStore, journal::CapacityImportJournalStore,
            },
            infrastructure_bootstrap::{
                InfrastructureBootstrapError, automatic, survey::BootstrapSurvey,
            },
        },
    },
    icp::IcpCli,
};
use std::path::Path;

/// Freeze original observations under durable inspection allowances before reset review.
pub(super) fn survey(
    workspace: &Path,
    desired: &DesiredFleet,
    icp: &IcpCli,
) -> Result<InfrastructureBootstrapRecord, InfrastructureBootstrapError> {
    let paths = EnsurePaths::under(workspace, &desired.environment, &desired.fleet);
    ops::operation_selection::retirement::prepare_bootstrap(&paths, desired)?;
    let owner = CapacityImportJournalStore::open(&paths)?;
    let digest = automatic::bind(&paths, desired)?;
    if let Some(source) = automatic::retained(&paths)? {
        return Ok(source);
    }
    let agent = automatic::agent(icp, desired)?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let custody = runtime.block_on(automatic::custody(&paths, desired, &agent))?;
    let ids = automatic::direct_ids(desired, &custody)?;
    let mut persistence = CapacityImportSurveyStore::open_bootstrap(&owner, &paths, digest, &ids)?;
    let mut samples = Vec::new();
    for id in ids {
        let sample = if let Some(sample) = persistence.sample(id) {
            sample.clone()
        } else {
            let prepared = runtime.block_on(BootstrapSurvey::prepare_sample(&agent, id))?;
            persistence.reserve(id)?;
            let sample = runtime.block_on(prepared.observe())?;
            persistence.retain(sample.clone())?;
            sample
        };
        samples.push(sample);
    }
    let declarations =
        automatic::declarations(desired, &custody, &samples, &agent.read_root_key())?;
    let mut survey = BootstrapSurvey::begin(
        &paths,
        desired,
        BootstrapCoordinatorSelection::Initialize,
        &declarations,
    )?;
    survey.ledger(desired, icp)?;
    runtime.block_on(survey.finish(&paths, desired, &agent, samples))
}
