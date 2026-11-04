//! Module: ingress::payload_contract
//!
//! Responsibility: describe compiled payload policy in the existing Candid artifact.
//! Does not own: endpoint enumeration, enforcement, authorization or a network endpoint.
//! Boundary: the declaration pass reads the same registrations used by runtime inspection.

#[cfg(test)]
mod tests;

use ic_wire::Limits;
use serde::{Deserialize, Serialize};

/// Dedicated machine field in a Candid comment; ordinary documentation is never parsed.
pub const CANDID_PAYLOAD_CONTRACT_PREFIX: &str = "// canic:payload-contract ";

/// Current compiled metadata identity, independent of RPC and stable-value frames.
pub const PAYLOAD_CONTRACT_HEADER: [u8; 4] = *b"CPW\x01";

/// Bound one metadata record; its hexadecimal comment carrier is at most 1 MiB.
pub const PAYLOAD_CONTRACT_LIMITS: Limits = Limits {
    bytes: 512 * 1024,
    allocation: 16 * 1024 * 1024,
    nodes: 65_536,
    depth: 16,
};

///
/// PayloadContract
///
/// Compiled inspector defaults and update guards, carried by the declaration artifact.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PayloadContract {
    pub schema_version: u8,
    pub default_update_ingress_max_bytes: u64,
    pub update_overrides: Vec<UpdatePayloadDescriptor>,
}

///
/// UpdatePayloadDescriptor
///
/// An explicit macro declaration also guards calls that bypass ingress inspection.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct UpdatePayloadDescriptor {
    pub method: String,
    pub max_bytes: u64,
}

/// Append exact compiled metadata without changing the Candid service schema.
///
/// # Panics
/// Panics on duplicate runtime registrations or an impossible in-memory encoding failure.
#[must_use]
pub fn annotate_candid(mut candid: String) -> String {
    let contract = PayloadContract {
        schema_version: 1,
        default_update_ingress_max_bytes: super::payload::DEFAULT_UPDATE_INGRESS_MAX_BYTES as u64,
        update_overrides: super::payload::declaration_limits(),
    };
    let bytes = ic_wire::native::encode(
        &(PAYLOAD_CONTRACT_HEADER, contract),
        PAYLOAD_CONTRACT_LIMITS,
    )
    .expect("compiled payload contract fits its declared limits");
    candid.push('\n');
    candid.push_str(CANDID_PAYLOAD_CONTRACT_PREFIX);
    candid.push_str(&crate::cdk::utils::hash::hex_bytes(&bytes));
    candid.push('\n');
    candid
}
