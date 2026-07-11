use super::ReplicaQueryError;
use super::cbor::{QueryOutcome, decode_query_response, encode_anonymous_query};
use crate::icp_config::{
    DEFAULT_LOCAL_GATEWAY_PORT, configured_local_gateway_port,
    configured_local_gateway_port_from_root,
};
use candid::{Encode, Principal};
use std::{
    io::{Read, Write},
    net::TcpStream,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

pub(super) fn local_query(
    network: Option<&str>,
    canister: &str,
    method: &str,
) -> Result<Vec<u8>, ReplicaQueryError> {
    local_query_with_endpoint(canister, method, local_replica_endpoint(network))
}

pub(super) fn local_query_from_root(
    network: Option<&str>,
    canister: &str,
    method: &str,
    icp_root: &Path,
) -> Result<Vec<u8>, ReplicaQueryError> {
    local_query_with_endpoint(
        canister,
        method,
        local_replica_endpoint_from_root(network, icp_root),
    )
}

#[must_use]
pub fn local_replica_endpoint_from_root(network: Option<&str>, icp_root: &Path) -> String {
    local_replica_endpoint_with_port(
        network,
        configured_local_gateway_port_from_root(icp_root).ok(),
    )
}

pub(super) fn get_http_status(endpoint: &str) -> Result<Vec<u8>, ReplicaQueryError> {
    let (host, port) = parse_http_endpoint(endpoint)?;
    let mut stream = TcpStream::connect((host.as_str(), port))?;
    let request =
        format!("GET /api/v2/status HTTP/1.1\r\nHost: {host}:{port}\r\nConnection: close\r\n\r\n");
    stream.write_all(request.as_bytes())?;

    let mut response = Vec::new();
    stream.read_to_end(&mut response)?;
    split_http_body(&response)
}

fn local_query_with_endpoint(
    canister: &str,
    method: &str,
    endpoint: String,
) -> Result<Vec<u8>, ReplicaQueryError> {
    let canister_id =
        Principal::from_text(canister).map_err(|err| ReplicaQueryError::Query(err.to_string()))?;
    let arg = Encode!().map_err(|err| ReplicaQueryError::Query(err.to_string()))?;
    let body = encode_anonymous_query(
        canister_id.as_slice(),
        method,
        &arg,
        ingress_expiry_nanos()?,
    )?;
    let response = post_cbor(
        &endpoint,
        &format!("/api/v2/canister/{canister}/query"),
        &body,
    )?;
    match decode_query_response(&response)? {
        QueryOutcome::Replied(arg) => Ok(arg),
        QueryOutcome::Rejected { code, message } => {
            Err(ReplicaQueryError::Rejected { code, message })
        }
    }
}

fn local_replica_endpoint(network: Option<&str>) -> String {
    local_replica_endpoint_with_port(network, configured_local_gateway_port().ok())
}

fn local_replica_endpoint_with_port(network: Option<&str>, configured_port: Option<u16>) -> String {
    if let Some(network) = network.filter(|network| network.starts_with("http://")) {
        return network.trim_end_matches('/').to_string();
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

fn post_cbor(endpoint: &str, path: &str, body: &[u8]) -> Result<Vec<u8>, ReplicaQueryError> {
    let (host, port) = parse_http_endpoint(endpoint)?;
    let mut stream = TcpStream::connect((host.as_str(), port))?;
    let request = format!(
        "POST {path} HTTP/1.1\r\nHost: {host}:{port}\r\nContent-Type: application/cbor\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(request.as_bytes())?;
    stream.write_all(body)?;

    let mut response = Vec::new();
    stream.read_to_end(&mut response)?;
    split_http_body(&response)
}

fn parse_http_endpoint(endpoint: &str) -> Result<(String, u16), ReplicaQueryError> {
    let rest = endpoint
        .strip_prefix("http://")
        .ok_or_else(|| ReplicaQueryError::Query(format!("unsupported endpoint {endpoint}")))?;
    let authority = rest.split('/').next().unwrap_or(rest);
    let (host, port) = authority
        .rsplit_once(':')
        .ok_or_else(|| ReplicaQueryError::Query(format!("missing port in {endpoint}")))?;
    let port = port
        .parse::<u16>()
        .map_err(|err| ReplicaQueryError::Query(err.to_string()))?;
    Ok((host.to_string(), port))
}

fn split_http_body(response: &[u8]) -> Result<Vec<u8>, ReplicaQueryError> {
    let marker = b"\r\n\r\n";
    let Some(index) = response
        .windows(marker.len())
        .position(|window| window == marker)
    else {
        return Err(ReplicaQueryError::Query(
            "malformed HTTP response".to_string(),
        ));
    };
    let header = String::from_utf8_lossy(&response[..index]);
    let status = header
        .lines()
        .next()
        .and_then(|line| line.split_ascii_whitespace().nth(1))
        .and_then(|status| status.parse::<u16>().ok());
    if !status.is_some_and(|status| (200..300).contains(&status)) {
        return Err(ReplicaQueryError::Query(header.to_string()));
    }
    Ok(response[index + marker.len()..].to_vec())
}

#[cfg(test)]
mod tests;
