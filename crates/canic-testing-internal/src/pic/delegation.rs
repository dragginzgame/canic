use candid::Principal;
use canic::{
    Error,
    dto::auth::{
        DelegatedRoleGrant, DelegatedToken, DelegatedTokenMintRequest, DelegationAudience,
        DelegationProof, DelegationProofIssueRequest,
    },
    dto::rpc::RootRequestMetadata,
    ids::{CanisterRole, cap},
    protocol,
};
use ic_testkit::pic::Pic;

const USER_SHARD_LOCAL_PUBLIC_KEY_TEST: &str = "user_shard_local_public_key_test";

// Create one user shard through the reference `user_hub` path.
#[must_use]
pub fn create_user_shard(pic: &Pic, user_hub_pid: Principal, user_pid: Principal) -> Principal {
    let created: Result<Principal, Error> =
        pic.update_call_or_panic(user_hub_pid, "create_account", (user_pid,));
    created.expect("create_account application failed")
}

// Mint one delegated token from a prepared shard with caller-selected claims.
#[must_use]
pub fn issue_delegated_token(
    pic: &Pic,
    shard_pid: Principal,
    subject: Principal,
    aud: DelegationAudience,
    grants: Vec<DelegatedRoleGrant>,
    token_ttl_secs: u64,
    cert_ttl_secs: u64,
) -> DelegatedToken {
    let request = DelegatedTokenMintRequest {
        metadata: Some(mint_token_request_metadata(
            shard_pid,
            subject,
            &aud,
            &grants,
            token_ttl_secs,
            cert_ttl_secs,
        )),
        subject,
        aud,
        grants,
        token_ttl_secs,
        cert_ttl_secs,
        nonce: [0; 16],
    };
    let issued: Result<DelegatedToken, Error> =
        pic.update_call_or_panic(shard_pid, "user_shard_issue_token", (request,));
    issued.expect("user_shard_issue_token application failed")
}

// Request one canonical root-issued delegation for a shard/verifier pair.
#[must_use]
pub fn request_root_delegation_provision(
    pic: &Pic,
    root_id: Principal,
    shard_pid: Principal,
    verifier_role: CanisterRole,
) -> DelegationProof {
    let _shard_public_key_sec1: Result<Vec<u8>, Error> =
        pic.update_call_or_panic(shard_pid, USER_SHARD_LOCAL_PUBLIC_KEY_TEST, ());
    let request = DelegationProofIssueRequest {
        metadata: Some(root_delegation_request_metadata(shard_pid, &verifier_role)),
        shard_pid,
        aud: DelegationAudience::Project("test".to_string()),
        grants: vec![role_grant(verifier_role, vec![cap::VERIFY.to_string()])],
        cert_ttl_secs: 60,
    };
    let response: Result<DelegationProof, Error> = pic.update_call_as_or_panic(
        root_id,
        shard_pid,
        protocol::CANIC_REQUEST_DELEGATION,
        (request,),
    );
    response.expect("canic_request_delegation application failed")
}

fn root_delegation_request_metadata(
    shard_pid: Principal,
    verifier_role: &CanisterRole,
) -> RootRequestMetadata {
    let mut request_id = [0u8; 32];
    for (index, byte) in shard_pid.as_slice().iter().enumerate() {
        request_id[index % request_id.len()] ^= *byte;
    }
    for (index, byte) in verifier_role.as_str().as_bytes().iter().enumerate() {
        request_id[(index + 13) % request_id.len()] ^= *byte;
    }
    RootRequestMetadata {
        request_id,
        ttl_seconds: 60,
    }
}

fn mint_token_request_metadata(
    shard_pid: Principal,
    subject: Principal,
    aud: &DelegationAudience,
    grants: &[DelegatedRoleGrant],
    token_ttl_secs: u64,
    cert_ttl_secs: u64,
) -> RootRequestMetadata {
    let mut request_id = [0u8; 32];
    mix_principal(&mut request_id, 0, shard_pid);
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
    mix_u64(&mut request_id, 3, token_ttl_secs);
    mix_u64(&mut request_id, 11, cert_ttl_secs);
    RootRequestMetadata {
        request_id,
        ttl_seconds: 60,
    }
}

fn mix_audience(request_id: &mut [u8; 32], offset: usize, aud: &DelegationAudience) {
    match aud {
        DelegationAudience::Canic => request_id[offset % request_id.len()] ^= 1,
        DelegationAudience::Project(project) => {
            for (index, byte) in project.as_bytes().iter().enumerate() {
                request_id[(index + offset) % request_id.len()] ^= *byte;
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
