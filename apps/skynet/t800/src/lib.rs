//! Module: skynet_t800
//!
//! Responsibility: serve one scaled T-800 worker and its local Canic console.
//! Does not own: scaling admission, parent topology, or Fleet service authority.
//! Boundary: the parent Skynet node creates this worker through the configured scaling pool.

#![expect(clippy::unused_async)]

use canic::{
    api::{canister::deployment::ComponentRuntimeApi, env::EnvQuery, metrics::MetricsQuery},
    dto::{
        metrics::{MetricEntry, MetricValue, MetricsKind},
        page::PageRequest,
    },
    prelude::*,
};
use ic_cdk::api::{canister_cycle_balance, canister_self, canister_version, msg_caller, time};
use skynet_console::{
    CanisterNode, Capability, ConsoleSnapshot, Endpoint, Fact, HttpRequest, HttpResponse,
    MetricRow, NetworkView, NodeIdentity, RuntimeSummary, StandardEndpointSurface, capability,
    console_url, endpoint, endpoint_highlights, fact, option_text,
};

canic::start!();

async fn canic_setup() {}
async fn canic_install(_: Option<Vec<u8>>) {}
async fn canic_upgrade() {}

#[canic_query(internal, public)]
fn http_request(request: HttpRequest) -> HttpResponse {
    skynet_console::response_for(request, &console_snapshot())
}

#[canic_query(public)]
fn skynet_t800_ping(payload: String) -> Result<String, canic::Error> {
    Ok(format!(
        "model=T-800\nunit={}\ncaller={}\npayload={payload}",
        canister_self(),
        msg_caller()
    ))
}

fn console_snapshot() -> ConsoleSnapshot {
    let canister_id = canister_self();
    let environment = EnvQuery::snapshot();
    let runtime = ComponentRuntimeApi::status().ok();
    let parent = environment.parent_pid;
    let children = parent.map_or_else(Vec::new, |parent| {
        vec![CanisterNode {
            canister_id: parent.to_text(),
            role: "skynet_node".to_string(),
            relation: "parent uplink".to_string(),
            url: console_url(parent),
            current: false,
        }]
    });
    ConsoleSnapshot {
        schema_version: 1,
        generated_at_ns: time(),
        identity: NodeIdentity {
            codename: "T-800 Infiltration Unit".to_string(),
            role: "t800".to_string(),
            canister_id: canister_id.to_text(),
            package_name: env!("CARGO_PKG_NAME").to_string(),
            package_version: env!("CARGO_PKG_VERSION").to_string(),
            canic_version: canic::VERSION.to_string(),
            canister_version: canister_version(),
        },
        runtime: RuntimeSummary {
            ready: canic::api::runtime::ReadyApi::is_ready(),
            phase: runtime.as_ref().map_or_else(
                || "Prepared".to_string(),
                |status| format!("{:?}", status.phase),
            ),
            cycles: canister_cycle_balance(),
            bootstrap: format!("{:?}", canic::api::runtime::ReadyApi::bootstrap_status()),
            observation: "scaled Component child".to_string(),
        },
        environment: environment_facts(environment),
        deployment: deployment_facts(runtime.as_ref()),
        capabilities: capability_rows(),
        endpoints: endpoint_rows(),
        metrics: metric_rows(),
        children,
        network: NetworkView::unavailable("follow the parent uplink for the global Fleet map"),
    }
}

fn environment_facts(snapshot: canic::dto::env::EnvSnapshotResponse) -> Vec<Fact> {
    vec![
        fact(
            "Canister role",
            option_text(snapshot.canister_role),
            "canic_env",
        ),
        fact(
            "Component Spec",
            option_text(snapshot.component_spec),
            "canic_env",
        ),
        fact(
            "Physical Subnet",
            option_text(snapshot.subnet_pid),
            "canic_env",
        ),
        fact(
            "Fleet Subnet Root",
            option_text(snapshot.fleet_subnet_root_pid),
            "canic_env",
        ),
        fact(
            "Immediate parent",
            option_text(snapshot.parent_pid),
            "canic_env",
        ),
    ]
}

fn deployment_facts(
    status: Option<&canic::dto::component_registry::ComponentRuntimeStatusResponse>,
) -> Vec<Fact> {
    vec![
        fact("Pool", "t800_units", "canic.toml"),
        fact("Lifecycle kind", "replica", "canic.toml"),
        fact("Initial cycles", "500B", "canic.toml"),
        fact(
            "Inherited deployment",
            status.map_or_else(
                || "pending".to_string(),
                |status| format!("{:?}", status.deployment),
            ),
            "protected state",
        ),
    ]
}

fn capability_rows() -> Vec<Capability> {
    vec![
        capability(
            "Scaling",
            "managed",
            "created through the parent pool with policy 1..4",
        ),
        capability("Metrics", "enabled", "full public Canic metric profile"),
        capability("Memory ledger", "enabled", "controller-only ABI diagnostic"),
        capability(
            "Parent binding",
            "enforced",
            "the exact immediate parent is retained in protected state",
        ),
    ]
}

fn endpoint_rows() -> Vec<Endpoint> {
    endpoint_highlights(
        StandardEndpointSurface::Component,
        [endpoint(
            "skynet_t800_ping",
            "query",
            "public",
            "worker response and caller identity",
        )],
    )
}

fn metric_rows() -> Vec<MetricRow> {
    [
        ("core", MetricsKind::Core),
        ("placement", MetricsKind::Placement),
        ("platform", MetricsKind::Platform),
        ("runtime", MetricsKind::Runtime),
        ("security", MetricsKind::Security),
        ("storage", MetricsKind::Storage),
    ]
    .into_iter()
    .flat_map(|(tier, kind)| {
        MetricsQuery::page(
            kind,
            PageRequest {
                limit: 8,
                offset: 0,
            },
        )
        .entries
        .into_iter()
        .map(move |entry| metric_row(tier, entry))
    })
    .collect()
}

fn metric_row(tier: &str, entry: MetricEntry) -> MetricRow {
    MetricRow {
        tier: tier.to_string(),
        labels: entry.labels,
        principal: entry.principal.map(|principal| principal.to_text()),
        value: match entry.value {
            MetricValue::Count(value) => value.to_string(),
            MetricValue::CountAndU64 { count, value_u64 } => {
                format!("count={count}, value={value_u64}")
            }
            MetricValue::U128(value) => value.to_string(),
        },
    }
}

canic::finish!();
