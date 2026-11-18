//! Explicit mainnet subnet candidates in fleet context; no placement recommendation.

#[cfg(test)]
mod tests;

use crate::{Error, Network, Result, SelectedFleet, supplier};
use ic_platform_types::Principal;
use ic_query_topology::subnet::compare::{self, ComparisonError, SubnetComparisonReport};
use serde::Serialize;
use std::future::Future;

/// Fleet context and the complete, independently acquired Query evidence.
#[derive(Debug, Serialize)]
pub struct FleetSubnetComparison {
    /// Selected app; does not establish ownership of either subnet.
    pub app: String,
    /// Selected mainnet fleet; candidates are not inferred from its bindings.
    pub fleet: String,
    /// Original Query report, including partial outcomes and independent versions.
    pub observation: SubnetComparisonReport,
}

/// Compare two explicitly selected mainnet subnets for one configured fleet.
///
/// Uses Query's existing bounded comparison: at most 160 asynchronous seconds
/// and 774 logical calls, plus bounded synchronous work. No journal, signer,
/// retries, discovery, ranking or deployment effects. Cancellation drops reads.
/// Successful return may be partial; inspect `observation.collection_complete`.
///
/// # Errors
/// Rejects non-mainnet fleets and malformed IDs before IO. Query rejects duplicate
/// IDs before IO; its original typed failure remains the supplier error source.
pub async fn observe(
    selected: &SelectedFleet,
    left: &str,
    right: &str,
) -> Result<FleetSubnetComparison> {
    collect(selected, left, right, compare::fetch).await
}

async fn collect<F>(
    selected: &SelectedFleet,
    left: &str,
    right: &str,
    reader: impl FnOnce(Principal, Principal) -> F,
) -> Result<FleetSubnetComparison>
where
    F: Future<Output = std::result::Result<SubnetComparisonReport, ComparisonError>>,
{
    if selected.config.network != Network::Mainnet {
        return Err(Error::Invalid(
            "subnet comparison supports configured mainnet fleets only".into(),
        ));
    }
    let left = left
        .parse()
        .map_err(|_| Error::Invalid("invalid left subnet ID".into()))?;
    let right = right
        .parse()
        .map_err(|_| Error::Invalid("invalid right subnet ID".into()))?;
    let observation = reader(left, right).await.map_err(supplier)?;
    Ok(FleetSubnetComparison {
        app: selected.app.clone(),
        fleet: selected.fleet.clone(),
        observation,
    })
}
