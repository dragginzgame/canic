use candid::Principal;
use canic::{
    Error,
    dto::auth::{
        DelegatedToken, DelegatedTokenClaims, DelegationAudience, DelegationProvisionResponse,
        DelegationRequest,
    },
    ids::cap,
    protocol,
};
use canic_testkit::pic::Pic;

const USER_SHARD_LOCAL_PUBLIC_KEY_TEST: &str = "user_shard_local_public_key_test";

// Create one user shard through the reference `user_hub` path.
#[must_use]
pub fn create_user_shard(pic: &Pic, user_hub_pid: Principal, user_pid: Principal) -> Principal {
    let created: Result<Principal, Error> = pic
        .update_call(user_hub_pid, "create_account", (user_pid,))
        .expect("create_account transport failed");
    created.expect("create_account application failed")
}

// Mint one delegated token from a prepared shard with caller-selected claims.
#[must_use]
pub fn issue_delegated_token(
    pic: &Pic,
    shard_pid: Principal,
    subject: Principal,
    aud: DelegationAudience,
    scopes: Vec<String>,
    issued_at: u64,
    expires_at: u64,
) -> DelegatedToken {
    let claims = DelegatedTokenClaims {
        sub: subject,
        shard_pid,
        aud,
        scopes,
        iat: issued_at,
        exp: expires_at,
        ext: None,
    };
    let issued: Result<DelegatedToken, Error> = pic
        .update_call(shard_pid, "user_shard_issue_token", (claims,))
        .expect("user_shard_issue_token transport failed");
    issued.expect("user_shard_issue_token application failed")
}

// Request one canonical root-issued delegation for a shard/verifier pair.
#[must_use]
pub fn request_root_delegation_provision(
    pic: &Pic,
    root_id: Principal,
    shard_pid: Principal,
    _verifier_pid: Principal,
) -> DelegationProvisionResponse {
    let shard_public_key_sec1: Result<Vec<u8>, Error> = pic
        .update_call(shard_pid, USER_SHARD_LOCAL_PUBLIC_KEY_TEST, ())
        .expect("user_shard_local_public_key_test transport failed");
    let request = DelegationRequest {
        shard_pid,
        scopes: vec![cap::VERIFY.to_string()],
        aud: DelegationAudience::Any,
        ttl_secs: 60,
        shard_public_key_sec1: shard_public_key_sec1
            .expect("user_shard_local_public_key_test application failed"),
        metadata: None,
    };
    let response: Result<Result<DelegationProvisionResponse, Error>, Error> = pic.update_call_as(
        root_id,
        shard_pid,
        protocol::CANIC_REQUEST_DELEGATION,
        (request,),
    );
    response
        .expect("canic_request_delegation transport failed")
        .expect("canic_request_delegation application failed")
}
