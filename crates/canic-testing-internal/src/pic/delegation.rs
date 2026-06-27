use candid::Principal;
use canic::{
    Error,
    dto::auth::{
        AuthRequestMetadata, DelegatedRoleGrant, DelegatedToken, DelegatedTokenGetRequest,
        DelegatedTokenPrepareRequest, DelegatedTokenPrepareResponse, DelegationAudience,
    },
    ids::CanisterRole,
    protocol,
};
use ic_testkit::pic::Pic;

/// Create one user shard through the reference `user_hub` path.
///
/// # Panics
///
/// Panics if the `create_account` transport or application call fails.
#[must_use]
pub fn create_user_shard(pic: &Pic, user_hub_pid: Principal, user_pid: Principal) -> Principal {
    let created: Result<Principal, Error> =
        pic.update_call_or_panic(user_hub_pid, "create_account", (user_pid,));
    created.expect("create_account application failed")
}

/// Issue one delegated token from the issuer's already-installed active proof.
///
/// # Panics
///
/// Panics if delegated-token prepare/get transport fails or either application
/// call returns an error.
#[must_use]
pub fn issue_delegated_token_from_active_proof(
    pic: &Pic,
    issuer_pid: Principal,
    subject: Principal,
    aud: DelegationAudience,
    grants: Vec<DelegatedRoleGrant>,
    token_ttl_ns: u64,
) -> DelegatedToken {
    issue_delegated_token_from_active_proof_with_request_nonce(
        pic,
        issuer_pid,
        subject,
        aud,
        grants,
        token_ttl_ns,
        0,
    )
}

/// Issue one delegated token using an explicit replay request nonce.
///
/// # Panics
///
/// Panics if delegated-token prepare/get transport fails or either application
/// call returns an error.
#[must_use]
pub fn issue_delegated_token_from_active_proof_with_request_nonce(
    pic: &Pic,
    issuer_pid: Principal,
    subject: Principal,
    aud: DelegationAudience,
    grants: Vec<DelegatedRoleGrant>,
    token_ttl_ns: u64,
    request_nonce: u64,
) -> DelegatedToken {
    let request = DelegatedTokenPrepareRequest {
        metadata: Some(issue_token_request_metadata(
            issuer_pid,
            subject,
            &aud,
            &grants,
            token_ttl_ns,
            request_nonce,
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

fn issue_token_request_metadata(
    issuer_pid: Principal,
    subject: Principal,
    aud: &DelegationAudience,
    grants: &[DelegatedRoleGrant],
    token_ttl_ns: u64,
    request_nonce: u64,
) -> AuthRequestMetadata {
    let mut request_id = [0u8; 32];
    mix_principal(&mut request_id, 0, issuer_pid);
    mix_principal(&mut request_id, 7, subject);
    mix_audience(&mut request_id, 13, aud);
    mix_u64(&mut request_id, 11, request_nonce);
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
