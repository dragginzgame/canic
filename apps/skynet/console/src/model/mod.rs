//! Module: skynet_console::model
//!
//! Responsibility: define framework-independent data supplied to the Skynet console renderer.
//! Does not own: Canic runtime reads, authorization, persistence, or network discovery.
//! Boundary: role canisters translate their local observations into these presentation models.

use serde::Serialize;

///
/// ConsoleSnapshot
///
/// Complete public observation rendered by one Skynet demo canister.
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ConsoleSnapshot {
    pub schema_version: u16,
    pub generated_at_ns: u64,
    pub identity: NodeIdentity,
    pub runtime: RuntimeSummary,
    pub environment: Vec<Fact>,
    pub deployment: Vec<Fact>,
    pub capabilities: Vec<Capability>,
    pub endpoints: Vec<Endpoint>,
    pub metrics: Vec<MetricRow>,
    pub children: Vec<CanisterNode>,
    pub network: NetworkView,
}

///
/// NodeIdentity
///
/// Public package and canister identity for the current console.
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct NodeIdentity {
    pub codename: String,
    pub role: String,
    pub canister_id: String,
    pub package_name: String,
    pub package_version: String,
    pub canic_version: String,
    pub canister_version: u64,
}

///
/// RuntimeSummary
///
/// High-level local health and capacity values safe for public display.
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RuntimeSummary {
    pub ready: bool,
    pub phase: String,
    pub cycles: u128,
    pub bootstrap: String,
    pub observation: String,
}

///
/// Fact
///
/// Named public configuration or environment fact.
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Fact {
    pub name: String,
    pub value: String,
    pub source: String,
}

///
/// Capability
///
/// One configured or runtime-observed Canic capability.
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Capability {
    pub name: String,
    pub status: String,
    pub detail: String,
}

///
/// Endpoint
///
/// One public or guarded Candid endpoint advertised by the demo role.
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Endpoint {
    pub name: String,
    pub mode: String,
    pub access: String,
    pub purpose: String,
}

///
/// MetricRow
///
/// Flattened public metric suitable for the browser console and JSON view.
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct MetricRow {
    pub tier: String,
    pub labels: Vec<String>,
    pub principal: Option<String>,
    pub value: String,
}

///
/// CanisterNode
///
/// One local child or Fleet service member that has its own console URL.
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CanisterNode {
    pub canister_id: String,
    pub role: String,
    pub relation: String,
    pub url: String,
    pub current: bool,
}

///
/// NetworkView
///
/// Fleet-wide roots and published services from the current protected Directory.
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct NetworkView {
    pub authority: String,
    pub registry_revision: Option<u64>,
    pub registry_hash: Option<String>,
    pub roots: Vec<NetworkRoot>,
    pub services: Vec<NetworkService>,
}

impl NetworkView {
    #[must_use]
    pub fn unavailable(authority: impl Into<String>) -> Self {
        Self {
            authority: authority.into(),
            registry_revision: None,
            registry_hash: None,
            roots: Vec::new(),
            services: Vec::new(),
        }
    }
}

///
/// NetworkRoot
///
/// One physical Subnet and Fleet Subnet Root projected by the Fleet Directory.
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct NetworkRoot {
    pub subnet_id: String,
    pub root_canister_id: String,
    pub url: String,
    pub status: String,
    pub current: bool,
}

///
/// NetworkService
///
/// One published Fleet service and its complete configured membership.
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct NetworkService {
    pub service: String,
    pub mode: String,
    pub role: String,
    pub maximum_members_per_root: u32,
    pub minimum_distinct_roots: u32,
    pub members: Vec<NetworkMember>,
}

///
/// NetworkMember
///
/// One Authority, Replica, or PoolMember in a published Fleet service.
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct NetworkMember {
    pub purpose: String,
    pub canister_id: String,
    pub root_canister_id: String,
    pub placement: String,
    pub url: String,
    pub current: bool,
}
