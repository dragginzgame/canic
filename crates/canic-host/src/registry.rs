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
    #[error("registry JSON must be an array or {{\"Ok\": [...]}}")]
    InvalidRegistryJsonShape,

    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

/// Parse the wrapped subnet registry JSON shape.
pub fn parse_registry_entries(
    registry_json: &str,
) -> Result<Vec<RegistryEntry>, RegistryParseError> {
    let data = serde_json::from_str::<Value>(registry_json)?;
    let entries = data
        .get("Ok")
        .and_then(Value::as_array)
        .or_else(|| data.as_array())
        .ok_or(RegistryParseError::InvalidRegistryJsonShape)?;

    Ok(entries.iter().filter_map(parse_registry_entry).collect())
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
}
