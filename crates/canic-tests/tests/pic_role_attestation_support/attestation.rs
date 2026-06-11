use super::*;

pub const NS_PER_SEC: u64 = 1_000_000_000;
pub const TEST_ROLE_ATTESTATION_TTL_NS: u64 = 60 * NS_PER_SEC;
pub const TEST_SHORT_ROLE_ATTESTATION_TTL_NS: u64 = NS_PER_SEC;

// Issue one self-attestation from the root test hook for the requested audience.
pub fn issue_self_attestation(
    pic: &Pic,
    root_id: Principal,
    ttl_ns: u64,
    audience: Principal,
) -> SignedRoleAttestation {
    issue_self_attestation_as(pic, root_id, root_id, ttl_ns, audience)
}

// Issue one self-attestation from the root test hook as an explicit caller.
pub fn issue_self_attestation_as(
    pic: &Pic,
    root_id: Principal,
    caller: Principal,
    ttl_ns: u64,
    audience: Principal,
) -> SignedRoleAttestation {
    let prepared: Result<RoleAttestationPrepareResponse, Error> = pic.update_call_as_or_panic(
        root_id,
        caller,
        "canic_prepare_role_attestation",
        (RoleAttestationRequest {
            subject: caller,
            role: CanisterRole::ROOT,
            subnet_id: None,
            audience,
            ttl_ns,
            epoch: 0,
            metadata: Some(RootRequestMetadata {
                request_id: attestation_request_id(caller, ttl_ns, audience),
                ttl_ns: TEST_ROLE_ATTESTATION_TTL_NS,
            }),
        },),
    );
    let prepared = prepared.expect("role attestation prepare failed");
    let issued: Result<SignedRoleAttestation, Error> = pic.query_call_as_or_panic(
        root_id,
        caller,
        "canic_get_role_attestation",
        (RoleAttestationGetRequest {
            payload_hash: prepared.payload_hash,
        },),
    );

    issued.expect("attestation issuance failed")
}

fn attestation_request_id(caller: Principal, ttl_ns: u64, audience: Principal) -> [u8; 32] {
    let mut out = [0u8; 32];
    out[..8].copy_from_slice(&ttl_ns.to_be_bytes());
    for (idx, byte) in caller.as_slice().iter().take(12).enumerate() {
        out[8 + idx] = *byte;
    }
    for (idx, byte) in audience.as_slice().iter().take(12).enumerate() {
        out[20 + idx] = *byte;
    }
    out
}

pub const fn capability_metadata(
    issued_at_ns: u64,
    request_id_seed: u8,
    nonce_seed: u8,
    ttl_ns: u64,
) -> CapabilityRequestMetadata {
    CapabilityRequestMetadata {
        request_id: [request_id_seed; 16],
        nonce: [nonce_seed; 16],
        issued_at_ns,
        ttl_ns,
    }
}
