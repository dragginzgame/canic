//! Expected native issuer fixture contracts for installed Auth qualification.

use ic_testkit::pic::WireCallSpec;

/// Installed `issuer_application_subject` contract.
pub const SUBJECT: WireCallSpec = spec("issuer_application_subject", 0x5445_5354_0000_0800);
/// Installed `issuer_fleet_admission_probe` contract.
pub const ADMISSION: WireCallSpec = spec("issuer_fleet_admission_probe", 0x5445_5354_0000_0801);
/// Installed `issuer_application_denial_perf` contract.
pub const DENIAL: WireCallSpec = spec("issuer_application_denial_perf", 0x5445_5354_0000_0802);
/// Installed `issuer_verify_role_attestation` contract.
pub const VERIFY: WireCallSpec = spec("issuer_verify_role_attestation", 0x5445_5354_0000_0803);
/// Installed `issuer_require_attested_local_subnet` contract.
pub const ATTESTED: WireCallSpec = spec(
    "issuer_require_attested_local_subnet",
    0x5445_5354_0000_0804,
);
/// Installed `issuer_guard_is_root` contract.
pub const ROOT_GUARD: WireCallSpec = spec("issuer_guard_is_root", 0x5445_5354_0000_0805);

const fn spec(method: &'static str, method_id: u64) -> WireCallSpec {
    WireCallSpec {
        method,
        method_id,
        request_limits: ic_wire::Limits {
            bytes: 65_536,
            allocation: 2_097_152,
            nodes: 65_536,
            depth: 32,
        },
        response_limits: ic_wire::Limits {
            bytes: 65_536,
            allocation: 262_144,
            nodes: 4096,
            depth: 32,
        },
    }
}
/// Verify the exact signed attestation at an explicit minimum accepted epoch.
#[derive(serde::Deserialize, serde::Serialize)]
pub struct VerifyRequest {
    pub attestation: canic::dto::role_attestation::SignedRoleAttestation,
    pub min_accepted_epoch: u64,
}
