use candid::Principal;
use canic::{
    Error,
    dto::auth::{
        AuthRequestMetadata, DelegatedRoleGrant, DelegatedToken, DelegatedTokenGetRequest,
        DelegatedTokenPrepareRequest, DelegatedTokenPrepareResponse, DelegationAudience,
        DelegationProof, DelegationProofGetRequest, DelegationProofIssueRequest,
        DelegationProofPrepareResponse, InstallActiveDelegationProofRequest,
        InstallActiveDelegationProofResponse,
    },
    ids::{CanisterRole, cap},
    protocol,
};
use ic_testkit::pic::Pic;

const TOKEN_CERT_EXPIRY_MARGIN_NS: u64 = 1_000_000_000;

// Create one user shard through the reference `user_hub` path.
#[must_use]
pub fn create_user_shard(pic: &Pic, user_hub_pid: Principal, user_pid: Principal) -> Principal {
    let created: Result<Principal, Error> =
        pic.update_call_or_panic(user_hub_pid, "create_account", (user_pid,));
    created.expect("create_account application failed")
}

// Issue one delegated token from a prepared shard with caller-selected claims.
#[must_use]
pub fn issue_delegated_token(
    pic: &Pic,
    issuer_pid: Principal,
    proof: DelegationProof,
    subject: Principal,
    aud: DelegationAudience,
    grants: Vec<DelegatedRoleGrant>,
    token_ttl_ns: u64,
) -> DelegatedToken {
    let installed: Result<InstallActiveDelegationProofResponse, Error> = pic.update_call_or_panic(
        issuer_pid,
        protocol::CANIC_INSTALL_ACTIVE_DELEGATION_PROOF,
        (InstallActiveDelegationProofRequest { proof },),
    );
    installed.expect("canic_install_active_delegation_proof application failed");

    let request = DelegatedTokenPrepareRequest {
        metadata: Some(issue_token_request_metadata(
            issuer_pid,
            subject,
            &aud,
            &grants,
            token_ttl_ns,
        )),
        subject,
        aud,
        grants,
        ttl_ns: token_ttl_ns,
        ext: None,
    };
    let prepared: Result<DelegatedTokenPrepareResponse, Error> = pic.update_call_as_or_panic(
        issuer_pid,
        subject,
        protocol::CANIC_PREPARE_DELEGATED_TOKEN,
        (request,),
    );
    let prepared = prepared.expect("canic_prepare_delegated_token application failed");
    let issued: Result<DelegatedToken, Error> = pic.query_call_as_or_panic(
        issuer_pid,
        subject,
        protocol::CANIC_GET_DELEGATED_TOKEN,
        (DelegatedTokenGetRequest {
            claims_hash: prepared.claims_hash,
        },),
    );
    issued.expect("canic_get_delegated_token application failed")
}

// Obtain one canonical root-issued proof through the prepare/update + get/query flow.
#[must_use]
pub fn obtain_root_delegation_proof(
    pic: &Pic,
    root_id: Principal,
    issuer_pid: Principal,
    verifier_role: CanisterRole,
) -> DelegationProof {
    let request = DelegationProofIssueRequest {
        metadata: Some(root_delegation_request_metadata(issuer_pid, &verifier_role)),
        issuer_pid,
        aud: DelegationAudience::Project("test".to_string()),
        grants: vec![role_grant(verifier_role, vec![cap::VERIFY.to_string()])],
        cert_ttl_ns: 60_000_000_000,
    };
    let prepared: Result<DelegationProofPrepareResponse, Error> = pic.update_call_as_or_panic(
        root_id,
        issuer_pid,
        protocol::CANIC_PREPARE_DELEGATION_PROOF,
        (request,),
    );
    let prepared = prepared.expect("canic_prepare_delegation_proof application failed");
    let response: Result<DelegationProof, Error> = pic.query_call_as_or_panic(
        root_id,
        issuer_pid,
        protocol::CANIC_GET_DELEGATION_PROOF,
        (DelegationProofGetRequest {
            cert_hash: prepared.cert_hash,
        },),
    );
    response.expect("canic_get_delegation_proof application failed")
}

/// Pick a reusable token TTL that stays inside the root-certified proof window.
#[must_use]
pub fn token_ttl_within_proof(pic: &Pic, proof: &DelegationProof) -> u64 {
    let remaining_cert_ttl_ns = proof
        .cert
        .expires_at_ns
        .saturating_sub(pic.current_time_nanos());
    let bounded_ttl_ns = remaining_cert_ttl_ns
        .saturating_sub(TOKEN_CERT_EXPIRY_MARGIN_NS)
        .min(proof.cert.max_token_ttl_ns);

    assert!(
        bounded_ttl_ns > 0,
        "delegation proof must have enough remaining lifetime for token issuance"
    );

    bounded_ttl_ns
}

fn root_delegation_request_metadata(
    issuer_pid: Principal,
    verifier_role: &CanisterRole,
) -> AuthRequestMetadata {
    let mut request_id = [0u8; 32];
    for (index, byte) in issuer_pid.as_slice().iter().enumerate() {
        request_id[index % request_id.len()] ^= *byte;
    }
    for (index, byte) in verifier_role.as_str().as_bytes().iter().enumerate() {
        request_id[(index + 13) % request_id.len()] ^= *byte;
    }
    AuthRequestMetadata {
        request_id,
        ttl_ns: 60_000_000_000,
    }
}

fn issue_token_request_metadata(
    issuer_pid: Principal,
    subject: Principal,
    aud: &DelegationAudience,
    grants: &[DelegatedRoleGrant],
    token_ttl_ns: u64,
) -> AuthRequestMetadata {
    let mut request_id = [0u8; 32];
    mix_principal(&mut request_id, 0, issuer_pid);
    mix_principal(&mut request_id, 7, subject);
    mix_audience(&mut request_id, 13, aud);
    for (grant_index, grant) in grants.iter().enumerate() {
        for (byte_index, byte) in grant.target.as_str().as_bytes().iter().enumerate() {
            request_id[(grant_index + byte_index + 19) % request_id.len()] ^= *byte;
        }
        for (scope_index, scope) in grant.scopes.iter().enumerate() {
            for (byte_index, byte) in scope.as_bytes().iter().enumerate() {
                request_id[(grant_index + scope_index + byte_index + 23) % request_id.len()] ^=
                    *byte;
            }
        }
    }
    mix_u64(&mut request_id, 3, token_ttl_ns);
    AuthRequestMetadata {
        request_id,
        ttl_ns: 60_000_000_000,
    }
}

fn mix_audience(request_id: &mut [u8; 32], offset: usize, aud: &DelegationAudience) {
    match aud {
        DelegationAudience::Canister(canister) => {
            request_id[offset % request_id.len()] ^= 1;
            mix_principal(request_id, offset + 1, *canister);
        }
        DelegationAudience::CanicSubnet(subnet) => {
            request_id[offset % request_id.len()] ^= 2;
            mix_principal(request_id, offset + 1, *subnet);
        }
        DelegationAudience::Project(project) => {
            request_id[offset % request_id.len()] ^= 3;
            for (index, byte) in project.as_bytes().iter().enumerate() {
                request_id[(index + offset + 1) % request_id.len()] ^= *byte;
            }
        }
    }
}

fn mix_principal(request_id: &mut [u8; 32], offset: usize, principal: Principal) {
    for (index, byte) in principal.as_slice().iter().enumerate() {
        request_id[(index + offset) % request_id.len()] ^= *byte;
    }
}

fn mix_u64(request_id: &mut [u8; 32], offset: usize, value: u64) {
    for (index, byte) in value.to_be_bytes().iter().enumerate() {
        request_id[(index + offset) % request_id.len()] ^= *byte;
    }
}

#[must_use]
pub const fn role_grant(target: CanisterRole, scopes: Vec<String>) -> DelegatedRoleGrant {
    DelegatedRoleGrant { target, scopes }
}
