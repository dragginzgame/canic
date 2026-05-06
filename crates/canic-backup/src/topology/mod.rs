use candid::Principal;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

///
/// TopologyRecord
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TopologyRecord {
    pub pid: Principal,
    pub parent_pid: Option<Principal>,
    pub role: String,
    pub module_hash: Option<String>,
}

///
/// TopologyHash
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TopologyHash {
    pub algorithm: String,
    pub input: String,
    pub hash: String,
}

///
/// TopologyHasher
///

pub struct TopologyHasher;

impl TopologyHasher {
    /// Compute the canonical SHA-256 topology hash for discovery invariants.
    #[must_use]
    pub fn hash(records: &[TopologyRecord]) -> TopologyHash {
        let input = Self::canonical_input(records);
        let hash = sha256_hex(input.as_bytes());

        TopologyHash {
            algorithm: "sha256".to_string(),
            input: "sorted(pid,parent_pid,role,module_hash)".to_string(),
            hash,
        }
    }

    /// Build the stable canonical topology input used by `hash`.
    #[must_use]
    pub fn canonical_input(records: &[TopologyRecord]) -> String {
        let mut rows = records.iter().map(canonical_row).collect::<Vec<_>>();
        rows.sort();
        rows.join("\n")
    }
}

// Encode one topology record with explicit null markers for optional fields.
fn canonical_row(record: &TopologyRecord) -> String {
    format!(
        "pid={}|parent_pid={}|role={}|module_hash={}",
        record.pid,
        optional_principal(record.parent_pid),
        record.role,
        optional_str(record.module_hash.as_deref())
    )
}

// Encode optional principals with a stable null marker.
fn optional_principal(value: Option<Principal>) -> String {
    value.map_or_else(|| "null".to_string(), |pid| pid.to_string())
}

// Encode optional string fields with a stable null marker.
fn optional_str(value: Option<&str>) -> &str {
    value.unwrap_or("null")
}

// Compute lowercase hexadecimal SHA-256 without adding another dependency.
fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(digest.len() * 2);
    for byte in digest {
        out.push(hex_char(byte >> 4));
        out.push(hex_char(byte & 0x0f));
    }
    out
}

// Convert one four-bit nibble to lowercase hexadecimal.
const fn hex_char(nibble: u8) -> char {
    match nibble {
        0..=9 => (b'0' + nibble) as char,
        10..=15 => (b'a' + (nibble - 10)) as char,
        _ => unreachable!(),
    }
}

#[cfg(test)]
mod tests;
