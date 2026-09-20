//! Explicit collection budgets and data-only renderer configuration.

pub(super) mod cost;

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Finite operator-selected collection and publication budgets.
pub struct ObservatoryOptions {
    pub environment: String,
    pub fleet: String,
    pub collect_costs: bool,
    pub maximum_canisters: usize,
    pub maximum_response_bytes: usize,
    pub freshness_secs: u32,
    pub query_timeout_secs: u32,
    pub maximum_collection_secs: u32,
}

/// Downstream labels; no executable markup, queries or authority fields are accepted.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ObservatoryProfile {
    pub schema_version: u16,
    pub title: String,
    pub role_labels: BTreeMap<String, String>,
}
