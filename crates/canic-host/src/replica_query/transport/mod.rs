//! Module: replica_query::transport
//!
//! Responsibility: anonymous local replica reads through the existing HTTP client.
//! Does not own: Candid/CBOR contracts, readiness decisions or environment selection.
//! Boundary: exact selected-origin requests and typed transport failure projection.

use super::ReplicaQueryError;
use super::cbor::{QueryOutcome, decode_query_response, encode_anonymous_query};
use crate::icp_config::{
    DEFAULT_LOCAL_GATEWAY_PORT, configured_local_gateway_port,
    configured_local_gateway_port_from_root,
};
use candid::Principal;
use reqwest::{Method, Url, blocking::Client, redirect::Policy};
use std::{
    error::Error,
    io,
    path::Path,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

const LOCAL_HTTP_TIMEOUT: Duration = Duration::from_secs(30);

pub(super) fn local_query(
    environment: Option<&str>,
    canister: &str,
    method: &str,
    arg: &[u8],
    icp_root: Option<&Path>,
) -> Result<Vec<u8>, ReplicaQueryError> {
    let endpoint = icp_root.map_or_else(
        || local_replica_endpoint(environment),
        |root| local_replica_endpoint_from_root(environment, root),
    );
    local_query_with_endpoint(canister, method, arg, endpoint)
}

#[must_use]
pub fn local_replica_endpoint_from_root(environment: Option<&str>, icp_root: &Path) -> String {
    local_replica_endpoint_with_port(
        environment,
        configured_local_gateway_port_from_root(icp_root).ok(),
    )
}

pub(super) fn get_http_status(endpoint: &str) -> Result<Vec<u8>, ReplicaQueryError> {
    http_request(
        endpoint,
        "/api/v2/status",
        Method::GET,
        None,
        LOCAL_HTTP_TIMEOUT,
    )
}

fn local_query_with_endpoint(
    canister: &str,
    method: &str,
    arg: &[u8],
    endpoint: String,
) -> Result<Vec<u8>, ReplicaQueryError> {
    let canister_id =
        Principal::from_text(canister).map_err(|err| ReplicaQueryError::Query(err.to_string()))?;
    let body =
        encode_anonymous_query(canister_id.as_slice(), method, arg, ingress_expiry_nanos()?)?;
    let response = http_request(
        &endpoint,
        &format!("/api/v2/canister/{canister}/query"),
        Method::POST,
        Some(body),
        LOCAL_HTTP_TIMEOUT,
    )?;
    match decode_query_response(&response)? {
        QueryOutcome::Replied(arg) => Ok(arg),
        QueryOutcome::Rejected { code, message } => {
            Err(ReplicaQueryError::Rejected { code, message })
        }
    }
}

fn local_replica_endpoint(environment: Option<&str>) -> String {
    local_replica_endpoint_with_port(environment, configured_local_gateway_port().ok())
}

fn local_replica_endpoint_with_port(
    environment: Option<&str>,
    configured_port: Option<u16>,
) -> String {
    if let Some(environment) = environment.filter(|environment| environment.starts_with("http://"))
    {
        return environment.trim_end_matches('/').to_string();
    }

    let port = configured_port.unwrap_or(DEFAULT_LOCAL_GATEWAY_PORT);
    format!("http://127.0.0.1:{port}")
}

fn ingress_expiry_nanos() -> Result<u64, ReplicaQueryError> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|err| ReplicaQueryError::Query(err.to_string()))?;
    let expiry = now
        .as_nanos()
        .saturating_add(5 * 60 * 1_000_000_000)
        .min(u128::from(u64::MAX));
    u64::try_from(expiry).map_err(|err| ReplicaQueryError::Query(err.to_string()))
}

/// Keep anonymous local reads on the selected origin, without redirects or proxies.
fn http_request(
    endpoint: &str,
    path: &str,
    method: Method,
    body: Option<Vec<u8>>,
    timeout: Duration,
) -> Result<Vec<u8>, ReplicaQueryError> {
    let mut url =
        Url::parse(endpoint).map_err(|error| ReplicaQueryError::Query(error.to_string()))?;
    if url.scheme() != "http" || !url.username().is_empty() || url.password().is_some() {
        return Err(ReplicaQueryError::Query(
            "local replica endpoint must use anonymous HTTP".to_string(),
        ));
    }
    url.set_path(path);
    url.set_query(None);
    url.set_fragment(None);
    let client = Client::builder()
        .no_proxy()
        .redirect(Policy::none())
        .timeout(timeout)
        .build()
        .map_err(transport_error)?;
    let mut request = client.request(method, url);
    if let Some(body) = body {
        request = request
            .header(reqwest::header::CONTENT_TYPE, "application/cbor")
            .body(body);
    }
    let response = request.send().map_err(transport_error)?;
    if !response.status().is_success() {
        return Err(ReplicaQueryError::Query(format!(
            "local replica HTTP response: {}",
            response.status()
        )));
    }
    response
        .bytes()
        .map(|body| body.to_vec())
        .map_err(transport_error)
}

/// Retain the HTTP failure and the native I/O kind when one is supplied.
fn transport_error(error: reqwest::Error) -> ReplicaQueryError {
    let kind = if error.is_timeout() {
        io::ErrorKind::TimedOut
    } else {
        std::iter::successors(error.source(), |&source| source.source())
            .find_map(|source| source.downcast_ref::<io::Error>())
            .map_or(io::ErrorKind::Other, io::Error::kind)
    };
    ReplicaQueryError::Io(io::Error::new(kind, error))
}

#[cfg(test)]
mod tests;
