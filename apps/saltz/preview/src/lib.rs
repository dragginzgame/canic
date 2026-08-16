//! Module: saltz_preview
//!
//! Responsibility: expose the immutable Saltz waveform as one read-only HTTP query.
//! Does not own: Fleet behavior, scheduling, authorization, stable state, or cycle burning.
//! Boundary: every request delegates to a pure router over build-verified static evidence.

mod http;
mod render;
#[cfg(test)]
mod tests;

use ic_cdk::api::canister_self;

use crate::http::{HttpRequest, HttpResponse};

#[ic_cdk::query]
fn http_request(request: HttpRequest) -> HttpResponse {
    http::response_for(request, &canister_self().to_text())
}

ic_cdk::export_candid!();
