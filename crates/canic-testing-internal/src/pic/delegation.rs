use candid::Principal;
use canic::{
    Error,
    dto::auth::{
        DelegatedToken, DelegatedTokenMintRequest, DelegationAudience, DelegationProof,
        DelegationProofIssueRequest,
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
    token_ttl_secs: u64,
    cert_ttl_secs: u64,
) -> DelegatedToken {
    let request = DelegatedTokenMintRequest {
        subject,
        aud,
        scopes,
        token_ttl_secs,
        cert_ttl_secs,
        nonce: [0; 16],
    };
    let issued: Result<DelegatedToken, Error> = pic
        .update_call(shard_pid, "user_shard_issue_token", (request,))
        .expect("user_shard_issue_token transport failed");
    issued.expect("user_shard_issue_token application failed")
}

// Request one canonical root-issued delegation for a shard/verifier pair.
#[must_use]
pub fn request_root_delegation_provision(
    pic: &Pic,
    root_id: Principal,
    shard_pid: Principal,
    verifier_pid: Principal,
) -> DelegationProof {
    let _shard_public_key_sec1: Result<Vec<u8>, Error> = pic
        .update_call(shard_pid, USER_SHARD_LOCAL_PUBLIC_KEY_TEST, ())
        .expect("user_shard_local_public_key_test transport failed");
    let request = DelegationProofIssueRequest {
        shard_pid,
        scopes: vec![cap::VERIFY.to_string()],
        aud: DelegationAudience::Principals(vec![verifier_pid]),
        cert_ttl_secs: 60,
    };
    let response: Result<Result<DelegationProof, Error>, Error> = pic.update_call_as(
        root_id,
        shard_pid,
        protocol::CANIC_REQUEST_DELEGATION,
        (request,),
    );
    response
        .expect("canic_request_delegation transport failed")
        .expect("canic_request_delegation application failed")
}
