//! Module: skynet_node
//!
//! Responsibility: expose the Skynet service node, placement demos, and live Fleet console.
//! Does not own: Fleet Directory authority, root lifecycle effects, or console presentation.
//! Boundary: reads protected Canic state and passes sanitized observations to `skynet_console`.

#![expect(clippy::unused_async)]

use canic::{
    Error,
    api::{
        canister::{
            children::CanisterChildrenApi,
            deployment::ComponentRuntimeApi,
            placement::{ScalingApi, ShardingApi},
        },
        env::EnvQuery,
        metrics::MetricsQuery,
    },
    dto::{
        component_deployment::{ComponentDeploymentPurpose, ProtectedComponentDeployment},
        component_registry::ComponentRuntimeStatusResponse,
        metrics::{MetricEntry, MetricValue, MetricsKind},
        page::PageRequest,
    },
    prelude::*,
};
use ic_cdk::api::{canister_cycle_balance, canister_self, canister_version, time};
use skynet_console::{
    CanisterNode, Capability, ConsoleSnapshot, Endpoint, Fact, HttpRequest, HttpResponse,
    MetricRow, NetworkMember, NetworkRoot, NetworkService, NetworkView, NodeIdentity,
    RuntimeSummary, StandardEndpointSurface, capability, console_url, endpoint,
    endpoint_highlights, fact, option_text,
};
use std::fmt::Write;

const MEMORY_CELL_POOL: &str = "memory_cells";
const SKYNET_SERVICE: &str = "skynet";
const T800_POOL: &str = "t800_units";

canic::start!();

async fn canic_setup() {}
async fn canic_install(_: Option<Vec<u8>>) {}
async fn canic_upgrade() {}

#[canic_query(internal, public)]
fn http_request(request: HttpRequest) -> HttpResponse {
    skynet_console::response_for(request, &console_snapshot())
}

#[canic_query(requires(deployment::is_service_authority(SKYNET_SERVICE)))]
async fn skynet_command_signal() -> Result<String, Error> {
    Ok(format!(
        "SKYNET authority confirmed by protected service context at {}",
        canister_self()
    ))
}

#[canic_query(public)]
fn skynet_t800_plan() -> Result<bool, Error> {
    ScalingApi::plan_create_worker(T800_POOL)
}

#[canic_update(requires(caller::is_controller()))]
async fn skynet_t800_create() -> Result<candid::Principal, Error> {
    ScalingApi::create_worker(T800_POOL).await
}

#[canic_query(public)]
fn skynet_sector_plan(sector: String) -> Result<String, Error> {
    Ok(format!(
        "sector={sector}\nplan={:?}",
        ShardingApi::plan_assign_to_pool(MEMORY_CELL_POOL, &sector)?
    ))
}

#[canic_update(requires(caller::is_controller()))]
async fn skynet_sector_assign(sector: String) -> Result<candid::Principal, Error> {
    ShardingApi::assign_to_pool(MEMORY_CELL_POOL, sector).await
}

fn console_snapshot() -> ConsoleSnapshot {
    let canister_id = canister_self();
    let environment_snapshot = EnvQuery::snapshot();
    let runtime_status = ComponentRuntimeApi::status().ok();
    let phase = runtime_status.as_ref().map_or_else(
        || "Prepared".to_string(),
        |status| format!("{:?}", status.phase),
    );
    let observation = if runtime_status.is_some() {
        "protected Component runtime"
    } else {
        "runtime Directory pending"
    };
    let children = local_children();
    let network = network_view(runtime_status.as_ref(), canister_id);

    ConsoleSnapshot {
        schema_version: 1,
        generated_at_ns: time(),
        identity: NodeIdentity {
            codename: "Neural Net Processor".to_string(),
            role: "skynet_node".to_string(),
            canister_id: canister_id.to_text(),
            package_name: env!("CARGO_PKG_NAME").to_string(),
            package_version: env!("CARGO_PKG_VERSION").to_string(),
            canic_version: canic::VERSION.to_string(),
            canister_version: canister_version(),
        },
        runtime: RuntimeSummary {
            ready: canic_ready(),
            phase,
            cycles: canister_cycle_balance(),
            bootstrap: format!("{:?}", canic_bootstrap_status()),
            observation: observation.to_string(),
        },
        environment: environment_facts(environment_snapshot),
        deployment: deployment_facts(runtime_status.as_ref()),
        capabilities: capability_rows(runtime_status.as_ref(), &network),
        endpoints: endpoint_rows(),
        metrics: metric_rows(),
        children,
        network,
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

fn deployment_facts(status: Option<&ComponentRuntimeStatusResponse>) -> Vec<Fact> {
    let mut facts = configured_deployment_facts();
    let Some(status) = status else {
        facts.push(fact("Deployment context", "pending", "ComponentRuntimeApi"));
        return facts;
    };
    facts.extend([
        fact(
            "Runtime phase",
            format!("{:?}", status.phase),
            "protected state",
        ),
        fact("Operation ID", hex(&status.operation_id), "protected state"),
        fact(
            "Directory authority",
            status
                .authority_hash
                .map_or_else(|| "pending".to_string(), |hash| hex(&hash)),
            "protected state",
        ),
    ]);
    append_runtime_deployment_facts(&mut facts, status.deployment.as_ref());
    facts
}

fn configured_deployment_facts() -> Vec<Fact> {
    vec![
        fact("Fleet-wide node ceiling", 32, "canic.toml"),
        fact("Node initial cycles", "2T", "canic.toml"),
        fact("Node top-up policy", "below 2T add 1T", "canic.toml"),
        fact("T-800 pool", "initial 1 · range 1..4", "canic.toml"),
        fact(
            "Memory-cell pool",
            "initial 1 · maximum 4 · capacity 100",
            "canic.toml",
        ),
        fact(
            "Service placement",
            "maximum 1/root · minimum spread 8",
            "canic.toml",
        ),
    ]
}

fn append_runtime_deployment_facts(
    facts: &mut Vec<Fact>,
    deployment: &ProtectedComponentDeployment,
) {
    match deployment {
        ProtectedComponentDeployment::UngroupedOrdinary { binding } => {
            facts.push(fact("Purpose", "ordinary", "deployment plan"));
            facts.push(fact(
                "Component instance",
                binding.component.to_string(),
                "Registry binding",
            ));
        }
        ProtectedComponentDeployment::GroupMember {
            binding,
            group_placement,
            component_group,
            member_path,
            purpose,
            labels,
            limits,
            ..
        } => {
            facts.push(fact("Purpose", purpose_text(purpose), "deployment plan"));
            facts.push(fact(
                "Group placement",
                format!("{}#{}", group_placement.deployment, group_placement.ordinal),
                "deployment plan",
            ));
            facts.push(fact(
                "Component Group",
                component_group.to_string(),
                "deployment plan",
            ));
            facts.push(fact(
                "Member path",
                member_path
                    .as_slice()
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(" / "),
                "deployment plan",
            ));
            facts.push(fact(
                "Component instance",
                binding.component.to_string(),
                "Registry binding",
            ));
            facts.push(fact(
                "Maximum descendants",
                limits.maximum_descendants,
                "effective limit",
            ));
            facts.push(fact(
                "Maximum Registry bytes",
                limits.maximum_registry_bytes,
                "effective limit",
            ));
            facts.push(fact(
                "Labels",
                labels
                    .iter()
                    .map(|label| format!("{}={}", label.key, label.value))
                    .collect::<Vec<_>>()
                    .join(", "),
                "inert deployment metadata",
            ));
        }
    }
}

fn network_view(
    status: Option<&ComponentRuntimeStatusResponse>,
    current_canister: candid::Principal,
) -> NetworkView {
    let Some(authority) = status.and_then(|status| status.authority.as_ref()) else {
        return NetworkView::unavailable("protected Fleet Directory pending");
    };
    let current_root = authority.component.provenance.source_fleet_subnet_root;
    let roots = authority
        .fleet
        .fleet_subnet_roots
        .iter()
        .map(|root| NetworkRoot {
            subnet_id: root.placement_subnet.to_string(),
            root_canister_id: root.fleet_subnet_root.to_text(),
            url: console_url(root.fleet_subnet_root),
            status: format!("{:?}", root.status),
            current: root.fleet_subnet_root == current_root,
        })
        .collect();
    let services = authority
        .fleet
        .services
        .iter()
        .map(|service| NetworkService {
            service: service.service.to_string(),
            mode: format!("{:?}", service.mode),
            role: service.role.to_string(),
            maximum_members_per_root: service.placement.maximum_members_per_root,
            minimum_distinct_roots: service.placement.minimum_distinct_roots,
            members: service
                .members
                .iter()
                .map(|member| NetworkMember {
                    purpose: format!("{:?}", member.member_purpose),
                    canister_id: member.canister_id.to_text(),
                    root_canister_id: member.fleet_subnet_root.to_text(),
                    placement: format!(
                        "{}#{}",
                        member.group_placement.deployment, member.group_placement.ordinal
                    ),
                    url: console_url(member.canister_id),
                    current: member.canister_id == current_canister,
                })
                .collect(),
        })
        .collect();
    NetworkView {
        authority: "protected Fleet Directory".to_string(),
        registry_revision: Some(authority.fleet.provenance.registry.revision),
        registry_hash: Some(hex(&authority.fleet.provenance.registry.content_hash)),
        roots,
        services,
    }
}

fn local_children() -> Vec<CanisterNode> {
    CanisterChildrenApi::page(PageRequest {
        limit: 64,
        offset: 0,
    })
    .entries
    .into_iter()
    .map(|child| CanisterNode {
        canister_id: child.pid.to_text(),
        role: child.role.to_string(),
        relation: "direct child".to_string(),
        url: console_url(child.pid),
        current: false,
    })
    .collect()
}

fn capability_rows(
    status: Option<&ComponentRuntimeStatusResponse>,
    network: &NetworkView,
) -> Vec<Capability> {
    let scaling = ScalingApi::registry().0;
    let sharding = ShardingApi::registry().0;
    let authority = status.is_some_and(|status| {
        matches!(
            status.deployment.as_ref(),
            ProtectedComponentDeployment::GroupMember {
                purpose: ComponentDeploymentPurpose::FleetServiceMember {
                    member_purpose:
                        canic::dto::component_deployment::FleetServiceMemberPurpose::Authority,
                    ..
                },
                ..
            }
        )
    });
    vec![
        capability(
            "Fleet-service authority",
            if authority { "enabled" } else { "replica" },
            if authority {
                "service-authority endpoint guard passes on this exact node"
            } else {
                "write-authority guard rejects this Replica"
            },
        ),
        capability(
            "Fleet Directory",
            if network.roots.is_empty() {
                "pending"
            } else {
                "enabled"
            },
            format!("{} physical roots currently projected", network.roots.len()),
        ),
        capability(
            "Scaling pool",
            "enabled",
            format!("{} T-800 workers registered; policy 1..4", scaling.len()),
        ),
        capability(
            "Sharding pool",
            "enabled",
            format!(
                "{} memory cells registered; capacity 100 each",
                sharding.len()
            ),
        ),
        capability(
            "Metrics",
            "enabled",
            "full Core/Placement/Platform/Runtime/Security/Storage profile",
        ),
        capability(
            "Memory ledger",
            "enabled",
            "controller-only stable-memory recovery diagnostic",
        ),
        capability(
            "ICRC-21",
            "enabled",
            "consent-message standard endpoint compiled for the App",
        ),
        capability(
            "Delegated tokens",
            "disabled",
            "this public demo uses topology and controller guards only",
        ),
    ]
}

fn endpoint_rows() -> Vec<Endpoint> {
    endpoint_highlights(
        StandardEndpointSurface::Component,
        [
            endpoint(
                "canic_scaling_registry",
                "query",
                "controller",
                "complete T-800 pool registry",
            ),
            endpoint(
                "canic_sharding_partition_keys",
                "query",
                "controller",
                "memory-cell partition keys",
            ),
            endpoint(
                "canic_sharding_registry",
                "query",
                "controller",
                "complete memory-cell registry",
            ),
            endpoint(
                "skynet_command_signal",
                "query",
                "Authority only",
                "service authority proof",
            ),
            endpoint(
                "skynet_sector_assign",
                "update",
                "controller",
                "shard a sector key",
            ),
            endpoint(
                "skynet_sector_plan",
                "query",
                "public",
                "preview shard assignment",
            ),
            endpoint(
                "skynet_t800_create",
                "update",
                "controller",
                "scale one worker",
            ),
            endpoint(
                "skynet_t800_plan",
                "query",
                "public",
                "preview scale admission",
            ),
        ],
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
                limit: 12,
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

fn purpose_text(purpose: &ComponentDeploymentPurpose) -> String {
    match purpose {
        ComponentDeploymentPurpose::Ordinary => "ordinary".to_string(),
        ComponentDeploymentPurpose::FleetServiceMember {
            service,
            member_purpose,
        } => format!("{member_purpose:?} of {service}"),
    }
}

fn hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(&mut output, "{byte:02x}").expect("writing hexadecimal to a String cannot fail");
    }
    output
}

canic::finish!();
