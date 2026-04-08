use super::*;

pub fn encode_role_attestation_capability_proof(proof: RoleAttestationProof) -> CapabilityProof {
    proof
        .try_into()
        .expect("role attestation proof should encode")
}

pub fn encode_delegated_grant_capability_proof(proof: DelegatedGrantProof) -> CapabilityProof {
    proof
        .try_into()
        .expect("delegated grant proof should encode")
}

// Issue one self-attestation from the root test hook for the requested audience.
pub fn issue_self_attestation(
    pic: &Pic,
    root_id: Principal,
    ttl_secs: u64,
    audience: Option<Principal>,
) -> SignedRoleAttestation {
    issue_self_attestation_as(pic, root_id, root_id, ttl_secs, audience)
}

// Issue one self-attestation from the root test hook as an explicit caller.
pub fn issue_self_attestation_as(
    pic: &Pic,
    root_id: Principal,
    caller: Principal,
    ttl_secs: u64,
    audience: Option<Principal>,
) -> SignedRoleAttestation {
    let issued: Result<SignedRoleAttestation, Error> = update_call_as(
        pic,
        root_id,
        caller,
        "root_issue_self_attestation_test",
        (ttl_secs, audience, 0u64),
    );

    issued.expect("attestation issuance failed")
}

// Build a cycles capability envelope backed by a role-attestation proof.
pub fn cycles_role_attestation_envelope(
    root_id: Principal,
    request: Request,
    attestation: SignedRoleAttestation,
    issued_at: u64,
    request_id_seed: u8,
    nonce_seed: u8,
) -> RootCapabilityEnvelopeV1 {
    RootCapabilityEnvelopeV1 {
        service: CapabilityService::Root,
        capability_version: CAPABILITY_VERSION_V1,
        capability: request.clone(),
        proof: encode_role_attestation_capability_proof(RoleAttestationProof {
            proof_version: PROOF_VERSION_V1,
            capability_hash: root_capability_hash(root_id, &request),
            attestation,
        }),
        metadata: capability_metadata(issued_at, request_id_seed, nonce_seed, 60),
    }
}

pub fn root_capability_hash(target_canister: Principal, capability: &Request) -> [u8; 32] {
    RpcApi::root_capability_hash(target_canister, CAPABILITY_VERSION_V1, capability)
        .expect("compute root capability hash")
}

pub const fn capability_metadata(
    issued_at: u64,
    request_id_seed: u8,
    nonce_seed: u8,
    ttl_seconds: u32,
) -> CapabilityRequestMetadata {
    CapabilityRequestMetadata {
        request_id: [request_id_seed; 16],
        nonce: [nonce_seed; 16],
        issued_at,
        ttl_seconds,
    }
}
