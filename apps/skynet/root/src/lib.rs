//! Module: skynet_root
//!
//! Responsibility: serve one Fleet Subnet Root's Skynet-themed local observability console.
//! Does not own: Coordinator state, application service discovery, or console presentation.
//! Boundary: publishes sanitized root-local observations and lists guarded control-plane methods.

#![expect(clippy::unused_async)]

use canic::{
    api::{env::EnvQuery, metrics::MetricsQuery},
    dto::{
        metrics::{MetricEntry, MetricValue, MetricsKind},
        page::PageRequest,
    },
    prelude::*,
};
use ic_cdk::api::{canister_cycle_balance, canister_self, canister_version, time};
use skynet_console::{
    ConsoleSnapshot, Endpoint, HttpRequest, HttpResponse, MetricRow, NetworkView, NodeIdentity,
    RuntimeSummary, StandardEndpointSurface, capability, endpoint_highlights, fact, option_text,
};

canic::start!();

async fn canic_setup() {}
async fn canic_install() {}
async fn canic_upgrade() {}

#[canic_query(internal, public)]
fn http_request(request: HttpRequest) -> HttpResponse {
    skynet_console::response_for(request, &console_snapshot())
}

fn console_snapshot() -> ConsoleSnapshot {
    let canister_id = canister_self();
    let environment = EnvQuery::snapshot();
    ConsoleSnapshot {
        schema_version: 1,
        generated_at_ns: time(),
        identity: NodeIdentity {
            codename: "Defense Grid Root".to_string(),
            role: "fleet_subnet_root".to_string(),
            canister_id: canister_id.to_text(),
            package_name: env!("CARGO_PKG_NAME").to_string(),
            package_version: env!("CARGO_PKG_VERSION").to_string(),
            canic_version: canic::VERSION.to_string(),
            canister_version: canister_version(),
        },
        runtime: RuntimeSummary {
            ready: canic::api::runtime::ReadyApi::is_ready(),
            phase: if canic::api::runtime::ReadyApi::is_ready() {
                "Active"
            } else {
                "Prepared"
            }
            .to_string(),
            cycles: canister_cycle_balance(),
            bootstrap: format!("{:?}", canic::api::runtime::ReadyApi::bootstrap_status()),
            observation: "root-local control plane".to_string(),
        },
        environment: vec![
            fact(
                "Canister role",
                option_text(environment.canister_role),
                "canic_env",
            ),
            fact(
                "Physical Subnet",
                option_text(environment.subnet_pid),
                "canic_env",
            ),
            fact("Fleet Subnet Root", canister_id, "canister_self"),
            fact(
                "Immediate parent",
                option_text(environment.parent_pid),
                "canic_env",
            ),
        ],
        deployment: vec![
            fact("Node admission", "1 Skynet Component", "Fleet input"),
            fact("Ready pool target", "1 prepaid Canister", "Fleet input"),
            fact("Group-placement ceiling", "1", "Fleet input"),
            fact("Wasm Store ceiling", "100000000 bytes", "Fleet input"),
        ],
        capabilities: vec![
            capability(
                "Component Registry",
                "enabled",
                "root-owned topology and lifecycle journal",
            ),
            capability(
                "Prepaid pool",
                "enabled",
                "root-local lifecycle asset inventory",
            ),
            capability(
                "Wasm Store",
                "enabled",
                "implicit sibling Store on this physical Subnet",
            ),
            capability("Metrics", "enabled", "root-derived Canic metric profile"),
            capability(
                "Fleet Directory",
                "guarded",
                "application runtimes receive the published projection",
            ),
        ],
        endpoints: endpoint_rows(),
        metrics: metric_rows(),
        children: Vec::new(),
        network: NetworkView::unavailable(
            "root-local console; open a Skynet node for the protected Fleet Directory",
        ),
    }
}

fn endpoint_rows() -> Vec<Endpoint> {
    endpoint_highlights(StandardEndpointSurface::Root, [])
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
                limit: 10,
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
