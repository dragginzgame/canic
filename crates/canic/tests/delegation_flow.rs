// Category C - Artifact / deployment test (embedded config).
// This test relies on embedded production config by design.

mod root;

use canic::{
    Error,
    api::{auth::DelegationApi, ic::network::NetworkApi},
    cdk::{types::Principal, utils::time::now_secs},
    dto::{
        auth::{DelegatedToken, DelegatedTokenClaims},
        error::ErrorCode,
        rpc::{AuthenticatedRequest, CyclesRequest, Request, Response},
    },
    ids::BuildNetwork,
};
use canic_internal::canister;
use root::harness::{RootSetup, setup_root};

const fn p(id: u8) -> Principal {
    Principal::from_slice(&[id; 29])
}

#[test]
fn delegation_provisioning_flow() {
    if !should_run_certified("delegation_provisioning_flow") {
        return;
    }

    log_step("delegation_provisioning_flow: setup root");
    let setup = setup_root();

    let user_hub_pid = setup
        .subnet_directory
        .get(&canister::USER_HUB)
        .copied()
        .expect("user_hub must exist in subnet directory");

    log_step(&format!("user_hub={user_hub_pid} root={}", setup.root_id));

    let tenant = p(7);
    let shard_pid = create_user_shard(&setup, user_hub_pid, tenant);

    let now = now_secs();
    let claims = DelegatedTokenClaims {
        sub: p(9),
        aud: "login".to_string(),
        scopes: vec!["read".to_string()],
        iat: now,
        exp: now + 60,
        ext: None,
        nonce: None,
    };

    let minted: Result<Result<DelegatedToken, Error>, Error> =
        setup
            .pic
            .update_call(shard_pid, "user_shard_mint_token", (claims,));

    let token = minted
        .expect("user_shard_mint_token transport failed")
        .expect("user_shard_mint_token application failed");

    DelegationApi::verify_delegation_proof(&token.proof, setup.root_id)
        .expect("delegation proof must verify");
}

#[test]
fn delegated_token_flow() {
    if !should_run_certified("delegated_token_flow") {
        return;
    }

    log_step("delegated_token_flow: setup root");
    let setup = setup_root();

    let user_hub_pid = setup
        .subnet_directory
        .get(&canister::USER_HUB)
        .copied()
        .expect("user_hub must exist in subnet directory");

    log_step(&format!("user_hub={user_hub_pid} root={}", setup.root_id));

    let tenant = p(7);
    let shard_pid = create_user_shard(&setup, user_hub_pid, tenant);

    let now = now_secs();
    let claims = DelegatedTokenClaims {
        sub: p(9),
        aud: "login".to_string(),
        scopes: vec!["read".to_string()],
        iat: now,
        exp: now + 60,
        ext: None,
        nonce: None,
    };

    let minted: Result<Result<DelegatedToken, Error>, Error> =
        setup
            .pic
            .update_call(shard_pid, "user_shard_mint_token", (claims,));

    let token = minted
        .expect("user_shard_mint_token transport failed")
        .expect("user_shard_mint_token application failed");
    log_step(&format!(
        "minted token proof signer={}",
        token.proof.cert.signer_pid
    ));

    let verify: Result<Result<Response, Error>, Error> = setup.pic.update_call_as(
        setup.root_id,
        shard_pid,
        "canic_response_authenticated",
        (AuthenticatedRequest {
            request: Request::Cycles(CyclesRequest { cycles: 1 }),
            delegated_token: token,
        },),
    );

    let response = verify
        .expect("canic_response_authenticated transport failed")
        .expect("canic_response_authenticated application failed");

    match response {
        Response::Cycles(cycles) => {
            assert_eq!(cycles.cycles_transferred, 1);
        }
        other => panic!("unexpected response: {other:?}"),
    }
}

#[test]
fn authenticated_rpc_flow() {
    if !should_run_certified("authenticated_rpc_flow") {
        return;
    }

    log_step("authenticated_rpc_flow: setup root");
    let setup = setup_root();

    let user_hub_pid = setup
        .subnet_directory
        .get(&canister::USER_HUB)
        .copied()
        .expect("user_hub must exist in subnet directory");

    let tenant = p(7);
    let shard_pid = create_user_shard(&setup, user_hub_pid, tenant);

    let now = now_secs();
    let claims = DelegatedTokenClaims {
        sub: p(9),
        aud: "login".to_string(),
        scopes: vec!["read".to_string()],
        iat: now,
        exp: now + 60,
        ext: None,
        nonce: None,
    };

    let minted: Result<Result<DelegatedToken, Error>, Error> =
        setup
            .pic
            .update_call(shard_pid, "user_shard_mint_token", (claims,));

    let token = minted
        .expect("user_shard_mint_token transport failed")
        .expect("user_shard_mint_token application failed");

    log_step(&format!(
        "calling canic_response_authenticated via shard={}",
        shard_pid
    ));
    let request = AuthenticatedRequest {
        request: Request::Cycles(CyclesRequest { cycles: 1 }),
        delegated_token: token,
    };

    let response: Result<Result<Response, Error>, Error> = setup.pic.update_call_as(
        setup.root_id,
        shard_pid,
        "canic_response_authenticated",
        (request,),
    );

    let response = response
        .expect("canic_response_authenticated transport failed")
        .expect("canic_response_authenticated application failed");

    match response {
        Response::Cycles(cycles) => {
            assert_eq!(cycles.cycles_transferred, 1);
        }
        other => panic!("unexpected response: {other:?}"),
    }
}

#[test]
fn delegated_token_request_rejected_on_invalid_claims() {
    if !should_run_certified("delegated_token_request_rejected_on_invalid_claims") {
        return;
    }

    log_step("delegated_token_request_rejected_on_invalid_claims: setup root");
    let setup = setup_root();

    let user_hub_pid = setup
        .subnet_directory
        .get(&canister::USER_HUB)
        .copied()
        .expect("user_hub must exist in subnet directory");
    let shard_pid = create_user_shard(&setup, user_hub_pid, p(7));

    let now = now_secs();
    let claims = DelegatedTokenClaims {
        sub: p(9),
        aud: String::new(),
        scopes: Vec::new(),
        iat: now,
        exp: now + 60,
        ext: None,
        nonce: None,
    };

    let minted: Result<Result<DelegatedToken, Error>, Error> =
        setup
            .pic
            .update_call(shard_pid, "user_shard_mint_token", (claims,));

    let err = minted
        .expect("user_shard_mint_token transport failed")
        .expect_err("user_shard_mint_token should fail on invalid claims");
    assert_eq!(err.code, ErrorCode::InvalidInput);
}

fn log_step(step: &str) {
    canic::cdk::println!("[delegation_flow] {step}");
}

fn should_run_certified(test_name: &str) -> bool {
    if NetworkApi::build_network() == Some(BuildNetwork::Ic) {
        true
    } else {
        log_step(&format!("{test_name}: skipped (non-ic build)"));
        false
    }
}

fn create_user_shard(setup: &RootSetup, user_hub_pid: Principal, tenant: Principal) -> Principal {
    log_step(&format!(
        "create_user_shard tenant={tenant} via hub={user_hub_pid}"
    ));
    let created: Result<Result<Principal, Error>, Error> =
        setup
            .pic
            .update_call(user_hub_pid, "create_account", (tenant,));

    let pid = created
        .expect("create_account transport failed")
        .expect("create_account application failed");
    log_step(&format!("user_shard created pid={pid}"));
    pid
}
