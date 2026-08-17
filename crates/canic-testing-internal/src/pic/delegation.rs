use candid::{CandidType, Deserialize, Principal};
use canic::{
    Error,
    dto::auth::{
        AuthRequestMetadata, DelegatedRoleGrant, DelegatedToken, DelegatedTokenGetRequest,
        DelegatedTokenPrepareRequest, DelegatedTokenPrepareResponse, DelegationAudience,
    },
    ids::CanisterRole,
    protocol,
};
use ic_testkit::pic::{CandidCallExt, PocketIc};

#[derive(CandidType)]
enum CanisterCommand {
    PrepareDelegatedToken(DelegatedTokenPrepareRequest),
}

#[derive(CandidType, Deserialize)]
enum CanisterCommandResponse {
    PrepareDelegatedToken(DelegatedTokenPrepareResponse),
}

#[derive(CandidType)]
enum CanisterStatusRequest {
    DelegatedToken(DelegatedTokenGetRequest),
}

#[derive(CandidType, Deserialize)]
enum CanisterStatusResponse {
    DelegatedToken(DelegatedToken),
}

/// Create one user shard through the reference `user_hub` path.
///
/// # Panics
///
/// Panics if the `create_account` transport or application call fails.
#[must_use]
pub fn create_user_shard(
    pic: &PocketIc,
    user_hub_pid: Principal,
    user_pid: Principal,
) -> Principal {
    let created: Result<Principal, Error> =
        pic.update_candid_or_panic(user_hub_pid, "create_account", (user_pid,));
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
    pic: &PocketIc,
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
    pic: &PocketIc,
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
    let prepared: Result<CanisterCommandResponse, Error> = pic.update_candid_as_or_panic(
        issuer_pid,
        subject,
        protocol::CANIC_COMMAND,
        (CanisterCommand::PrepareDelegatedToken(request),),
    );
    let CanisterCommandResponse::PrepareDelegatedToken(prepared) =
        prepared.expect("delegated-token command application failed");
    let issued: Result<CanisterStatusResponse, Error> = pic.query_candid_as_or_panic(
        issuer_pid,
        subject,
        protocol::CANIC_STATUS,
        (CanisterStatusRequest::DelegatedToken(
            DelegatedTokenGetRequest {
                claims_hash: prepared.claims_hash,
            },
        ),),
    );
    let CanisterStatusResponse::DelegatedToken(token) =
        issued.expect("delegated-token status application failed");
    token
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
        DelegationAudience::Fleet(fleet) => {
            request_id[offset % request_id.len()] ^= 1;
            for (index, byte) in fleet
                .canonical_network_id
                .as_bytes()
                .iter()
                .chain(fleet.fleet_id.as_bytes())
                .enumerate()
            {
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
