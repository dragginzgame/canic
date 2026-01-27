// Category C - Artifact / deployment test (embedded config).
// This test relies on embedded production config by design.

mod root;

use canic::{
    Error,
    api::{auth::DelegationApi, ic::network::NetworkApi},
    cdk::{types::Principal, utils::time::now_secs},
    dto::{
        auth::{
            DelegatedToken, DelegatedTokenClaims, DelegationCert, DelegationProvisionRequest,
            DelegationProvisionResponse, DelegationProvisionStatus,
        },
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

    let test_pid = setup
        .subnet_directory
        .get(&canister::TEST)
        .copied()
        .expect("test canister must exist in subnet directory");

    log_step(&format!(
        "user_hub={user_hub_pid} test_canister={test_pid} root={}",
        setup.root_id
    ));

    let tenant = p(7);
    let shard_pid = create_user_shard(&setup, user_hub_pid, tenant);

    let audiences = vec!["login".to_string()];
    let scopes = vec!["read".to_string()];
    let cert = build_cert(shard_pid, audiences, scopes, 3600_u64);

    log_step(&format!(
        "provision signer={shard_pid} verifiers=[{test_pid}]"
    ));
    let response =
        provision_delegation(&setup, user_hub_pid, cert, vec![shard_pid], vec![test_pid]);

    assert_provisioned(&response);
    log_step(&format!(
        "provisioned proof signer={}",
        response.proof.cert.signer_pid
    ));

    DelegationApi::verify_delegation_proof(&response.proof, setup.root_id)
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

    let test_pid = setup
        .subnet_directory
        .get(&canister::TEST)
        .copied()
        .expect("test canister must exist in subnet directory");

    log_step(&format!(
        "user_hub={user_hub_pid} test_canister={test_pid} root={}",
        setup.root_id
    ));

    let tenant = p(7);
    let audiences = vec!["login".to_string()];
    let scopes = vec!["read".to_string()];

    let shard_pid = create_user_shard(&setup, user_hub_pid, tenant);
    let cert = build_cert(shard_pid, audiences.clone(), scopes.clone(), 3600_u64);

    log_step(&format!(
        "provision signer={shard_pid} verifiers=[{test_pid}]"
    ));
    let response =
        provision_delegation(&setup, user_hub_pid, cert, vec![shard_pid], vec![test_pid]);

    assert_provisioned(&response);

    let now = now_secs();
    let claims = DelegatedTokenClaims {
        sub: p(9),
        aud: audiences[0].clone(),
        scopes,
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

    let verify: Result<Result<(), Error>, Error> =
        setup
            .pic
            .update_call(test_pid, "test_verify_delegated_token", (token.clone(),));

    verify
        .expect("test_verify_delegated_token transport failed")
        .expect("test_verify_delegated_token application failed");

    assert_eq!(token.proof, response.proof);
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
    let audiences = vec!["login".to_string()];
    let scopes = vec!["read".to_string()];

    let shard_pid = create_user_shard(&setup, user_hub_pid, tenant);
    let cert = build_cert(shard_pid, audiences.clone(), scopes.clone(), 3600_u64);

    log_step(&format!(
        "provision signer={shard_pid} verifiers=[{}]",
        setup.root_id
    ));
    let provisioned = provision_delegation(
        &setup,
        user_hub_pid,
        cert,
        vec![shard_pid],
        vec![setup.root_id],
    );

    assert_provisioned(&provisioned);

    let now = now_secs();
    let claims = DelegatedTokenClaims {
        sub: p(9),
        aud: audiences[0].clone(),
        scopes,
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
        "calling canic_response_authenticated via shard={shard_pid}"
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
fn delegated_token_requires_proof() {
    if !should_run_certified("delegated_token_requires_proof") {
        return;
    }

    log_step("delegated_token_requires_proof: setup root");
    let setup = setup_root();

    let user_hub_pid = setup
        .subnet_directory
        .get(&canister::USER_HUB)
        .copied()
        .expect("user_hub must exist in subnet directory");

    let tenant = p(7);
    let audiences = ["login".to_string()];
    let scopes = ["read".to_string()];

    let shard_pid = create_user_shard(&setup, user_hub_pid, tenant);

    let now = now_secs();
    let claims = DelegatedTokenClaims {
        sub: p(9),
        aud: audiences[0].clone(),
        scopes: scopes.to_vec(),
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
        .expect_err("user_shard_mint_token should fail without proof");
    assert_eq!(err.code, ErrorCode::NotFound);
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
            .update_call(user_hub_pid, "create_user_shard", (tenant,));

    let pid = created
        .expect("create_user_shard transport failed")
        .expect("create_user_shard application failed");
    log_step(&format!("user_shard created pid={pid}"));
    pid
}

fn build_cert(
    signer_pid: Principal,
    audiences: Vec<String>,
    scopes: Vec<String>,
    ttl_secs: u64,
) -> DelegationCert {
    let issued_at = now_secs();
    let expires_at = issued_at.saturating_add(ttl_secs);

    DelegationCert {
        v: 1,
        signer_pid,
        audiences,
        scopes,
        issued_at,
        expires_at,
    }
}

fn provision_delegation(
    setup: &RootSetup,
    user_hub_pid: Principal,
    cert: DelegationCert,
    signer_targets: Vec<Principal>,
    verifier_targets: Vec<Principal>,
) -> DelegationProvisionResponse {
    log_step(&format!(
        "provision request signer_targets={signer_targets:?} verifier_targets={verifier_targets:?}"
    ));
    let request = DelegationProvisionRequest {
        cert,
        signer_targets,
        verifier_targets,
    };

    let provisioned: Result<Result<DelegationProvisionResponse, Error>, Error> = setup
        .pic
        .update_call(user_hub_pid, "provision_user_shard", (request,));

    let response = provisioned
        .expect("provision_user_shard transport failed")
        .expect("provision_user_shard application failed");
    log_step(&format!(
        "provision response entries={}",
        response.results.len()
    ));
    response
}

fn assert_provisioned(response: &DelegationProvisionResponse) {
    assert!(
        response
            .results
            .iter()
            .all(|entry| entry.status == DelegationProvisionStatus::Ok),
        "provisioning failed: {:?}",
        response.results
    );
}
