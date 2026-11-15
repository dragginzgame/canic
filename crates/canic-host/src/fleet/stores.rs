//! Diagnostic reads selected by retained Store authority, never role-based discovery.
use super::{FleetOperation, Network};
use crate::{Error, Result, network, supplier};
use canic_management_protocol::store_maintenance as wire;
use serde::Serialize;
use std::{collections::BTreeMap, time::Duration};

/// Public Root report for one known Store; no signed attempt or completion receipt.
#[derive(Debug, Serialize)]
pub struct FleetStoreMaintenance {
    /// Exact network trust binding, also used for independent canister metadata.
    pub network_id: String,
    /// Root from a retained Store bootstrap plan.
    pub root: String,
    /// Known Store identity; a public report cannot replace it or discover another Store.
    pub store: String,
    /// Retained operation metadata, not current admission or readiness. None with no error means no attempt.
    pub reported: Option<wire::Status>,
    /// Unavailable or mismatched reports remain unknown, including missing methods in older Roots.
    pub observation_error: Option<String>,
}

fn selected(
    operations: &[FleetOperation],
) -> Result<std::collections::BTreeSet<(String, String, String)>> {
    let selected: std::collections::BTreeSet<_> = operations
        .iter()
        .filter_map(|operation| {
            Some((
                operation.network_id.clone(),
                operation.store_root.clone()?,
                operation.canister.clone()?,
            ))
        })
        .collect();
    if selected.len() > 16 {
        return Err(Error::Invalid(
            "at most sixteen network-bound Store reports per snapshot".into(),
        ));
    }
    Ok(selected)
}

fn decode(store: &str, response: &[u8]) -> Result<Option<wire::Status>> {
    let report: Option<wire::Status> =
        ic_wire::native::decode_response(wire::STATUS, response, wire::LIMITS).map_err(supplier)?;
    if report
        .as_ref()
        .is_some_and(|status| status.approval.store != store)
    {
        return Err(Error::Invalid(
            "Root maintenance report names another Store; selected identity is unchanged".into(),
        ));
    }
    Ok(report)
}

pub(super) async fn collect(
    operations: &[FleetOperation],
    targets: &BTreeMap<(String, String), Network>,
    deadline: tokio::time::Instant,
) -> Result<Vec<FleetStoreMaintenance>> {
    let mut reports = Vec::new();
    for (network_id, root, store) in selected(operations)? {
        let network = targets
            .get(&(network_id.clone(), root.clone()))
            .ok_or_else(|| Error::Invalid("Store owner has no exact network binding".into()))?;
        let budget = deadline
            .saturating_duration_since(tokio::time::Instant::now())
            .min(Duration::from_secs(5));
        let result = if budget.is_zero() {
            Err(Error::Blocked(
                "snapshot network budget exhausted; Store maintenance not observed".into(),
            ))
        } else {
            tokio::time::timeout(budget, async {
                network::principal(&root)?;
                let response = network
                    .agent()?
                    .query(&root.parse().map_err(supplier)?, "store_maintenance_status")
                    .with_arg(
                        ic_wire::native::encode_request(wire::STATUS, &(), wire::LIMITS)
                            .map_err(supplier)?,
                    )
                    .call()
                    .await
                    .map_err(supplier)?;
                decode(&store, &response)
            })
            .await
            .map_err(|_| Error::Blocked("Store maintenance observation timed out".into()))
            .and_then(|value| value)
        };
        let (reported, observation_error) = match result {
            Ok(report) => (report, None),
            Err(error) => (None, Some(error.to_string().chars().take(500).collect())),
        };
        reports.push(FleetStoreMaintenance {
            network_id,
            root,
            store,
            reported,
            observation_error,
        });
    }
    Ok(reports)
}

#[cfg(test)]
mod tests;
