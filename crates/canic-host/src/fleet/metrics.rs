//! Bounded queries after operational observations; missing endpoints remain unknown.
use crate::{Error, Network, Result, supplier};
use canic::{
    method::Contract,
    metrics::{ReadMetrics, Report, Request, Snapshot},
};
use serde::Serialize;
use std::{collections::BTreeMap, time::Duration};

/// One owner's uncertified diagnostics, separate from every child's operation custody.
#[derive(Debug, Serialize)]
pub struct FleetMetrics {
    /// Exact selected network trust binding.
    pub network_id: String,
    /// Known management owner; never inferred from an arbitrary report field.
    pub canister: String,
    /// Present only after a bounded, correctly framed reply.
    pub reported: Option<Snapshot>,
    /// Missing/older endpoints and transport failures are unknown, never zero counters.
    pub observation_error: Option<String>,
}

pub(super) async fn collect(
    targets: &BTreeMap<(String, String), Network>,
    deadline: tokio::time::Instant,
) -> Vec<FleetMetrics> {
    let mut reports = Vec::new();
    for ((network_id, canister), network) in targets {
        let budget = deadline
            .saturating_duration_since(tokio::time::Instant::now())
            .min(Duration::from_secs(2));
        let result = if budget.is_zero() {
            Err(Error::Blocked(
                "snapshot budget exhausted; metrics not observed".into(),
            ))
        } else {
            tokio::time::timeout(budget, observe(network, canister))
                .await
                .map_err(|_| Error::Blocked("metrics observation timed out".into()))
                .and_then(|value| value)
        };
        let (reported, observation_error) = match result {
            Ok(value) => (Some(value), None),
            Err(error) => (None, Some(error.to_string().chars().take(500).collect())),
        };
        reports.push(FleetMetrics {
            network_id: network_id.clone(),
            canister: canister.clone(),
            reported,
            observation_error,
        });
    }
    reports
}

async fn observe(network: &Network, canister: &str) -> Result<Snapshot> {
    let bytes = ReadMetrics::encode_request(&Request::Timers).map_err(supplier)?;
    let response = crate::network::query(
        &network.client()?,
        crate::network::principal(canister)?,
        ReadMetrics::NAME,
        &bytes,
    )
    .await?;
    match ReadMetrics::decode_reply(&response).map_err(supplier)? {
        Report::Timers(snapshot) => Ok(snapshot),
        _ => Err(Error::Invalid("timer report unavailable".into())),
    }
}
