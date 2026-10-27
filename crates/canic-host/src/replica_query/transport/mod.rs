//! Module: replica_query::transport
//!
//! Responsibility: anonymous local replica reads through the existing HTTP client.
//! Does not own: Candid/CBOR contracts, readiness decisions or environment selection.
//! Boundary: exact selected-origin requests and typed transport failure projection.

use super::ReplicaQueryError;
use crate::icp_config::{
    DEFAULT_LOCAL_GATEWAY_PORT, configured_local_gateway_port,
    configured_local_gateway_port_from_root,
};
use ic_platform::{
    agent::{Agent, AgentError, TransportError},
    principal::ExternalPrincipal as Principal,
};
use reqwest::{Method, Url, blocking::Client, redirect::Policy};
use std::{
    error::Error,
    io::{self, Read},
    path::Path,
    sync::Arc,
    time::Duration,
};

const LOCAL_HTTP_TIMEOUT: Duration = Duration::from_secs(30);
// Allow a full replica query envelope, matching the authenticated read transport.
const LOCAL_HTTP_RESPONSE_BYTES: usize = 8 * 1024 * 1024;

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
    local_query_with_timeout(canister, method, arg, &endpoint, LOCAL_HTTP_TIMEOUT)
}

fn local_query_with_timeout(
    canister: &str,
    method: &str,
    arg: &[u8],
    endpoint: &str,
    timeout: Duration,
) -> Result<Vec<u8>, ReplicaQueryError> {
    let canister_id =
        Principal::from_text(canister).map_err(|err| ReplicaQueryError::Query(err.to_string()))?;
    let endpoint = local_http_url(endpoint, "/")?;
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .thread_name("canic-local-query")
        .enable_all()
        .build()?;
    let result = runtime.block_on(async {
        let client = reqwest::Client::builder()
            .no_proxy()
            .redirect(Policy::none())
            .build()
            .map_err(transport_error)?;
        let agent = Agent::builder()
            .with_url(endpoint.as_str())
            .with_arc_http_middleware(Arc::new(client))
            .with_max_tcp_error_retries(0)
            .with_max_response_body_size(LOCAL_HTTP_RESPONSE_BYTES)
            .with_verify_query_signatures(false)
            .build()
            .map_err(agent_error)?;
        tokio::time::timeout(
            timeout,
            agent
                .query(&canister_id, method)
                .with_arg(arg)
                .expire_after(Duration::from_mins(5))
                .call(),
        )
        .await
        .map_err(|error| ReplicaQueryError::Io(io::Error::new(io::ErrorKind::TimedOut, error)))?
        .map_err(agent_error)
    });
    // A synchronous caller can itself run on another runtime; do not block its shutdown.
    runtime.shutdown_background();
    result
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

/// Keep anonymous local reads on the selected origin, without redirects or proxies.
fn http_request(
    endpoint: &str,
    path: &str,
    method: Method,
    body: Option<Vec<u8>>,
    timeout: Duration,
) -> Result<Vec<u8>, ReplicaQueryError> {
    let url = local_http_url(endpoint, path)?;
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
    let mut body = Vec::new();
    // Read one extra byte so an oversized stream cannot look like a valid prefix.
    response
        .take(LOCAL_HTTP_RESPONSE_BYTES as u64 + 1)
        .read_to_end(&mut body)
        .map_err(response_read_error)?;
    if body.len() > LOCAL_HTTP_RESPONSE_BYTES {
        return Err(ReplicaQueryError::Io(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("local replica HTTP response exceeds {LOCAL_HTTP_RESPONSE_BYTES} bytes"),
        )));
    }
    Ok(body)
}

/// Preserve reqwest timeout classification when its streaming reader wraps an error.
fn response_read_error(error: io::Error) -> ReplicaQueryError {
    let kind = error.kind();
    match error.into_inner() {
        Some(source) => match source.downcast::<reqwest::Error>() {
            Ok(error) => transport_error(*error),
            Err(source) => ReplicaQueryError::Io(io::Error::new(kind, source)),
        },
        None => ReplicaQueryError::Io(io::Error::from(kind)),
    }
}

fn local_http_url(endpoint: &str, path: &str) -> Result<Url, ReplicaQueryError> {
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
    Ok(url)
}

fn agent_error(error: AgentError) -> ReplicaQueryError {
    match error {
        AgentError::UncertifiedReject { reject, .. } => ReplicaQueryError::Rejected {
            code: reject.reject_code as u64,
            message: reject.reject_message,
        },
        AgentError::TransportError(TransportError::Reqwest(error)) => transport_error(error),
        other => ReplicaQueryError::Agent(Box::new(other)),
    }
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
