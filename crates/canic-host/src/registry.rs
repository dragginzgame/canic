#[cfg(test)]
use candid::Encode;
use candid::{CandidType, Decode, Principal};
use serde::Deserialize;
use serde_json::Value;
use thiserror::Error as ThisError;

///
/// RegistryEntry
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegistryEntry {
    pub pid: String,
    pub role: Option<String>,
    pub kind: Option<String>,
    pub parent_pid: Option<String>,
    pub module_hash: Option<String>,
}

///
/// RegistryParseError
///

#[derive(Debug, ThisError)]
pub enum RegistryParseError {
    #[error("registry JSON must be an array, {{\"Ok\": [...]}}, or ICP response_bytes envelope")]
    InvalidRegistryJsonShape,

    #[error("registry response_bytes was not valid hex")]
    InvalidResponseBytes,

    #[error("registry response rejected: {0}")]
    Rejected(String),

    #[error("could not decode registry response_bytes: {0}")]
    Candid(String),

    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

/// Parse the wrapped subnet registry JSON shape.
pub fn parse_registry_entries(
    registry_json: &str,
) -> Result<Vec<RegistryEntry>, RegistryParseError> {
    let data = serde_json::from_str::<Value>(registry_json)?;
    if let Some(entries) = parse_registry_entries_json(&data) {
        return Ok(entries);
    }
    if let Some(entries) = parse_registry_entries_response_bytes(&data)? {
        return Ok(entries);
    }

    Err(RegistryParseError::InvalidRegistryJsonShape)
}

fn parse_registry_entries_json(data: &Value) -> Option<Vec<RegistryEntry>> {
    let entries = data
        .get("Ok")
        .and_then(Value::as_array)
        .or_else(|| data.as_array())?;

    Some(entries.iter().filter_map(parse_registry_entry).collect())
}

fn parse_registry_entries_response_bytes(
    data: &Value,
) -> Result<Option<Vec<RegistryEntry>>, RegistryParseError> {
    let Some(bytes) = data.get("response_bytes").and_then(Value::as_str) else {
        return Ok(None);
    };
    let bytes = hex_to_bytes(bytes).ok_or(RegistryParseError::InvalidResponseBytes)?;
    let response = Decode!(
        &bytes,
        Result<SubnetRegistryResponseWire, CanicErrorWire>
    )
    .map_err(|err| RegistryParseError::Candid(err.to_string()))?;
    let response = response.map_err(|err| RegistryParseError::Rejected(err.message))?;
    Ok(Some(response.to_registry_entries()))
}

// Parse one registry entry from registry JSON.
fn parse_registry_entry(value: &Value) -> Option<RegistryEntry> {
    let pid = value.get("pid").and_then(Value::as_str)?.to_string();
    let role = value
        .get("role")
        .and_then(Value::as_str)
        .map(str::to_string);
    let parent_pid = value
        .get("record")
        .and_then(|record| record.get("parent_pid"))
        .and_then(parse_optional_principal);
    let kind = value
        .get("kind")
        .or_else(|| value.get("record").and_then(|record| record.get("kind")))
        .and_then(Value::as_str)
        .map(str::to_string);
    let module_hash = value
        .get("record")
        .and_then(|record| record.get("module_hash"))
        .and_then(parse_module_hash);

    Some(RegistryEntry {
        pid,
        role,
        kind,
        parent_pid,
        module_hash,
    })
}

// Parse optional wasm module hash JSON emitted as bytes or text.
fn parse_module_hash(value: &Value) -> Option<String> {
    if value.is_null() {
        return None;
    }
    if let Some(text) = value.as_str() {
        return Some(text.to_string());
    }
    let bytes = value
        .as_array()?
        .iter()
        .map(|item| {
            let value = item.as_u64()?;
            u8::try_from(value).ok()
        })
        .collect::<Option<Vec<_>>>()?;
    Some(hex_bytes(&bytes))
}

// Parse optional principal JSON emitted as null, string, or optional vector form.
fn parse_optional_principal(value: &Value) -> Option<String> {
    if value.is_null() {
        return None;
    }
    if let Some(text) = value.as_str() {
        return Some(text.to_string());
    }
    value
        .as_array()
        .and_then(|items| items.first())
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn hex_bytes(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(char::from(HEX[usize::from(byte >> 4)]));
        out.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    out
}

fn hex_to_bytes(text: &str) -> Option<Vec<u8>> {
    if !text.len().is_multiple_of(2) {
        return None;
    }
    text.as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let high = hex_value(pair[0])?;
            let low = hex_value(pair[1])?;
            Some((high << 4) | low)
        })
        .collect()
}

const fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

///
/// SubnetRegistryResponseWire
///

#[derive(CandidType, Deserialize)]
struct SubnetRegistryResponseWire(Vec<SubnetRegistryEntryWire>);

impl SubnetRegistryResponseWire {
    fn to_registry_entries(&self) -> Vec<RegistryEntry> {
        self.0
            .iter()
            .map(SubnetRegistryEntryWire::to_registry_entry)
            .collect()
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
    fn to_registry_entry(&self) -> RegistryEntry {
        let pid = self.pid.to_text();
        let record_pid = self.record.pid.to_text();
        debug_assert_eq!(record_pid, pid);
        let role = if self.role.is_empty() {
            self.record.role.clone()
        } else {
            self.role.clone()
        };
        RegistryEntry {
            pid,
            role: Some(role),
            kind: None,
            parent_pid: self.record.parent_pid.as_ref().map(Principal::to_text),
            module_hash: self.record.module_hash.as_deref().map(hex_bytes),
        }
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
}

///
/// CanicErrorWire
///

#[derive(CandidType, Deserialize)]
struct CanicErrorWire {
    message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROOT_TEXT: &str = "aaaaa-aa";
    const APP_TEXT: &str = "renrk-eyaaa-aaaaa-aaada-cai";
    const WORKER_TEXT: &str = "rno2w-sqaaa-aaaaa-aaacq-cai";
    const HASH: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    // Ensure registry parsing accepts the wrapped registry JSON shape.
    #[test]
    fn registry_entries_parse_wrapped_cli_json() {
        let entries = parse_registry_entries(&registry_json()).expect("parse registry");

        assert_eq!(
            entries,
            vec![
                RegistryEntry {
                    pid: ROOT_TEXT.to_string(),
                    role: Some("root".to_string()),
                    kind: Some("root".to_string()),
                    parent_pid: None,
                    module_hash: None,
                },
                RegistryEntry {
                    pid: APP_TEXT.to_string(),
                    role: Some("app".to_string()),
                    kind: Some("singleton".to_string()),
                    parent_pid: Some(ROOT_TEXT.to_string()),
                    module_hash: Some("01ab".to_string()),
                },
                RegistryEntry {
                    pid: WORKER_TEXT.to_string(),
                    role: Some("worker".to_string()),
                    kind: Some("replica".to_string()),
                    parent_pid: Some(APP_TEXT.to_string()),
                    module_hash: Some(HASH.to_string()),
                },
            ]
        );
    }

    fn registry_json() -> String {
        serde_json::json!({
            "Ok": [
                {
                    "pid": ROOT_TEXT,
                    "role": "root",
                    "record": {
                        "pid": ROOT_TEXT,
                        "role": "root",
                        "kind": "root",
                        "parent_pid": null,
                        "module_hash": null
                    }
                },
                {
                    "pid": APP_TEXT,
                    "role": "app",
                    "kind": "singleton",
                    "record": {
                        "pid": APP_TEXT,
                        "role": "app",
                        "parent_pid": [ROOT_TEXT],
                        "module_hash": [1, 171]
                    }
                },
                {
                    "pid": WORKER_TEXT,
                    "role": "worker",
                    "kind": "replica",
                    "record": {
                        "pid": WORKER_TEXT,
                        "role": "worker",
                        "parent_pid": [APP_TEXT],
                        "module_hash": HASH
                    }
                }
            ]
        })
        .to_string()
    }

    #[test]
    fn registry_entries_parse_icp_response_bytes_json() {
        #[derive(CandidType)]
        struct FullSubnetRegistryEntryWire {
            pid: Principal,
            role: String,
            record: FullCanisterInfoWire,
        }

        #[derive(CandidType)]
        struct FullSubnetRegistryResponseWire(Vec<FullSubnetRegistryEntryWire>);

        #[derive(CandidType)]
        struct FullCanisterInfoWire {
            pid: Principal,
            role: String,
            parent_pid: Option<Principal>,
            module_hash: Option<Vec<u8>>,
            created_at: u64,
        }

        let response = Ok::<_, CanicErrorWire>(FullSubnetRegistryResponseWire(vec![
            FullSubnetRegistryEntryWire {
                pid: Principal::from_text(ROOT_TEXT).expect("root principal"),
                role: "root".to_string(),
                record: FullCanisterInfoWire {
                    pid: Principal::from_text(ROOT_TEXT).expect("root principal"),
                    role: "root".to_string(),
                    parent_pid: None,
                    module_hash: None,
                    created_at: 1,
                },
            },
            FullSubnetRegistryEntryWire {
                pid: Principal::from_text(APP_TEXT).expect("app principal"),
                role: "app".to_string(),
                record: FullCanisterInfoWire {
                    pid: Principal::from_text(APP_TEXT).expect("app principal"),
                    role: "app".to_string(),
                    parent_pid: Some(Principal::from_text(ROOT_TEXT).expect("root principal")),
                    module_hash: Some(vec![1, 171]),
                    created_at: 2,
                },
            },
        ]));
        let bytes = candid::Encode!(&response).expect("encode registry response");
        let payload = serde_json::json!({
            "response_bytes": hex_bytes(&bytes),
            "response_candid": "(variant { Ok = vec { ... } })"
        })
        .to_string();
        let entries = parse_registry_entries(&payload).expect("parse response bytes registry");

        assert_eq!(
            entries,
            vec![
                RegistryEntry {
                    pid: ROOT_TEXT.to_string(),
                    role: Some("root".to_string()),
                    kind: None,
                    parent_pid: None,
                    module_hash: None,
                },
                RegistryEntry {
                    pid: APP_TEXT.to_string(),
                    role: Some("app".to_string()),
                    kind: None,
                    parent_pid: Some(ROOT_TEXT.to_string()),
                    module_hash: Some("01ab".to_string()),
                },
            ]
        );
    }

    #[test]
    fn registry_entries_reject_invalid_response_bytes_hex() {
        let payload = serde_json::json!({
            "response_bytes": "not-hex"
        })
        .to_string();
        let err = parse_registry_entries(&payload).expect_err("reject invalid response bytes");

        assert!(matches!(err, RegistryParseError::InvalidResponseBytes));
    }

    #[test]
    fn registry_entries_surface_response_bytes_rejection() {
        let response = Err::<SubnetRegistryResponseWire, _>(CanicErrorWire {
            message: "not ready".into(),
        });
        let bytes = candid::Encode!(&response).expect("encode registry rejection");
        let payload = serde_json::json!({
            "response_bytes": hex_bytes(&bytes)
        })
        .to_string();
        let err = parse_registry_entries(&payload).expect_err("surface registry rejection");

        assert!(matches!(err, RegistryParseError::Rejected(message) if message == "not ready"));
    }
}
