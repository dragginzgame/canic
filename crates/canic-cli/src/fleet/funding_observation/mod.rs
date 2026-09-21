//! Module: canic_cli::fleet::funding_observation
//!
//! Responsibility: present and approve one retained Root's bounded observation pass.
//! Boundary: host workflow owns authority, durable attempts and native funding reviews.

use crate::fleet::{EnsureOptions, FleetCommandError};
use canic_host::fleet_ensure::{
    IcpEnsurePlatform, LoadedDesiredFleet, ops::EnsurePaths, workflow::funding_observation,
};
use std::path::Path;

pub(super) fn run(
    workspace: &Path,
    loaded: &LoadedDesiredFleet,
    options: &EnsureOptions,
    root: &str,
) -> Result<(), FleetCommandError> {
    let paths = EnsurePaths::under(workspace, &loaded.desired.environment, &options.fleet);
    let mut platform = IcpEnsurePlatform::new(loaded.desired.clone(), &options.icp, workspace)
        .with_identity(options.identity.as_deref());
    if let Some(digest) = &options.apply {
        funding_observation::collect(&paths, root, digest, &mut platform)?;
    } else {
        funding_observation::review(&paths, root, &mut platform)?;
    }
    let report = funding_observation::status(&paths, root)
        .map_err(|error| FleetCommandError::FundingObservationStatus(Box::new(error)))?;
    if options.json {
        crate::output::write_pretty_json::<_, FleetCommandError>(
            None,
            &serde_json::json!({"schema_version":1,"funding_observation":report}),
        )?;
    } else {
        println!(
            "Funding observation for {root}: {}/{} attempts consumed",
            report.review.attempts.len(),
            report.review.body.requests.len()
        );
        println!("Review: {}", report.review.review_sha256);
        println!(
            "Maximum observation allowance: {} cycles; protected native recovery floor: {} cycles",
            report.review.body.maximum_cycles, report.review.body.recovery_floor_cycles
        );
        match report.recovery_demand {
            Ok(demand) => println!(
                "Recovery forecast: {} cycles native minimum; {} cycles observed shortfall; {} cycles uncovered by automatic child policy",
                demand.minimum_native_cycles, demand.shortfall_cycles, demand.uncovered_cycles
            ),
            Err(reason) => println!("Recovery forecast unavailable: {reason}"),
        }
        println!(
            "Observation approval permits bounded reads only. Repeat ordinary Fleet Ensure review for a separate funding approval. Forecasts do not reserve runtime window allowances."
        );
    }
    Ok(())
}
