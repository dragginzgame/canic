//! Module: ingress::payload_contract
//!
//! Responsibility: describe compiled payload policy in the existing Candid artifact.
//! Does not own: endpoint enumeration, enforcement, authorization or a network endpoint.
//! Boundary: the declaration pass reads the same registrations used by runtime inspection.

#[cfg(test)]
mod tests;

use serde::{Deserialize, Serialize};

/// Dedicated machine field in a Candid comment; ordinary documentation is never parsed.
pub const CANDID_PAYLOAD_CONTRACT_PREFIX: &str = "// canic:payload-contract ";

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
    pub variant_dependent_methods: Vec<String>,
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
pub fn annotate_candid(mut candid: String, variant_dependent_methods: &[&str]) -> String {
    let contract = PayloadContract {
        schema_version: 1,
        default_update_ingress_max_bytes: super::payload::DEFAULT_UPDATE_INGRESS_MAX_BYTES as u64,
        update_overrides: super::payload::declaration_limits(),
        variant_dependent_methods: variant_dependent_methods
            .iter()
            .map(|method| (*method).into())
            .collect(),
    };
    let mut bytes = Vec::new();
    ciborium::ser::into_writer(&contract, &mut bytes).expect("compiled payload contract encodes");
    candid.push('\n');
    candid.push_str(CANDID_PAYLOAD_CONTRACT_PREFIX);
    candid.push_str(&crate::cdk::utils::hash::hex_bytes(&bytes));
    candid.push('\n');
    candid
}
