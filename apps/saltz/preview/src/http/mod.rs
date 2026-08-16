//! Module: saltz_preview::http
//!
//! Responsibility: route the standalone preview's bounded read-only HTTP resources.
//! Does not own: waveform compilation, page design, mutable state, or update calls.
//! Boundary: accepts only GET/HEAD and serves bytes already embedded in the Wasm.

use candid::{CandidType, Deserialize};

use crate::render;

const CSV: &[u8] =
    include_bytes!("../../../../../docs/design/ideas/saltz/saltz_24h_waveform_floor_100B_860.csv");

/// Internet Computer HTTP-gateway request accepted by the preview canister.
#[derive(CandidType, Deserialize)]
pub struct HttpRequest {
    pub body: Vec<u8>,
    pub headers: Vec<(String, String)>,
    pub method: String,
    pub url: String,
}

/// Internet Computer HTTP-gateway response returned by the preview canister.
#[derive(CandidType, Deserialize)]
pub struct HttpResponse {
    pub body: Vec<u8>,
    pub headers: Vec<(String, String)>,
    pub status_code: u16,
}

pub fn response_for(request: HttpRequest, canister_id: &str) -> HttpResponse {
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
            render::page(canister_id).as_bytes(),
        ),
        "/api/status.json" => response(
            head,
            200,
            "application/json; charset=utf-8",
            render::status_json(canister_id).as_bytes(),
        ),
        "/waveform.csv" => response(head, 200, "text/csv; charset=utf-8", CSV),
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
        ],
        body: if head { Vec::new() } else { body.to_vec() },
    }
}
