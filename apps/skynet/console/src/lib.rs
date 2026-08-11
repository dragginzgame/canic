//! Module: skynet_console
//!
//! Responsibility: render the shared Skynet HTML console and public JSON snapshot.
//! Does not own: Canic runtime access, Fleet discovery, authorization, or application state.
//! Boundary: role canisters supply sanitized observations and expose the returned HTTP response.

mod model;
mod render;
#[cfg(test)]
mod tests;

use candid::{CandidType, Deserialize};

pub use model::{
    CanisterNode, Capability, ConsoleSnapshot, Endpoint, Fact, MetricRow, NetworkMember,
    NetworkRoot, NetworkService, NetworkView, NodeIdentity, RuntimeSummary,
};

/// Built-in Canic endpoint surface represented by one console role.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StandardEndpointSurface {
    Component,
    Root,
}

/// Construct one presentation fact without coupling callers to its storage shape.
#[must_use]
pub fn fact(name: impl Into<String>, value: impl ToString, source: impl Into<String>) -> Fact {
    Fact {
        name: name.into(),
        value: value.to_string(),
        source: source.into(),
    }
}

/// Construct one presentation capability row.
#[must_use]
pub fn capability(
    name: impl Into<String>,
    status: impl Into<String>,
    detail: impl ToString,
) -> Capability {
    Capability {
        name: name.into(),
        status: status.into(),
        detail: detail.to_string(),
    }
}

/// Construct one presentation endpoint row.
#[must_use]
pub fn endpoint(name: &str, mode: &str, access: &str, purpose: &str) -> Endpoint {
    Endpoint {
        name: name.to_string(),
        mode: mode.to_string(),
        access: access.to_string(),
        purpose: purpose.to_string(),
    }
}

/// Build one sorted endpoint highlight list from the standard surface and role additions.
#[must_use]
pub fn endpoint_highlights(
    surface: StandardEndpointSurface,
    additions: impl IntoIterator<Item = Endpoint>,
) -> Vec<Endpoint> {
    let mut endpoints = vec![
        endpoint(
            "canic_bootstrap_status",
            "query",
            "public",
            "bootstrap state",
        ),
        endpoint(
            "canic_canister_children",
            "query",
            "public",
            "direct-child topology",
        ),
        endpoint("canic_cycle_balance", "query", "public", "cycle reserve"),
        endpoint(
            "canic_cycle_topups",
            "query",
            "public",
            "cycle top-up history",
        ),
        endpoint(
            "canic_cycle_tracker",
            "query",
            "public",
            "cycle tracking observations",
        ),
        endpoint(
            "canic_fleet_activation_status",
            "query",
            "controller",
            "Fleet activation state",
        ),
        endpoint("canic_health", "query", "controller", "runtime health"),
        endpoint(
            "canic_metadata",
            "query",
            "public",
            "package and Canic versions",
        ),
        endpoint("canic_metrics", "query", "public", "paginated metric tiers"),
        endpoint("canic_ready", "query", "public", "readiness barrier"),
        endpoint(
            "canic_readiness",
            "query",
            "controller",
            "readiness diagnostics",
        ),
        endpoint(
            "canic_runtime_status",
            "query",
            "controller",
            "complete runtime status",
        ),
        endpoint(
            "http_request",
            "query",
            "public",
            "Skynet HTML/JSON console",
        ),
        endpoint(
            "icrc10_supported_standards",
            "query",
            "public",
            "supported standards",
        ),
    ];
    if surface == StandardEndpointSurface::Component {
        endpoints.extend([
            endpoint(
                "canic_component_runtime_status",
                "query",
                "Fleet root",
                "protected Component activation and Directory",
            ),
            endpoint(
                "canic_managed_canister_binding",
                "query",
                "controller",
                "protected parent and Component binding",
            ),
            endpoint(
                "canic_memory_ledger",
                "query",
                "controller",
                "stable-memory ABI ledger",
            ),
        ]);
    }
    endpoints.extend(additions);
    endpoints.sort_by(|left, right| left.name.cmp(&right.name));
    endpoints
}

/// Render one optional observed value without inventing a default.
#[must_use]
pub fn option_text<T: ToString>(value: Option<T>) -> String {
    value.map_or_else(|| "unavailable".to_string(), |value| value.to_string())
}

/// Return the canonical raw-gateway console URL for one Canister.
#[must_use]
pub fn console_url(canister_id: candid::Principal) -> String {
    format!("https://{canister_id}.raw.icp0.io/")
}

const JSON_PATH: &str = "/api/status.json";

///
/// HttpRequest
///
/// Internet Computer HTTP gateway request accepted by every Skynet console.
///

#[derive(CandidType, Deserialize)]
pub struct HttpRequest {
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

///
/// HttpResponse
///
/// Internet Computer HTTP gateway response returned by every Skynet console.
///

#[derive(CandidType, Deserialize)]
pub struct HttpResponse {
    pub status_code: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

/// Render the appropriate console resource for one HTTP gateway request.
#[must_use]
pub fn response_for(request: HttpRequest, snapshot: &ConsoleSnapshot) -> HttpResponse {
    if request.method != "GET" && request.method != "HEAD" {
        return response(
            false,
            405,
            "text/plain; charset=utf-8",
            b"method not allowed",
        );
    }

    let head = request.method == "HEAD";
    let path = request.url.split('?').next().unwrap_or("/");
    let response = match path {
        "/" => response(
            head,
            200,
            "text/html; charset=utf-8",
            render::page(snapshot).as_bytes(),
        ),
        JSON_PATH => match serde_json::to_vec_pretty(snapshot) {
            Ok(body) => response(head, 200, "application/json; charset=utf-8", &body),
            Err(_) => response(
                head,
                500,
                "application/json; charset=utf-8",
                br#"{"error":"snapshot_serialization_failed"}"#,
            ),
        },
        _ => response(head, 404, "text/plain; charset=utf-8", b"not found"),
    };

    let _ = (request.headers, request.body);
    response
}

fn response(head: bool, status_code: u16, content_type: &str, body: &[u8]) -> HttpResponse {
    HttpResponse {
        status_code,
        headers: vec![
            ("Content-Type".to_string(), content_type.to_string()),
            ("Content-Length".to_string(), body.len().to_string()),
            ("Cache-Control".to_string(), "no-store".to_string()),
            (
                "Content-Security-Policy".to_string(),
                "default-src 'none'; style-src 'unsafe-inline'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'".to_string(),
            ),
            ("Referrer-Policy".to_string(), "no-referrer".to_string()),
            ("X-Content-Type-Options".to_string(), "nosniff".to_string()),
            ("X-Robots-Tag".to_string(), "noindex, nofollow".to_string()),
        ],
        body: if head { Vec::new() } else { body.to_vec() },
    }
}
