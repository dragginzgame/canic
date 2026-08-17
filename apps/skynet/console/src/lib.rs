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
    let command_purpose = match surface {
        StandardEndpointSurface::Component => "managed lifecycle and capability variants",
        StandardEndpointSurface::Root => "Root lifecycle and Fleet control variants",
    };
    let mut endpoints = vec![
        endpoint(
            "canic_command",
            "update",
            "variant-specific",
            command_purpose,
        ),
        endpoint(
            "canic_status",
            "query",
            "variant-specific",
            "bounded role, runtime and operation observations",
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
