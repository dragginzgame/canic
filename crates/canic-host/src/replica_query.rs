use candid::{CandidType, Decode, Encode, Principal};
use serde::{Deserialize, Serialize};
use std::{
    error::Error,
    fmt,
    io::{Read, Write},
    net::TcpStream,
    time::{SystemTime, UNIX_EPOCH},
};

///
/// ReplicaQueryError
///

#[derive(Debug)]
pub enum ReplicaQueryError {
    Io(std::io::Error),
    Cbor(serde_cbor::Error),
    Json(serde_json::Error),
    Query(String),
    Rejected { code: u64, message: String },
}

impl fmt::Display for ReplicaQueryError {
    // Render local replica query failures as compact operator diagnostics.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => write!(formatter, "{err}"),
            Self::Cbor(err) => write!(formatter, "{err}"),
            Self::Json(err) => write!(formatter, "{err}"),
            Self::Query(message) => write!(formatter, "{message}"),
            Self::Rejected { code, message } => {
                write!(
                    formatter,
                    "local replica rejected query: code={code} message={message}"
                )
            }
        }
    }
}

impl Error for ReplicaQueryError {
    // Preserve structured source errors for I/O and serialization failures.
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            Self::Cbor(err) => Some(err),
            Self::Json(err) => Some(err),
            Self::Query(_) | Self::Rejected { .. } => None,
        }
    }
}

impl From<std::io::Error> for ReplicaQueryError {
    // Convert local socket and process I/O failures.
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<serde_cbor::Error> for ReplicaQueryError {
    // Convert CBOR encode/decode failures.
    fn from(err: serde_cbor::Error) -> Self {
        Self::Cbor(err)
    }
}

impl From<serde_json::Error> for ReplicaQueryError {
    // Convert JSON rendering failures.
    fn from(err: serde_json::Error) -> Self {
        Self::Json(err)
    }
}

/// Return whether the selected network should use direct local replica queries.
#[must_use]
pub fn should_use_local_replica_query(network: Option<&str>) -> bool {
    network.is_none_or(|network| network == "local" || network.starts_with("http://"))
}

/// Query `canic_ready` directly through the local replica HTTP API.
pub fn query_ready(network: Option<&str>, canister: &str) -> Result<bool, ReplicaQueryError> {
    let bytes = local_query(network, canister, "canic_ready")?;
    Decode!(&bytes, bool).map_err(|err| ReplicaQueryError::Query(err.to_string()))
}

/// Parse common JSON shapes returned by command-line calls for `canic_ready`.
#[must_use]
pub fn parse_ready_json_value(data: &serde_json::Value) -> bool {
    match data {
        serde_json::Value::Bool(value) => *value,
        serde_json::Value::String(value) => value.trim() == "(true)",
        serde_json::Value::Array(values) => values.iter().any(parse_ready_json_value),
        serde_json::Value::Object(map) => map.values().any(parse_ready_json_value),
        _ => false,
    }
}

/// Query `canic_subnet_registry` and render JSON in the CLI response shape.
pub fn query_subnet_registry_json(
    network: Option<&str>,
    root: &str,
) -> Result<String, ReplicaQueryError> {
    let bytes = local_query(network, root, "canic_subnet_registry")?;
    let result = Decode!(&bytes, Result<SubnetRegistryResponseWire, CanicErrorWire>)
        .map_err(|err| ReplicaQueryError::Query(err.to_string()))?;
    let response = result.map_err(|err| ReplicaQueryError::Query(err.to_string()))?;
    serde_json::to_string(&response.to_cli_json()).map_err(ReplicaQueryError::from)
}

// Execute one anonymous query call against the local replica.
fn local_query(
    network: Option<&str>,
    canister: &str,
    method: &str,
) -> Result<Vec<u8>, ReplicaQueryError> {
    let canister_id =
        Principal::from_text(canister).map_err(|err| ReplicaQueryError::Query(err.to_string()))?;
    let arg = Encode!().map_err(|err| ReplicaQueryError::Query(err.to_string()))?;
    let sender = Principal::anonymous();
    let envelope = QueryEnvelope {
        content: QueryContent {
            request_type: "query",
            canister_id: canister_id.as_slice(),
            method_name: method,
            arg: &arg,
            sender: sender.as_slice(),
            ingress_expiry: ingress_expiry_nanos()?,
        },
    };
    let body = serde_cbor::to_vec(&envelope)?;
    let endpoint = local_replica_endpoint(network);
    let response = post_cbor(
        &endpoint,
        &format!("/api/v2/canister/{canister}/query"),
        &body,
    )?;
    let query_response = serde_cbor::from_slice::<QueryResponse>(&response)?;

    if query_response.status == "replied" {
        return query_response
            .reply
            .map(|reply| reply.arg)
            .ok_or_else(|| ReplicaQueryError::Query("missing query reply".to_string()));
    }

    Err(ReplicaQueryError::Rejected {
        code: query_response.reject_code.unwrap_or_default(),
        message: query_response.reject_message.unwrap_or_default(),
    })
}

// Resolve the local replica endpoint from explicit URL or the conventional ICP CLI local port.
fn local_replica_endpoint(network: Option<&str>) -> String {
    if let Some(network) = network.filter(|network| network.starts_with("http://")) {
        return network.trim_end_matches('/').to_string();
    }

    "http://127.0.0.1:8000".to_string()
}

// Return an ingress expiry comfortably in the near future for local queries.
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

// POST one CBOR request over simple HTTP/1.1 and return the response body.
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

// Parse the limited HTTP endpoints supported by local direct queries.
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

// Split a simple HTTP response and reject non-2xx status codes.
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
    let status_ok = header
        .lines()
        .next()
        .is_some_and(|status| status.contains(" 2"));
    if !status_ok {
        return Err(ReplicaQueryError::Query(header.to_string()));
    }
    Ok(response[index + marker.len()..].to_vec())
}

///
/// QueryEnvelope
///

#[derive(Serialize)]
struct QueryEnvelope<'a> {
    content: QueryContent<'a>,
}

///
/// QueryContent
///

#[derive(Serialize)]
struct QueryContent<'a> {
    request_type: &'static str,
    #[serde(with = "serde_bytes")]
    canister_id: &'a [u8],
    method_name: &'a str,
    #[serde(with = "serde_bytes")]
    arg: &'a [u8],
    #[serde(with = "serde_bytes")]
    sender: &'a [u8],
    ingress_expiry: u64,
}

///
/// QueryResponse
///

#[derive(Deserialize)]
struct QueryResponse {
    status: String,
    reply: Option<QueryReply>,
    reject_code: Option<u64>,
    reject_message: Option<String>,
}

///
/// QueryReply
///

#[derive(Deserialize)]
struct QueryReply {
    #[serde(with = "serde_bytes")]
    arg: Vec<u8>,
}

///
/// SubnetRegistryResponseWire
///

#[derive(CandidType, Deserialize)]
struct SubnetRegistryResponseWire(Vec<SubnetRegistryEntryWire>);

impl SubnetRegistryResponseWire {
    // Convert direct Candid query output into the command JSON shape the discovery parser accepts.
    fn to_cli_json(&self) -> serde_json::Value {
        serde_json::json!({
            "Ok": self.0.iter().map(SubnetRegistryEntryWire::to_cli_json).collect::<Vec<_>>()
        })
    }
}

///
/// SubnetRegistryEntryWire
///

#[derive(CandidType, Deserialize)]
struct SubnetRegistryEntryWire {
    pid: Principal,
    role: String,
    record: CanisterInfoWire,
}

impl SubnetRegistryEntryWire {
    // Convert one registry entry into the command JSON shape used by existing list rendering.
    fn to_cli_json(&self) -> serde_json::Value {
        serde_json::json!({
            "pid": self.pid.to_text(),
            "role": self.role,
            "record": self.record.to_cli_json(),
        })
    }
}

///
/// CanisterInfoWire
///

#[derive(CandidType, Deserialize)]
struct CanisterInfoWire {
    pid: Principal,
    role: String,
    parent_pid: Option<Principal>,
    module_hash: Option<Vec<u8>>,
    created_at: u64,
}

impl CanisterInfoWire {
    // Convert one canister info record into a CLI-like JSON object.
    fn to_cli_json(&self) -> serde_json::Value {
        serde_json::json!({
            "pid": self.pid.to_text(),
            "role": self.role,
            "parent_pid": self.parent_pid.as_ref().map(Principal::to_text),
            "module_hash": self.module_hash,
            "created_at": self.created_at.to_string(),
        })
    }
}

///
/// CanicErrorWire
///

#[derive(CandidType, Deserialize)]
struct CanicErrorWire {
    code: ErrorCodeWire,
    message: String,
}

impl fmt::Display for CanicErrorWire {
    // Render a compact public API error from a direct local replica query.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:?}: {}", self.code, self.message)
    }
}

///
/// ErrorCodeWire
///

#[derive(CandidType, Debug, Deserialize)]
enum ErrorCodeWire {
    Conflict,
    Forbidden,
    Internal,
    InvalidInput,
    InvariantViolation,
    NotFound,
    PolicyInstanceRequiresSingletonWithDirectory,
    PolicyReplicaRequiresSingletonWithScaling,
    PolicyRoleAlreadyRegistered,
    PolicyShardRequiresSingletonWithSharding,
    PolicySingletonAlreadyRegisteredUnderParent,
    ResourceExhausted,
    Unauthorized,
    Unavailable,
}

#[cfg(test)]
mod tests {
    use super::*;

    // Ensure readiness parsing accepts common command-line JSON result shapes.
    #[test]
    fn parse_ready_json_value_accepts_nested_true_shapes() {
        assert!(parse_ready_json_value(&serde_json::json!(true)));
        assert!(parse_ready_json_value(&serde_json::json!({ "Ok": true })));
        assert!(parse_ready_json_value(&serde_json::json!([{ "Ok": true }])));
        assert!(parse_ready_json_value(&serde_json::json!({
            "response_candid": "(true)"
        })));
    }

    // Ensure readiness parsing rejects false and non-boolean result shapes.
    #[test]
    fn parse_ready_json_value_rejects_false_shapes() {
        assert!(!parse_ready_json_value(&serde_json::json!(false)));
        assert!(!parse_ready_json_value(&serde_json::json!({ "Ok": false })));
        assert!(!parse_ready_json_value(&serde_json::json!("true")));
    }

    // Ensure direct local queries use the ICP CLI local endpoint by default.
    #[test]
    fn local_replica_endpoint_defaults_to_icp_cli_port() {
        assert_eq!(local_replica_endpoint(None), "http://127.0.0.1:8000");
        assert_eq!(
            local_replica_endpoint(Some("http://127.0.0.1:9000/")),
            "http://127.0.0.1:9000"
        );
    }
}
