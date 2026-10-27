//! Native issuer fixture contracts; shared Auth owns authorization and proof verification.

pub const SUBJECT: u64 = 0x5445_5354_0000_0800;
pub const ADMISSION: u64 = 0x5445_5354_0000_0801;
pub const DENIAL: u64 = 0x5445_5354_0000_0802;
pub const VERIFY: u64 = 0x5445_5354_0000_0803;
pub const ATTESTED: u64 = 0x5445_5354_0000_0804;
pub const ROOT_GUARD: u64 = 0x5445_5354_0000_0805;

pub const REQUEST_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 65_536,
    allocation: 2_097_152,
    nodes: 65_536,
    depth: 32,
};
pub const RESPONSE_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 65_536,
    allocation: 262_144,
    nodes: 4096,
    depth: 32,
};
/// Verify the exact signed attestation at an explicit minimum accepted epoch.
#[derive(serde::Deserialize, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct VerifyRequest {
    pub attestation: canic::dto::role_attestation::SignedRoleAttestation,
    pub min_accepted_epoch: u64,
}
