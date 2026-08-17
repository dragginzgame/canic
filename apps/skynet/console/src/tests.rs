//! Module: skynet_console::tests
//!
//! Responsibility: verify HTML, JSON, routing, escaping, and HEAD behavior.
//! Does not own: canister integration or Canic runtime observation.
//! Boundary: exercises the framework-independent console renderer.

use crate::*;

fn snapshot() -> ConsoleSnapshot {
    ConsoleSnapshot {
        schema_version: 1,
        generated_at_ns: 42,
        identity: NodeIdentity {
            codename: "T-800 <unit>".to_string(),
            role: "t800".to_string(),
            canister_id: "aaaaa-aa".to_string(),
            package_name: "skynet_t800".to_string(),
            package_version: "0.1.0".to_string(),
            canic_version: "0.101.43".to_string(),
            canister_version: 7,
        },
        runtime: RuntimeSummary {
            ready: true,
            phase: "Active".to_string(),
            cycles: 2_500_000_000_000,
            bootstrap: "Ready".to_string(),
            observation: "protected runtime".to_string(),
        },
        environment: vec![Fact {
            name: "Subnet".to_string(),
            value: "subnet-1".to_string(),
            source: "canic_env".to_string(),
        }],
        deployment: Vec::new(),
        capabilities: vec![Capability {
            name: "Metrics".to_string(),
            status: "enabled".to_string(),
            detail: "full profile".to_string(),
        }],
        endpoints: vec![Endpoint {
            name: "canic_status".to_string(),
            mode: "query".to_string(),
            access: "public".to_string(),
            purpose: "runtime readiness".to_string(),
        }],
        metrics: vec![MetricRow {
            tier: "runtime".to_string(),
            labels: vec!["query".to_string()],
            principal: None,
            value: "9".to_string(),
        }],
        children: Vec::new(),
        network: NetworkView {
            authority: "protected Fleet Directory".to_string(),
            registry_revision: Some(3),
            registry_hash: Some("abcd".to_string()),
            roots: vec![NetworkRoot {
                subnet_id: "subnet-1".to_string(),
                root_canister_id: "root-1".to_string(),
                url: "https://root-1.raw.icp0.io/".to_string(),
                status: "Active".to_string(),
                current: true,
            }],
            services: vec![NetworkService {
                service: "skynet".to_string(),
                mode: "AuthorityReplica".to_string(),
                role: "skynet_node".to_string(),
                maximum_members_per_root: 1,
                minimum_distinct_roots: 8,
                members: vec![NetworkMember {
                    purpose: "Authority".to_string(),
                    canister_id: "aaaaa-aa".to_string(),
                    root_canister_id: "root-1".to_string(),
                    placement: "skynet_authority#0".to_string(),
                    url: "https://aaaaa-aa.raw.icp0.io/".to_string(),
                    current: true,
                }],
            }],
        },
    }
}

fn request(method: &str, url: &str) -> HttpRequest {
    HttpRequest {
        method: method.to_string(),
        url: url.to_string(),
        headers: Vec::new(),
        body: Vec::new(),
    }
}

#[test]
fn page_renders_live_topology_and_escapes_observed_text() {
    let response = response_for(request("GET", "/"), &snapshot());
    let body = String::from_utf8(response.body).expect("HTML is UTF-8");

    assert_eq!(response.status_code, 200);
    assert!(body.contains("Global neural-net topology"));
    assert!(body.contains("skynet_authority#0"));
    assert!(body.contains("https://aaaaa-aa.raw.icp0.io/"));
    assert!(body.contains("https://root-1.raw.icp0.io/"));
    assert!(body.contains("T-800 &lt;unit&gt;"));
    assert!(!body.contains("T-800 <unit>"));
}

#[test]
fn json_route_returns_the_complete_snapshot() {
    let response = response_for(request("GET", "/api/status.json"), &snapshot());
    let value: serde_json::Value =
        serde_json::from_slice(&response.body).expect("snapshot JSON is valid");

    assert_eq!(response.status_code, 200);
    assert_eq!(value["schema_version"], 1);
    assert_eq!(value["network"]["registry_revision"], 3);
    assert_eq!(
        value["network"]["services"][0]["members"][0]["purpose"],
        "Authority"
    );
}

#[test]
fn head_preserves_content_length_without_returning_the_body() {
    let get = response_for(request("GET", "/"), &snapshot());
    let head = response_for(request("HEAD", "/"), &snapshot());

    assert!(head.body.is_empty());
    assert_eq!(head.status_code, 200);
    assert_eq!(
        head.headers
            .iter()
            .find(|(name, _)| name == "Content-Length"),
        get.headers
            .iter()
            .find(|(name, _)| name == "Content-Length")
    );
}

#[test]
fn unsupported_method_and_unknown_path_fail_closed() {
    assert_eq!(
        response_for(request("POST", "/"), &snapshot()).status_code,
        405
    );
    assert_eq!(
        response_for(request("GET", "/missing"), &snapshot()).status_code,
        404
    );
}

#[test]
fn endpoint_highlights_are_centralized_sorted_and_consolidated() {
    let component = endpoint_highlights(
        StandardEndpointSurface::Component,
        [endpoint("skynet_probe", "query", "public", "demo probe")],
    );
    let root = endpoint_highlights(StandardEndpointSurface::Root, []);

    assert!(component.is_sorted_by(|left, right| left.name <= right.name));
    assert!(component.iter().any(|entry| entry.name == "canic_command"));
    assert!(component.iter().any(|entry| entry.name == "canic_status"));
    assert!(component.iter().any(|entry| entry.name == "skynet_probe"));
    assert_eq!(
        root.iter()
            .filter(|entry| entry.name == "canic_status")
            .count(),
        1
    );
}
