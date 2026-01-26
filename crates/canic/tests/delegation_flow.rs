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

    let tenant = p(7);
    let shard_pid = create_user_shard(&setup, user_hub_pid, tenant);

    let audiences = vec!["login".to_string()];
    let scopes = vec!["read".to_string()];
    let cert = build_cert(shard_pid, audiences, scopes, 3600_u64);

    let response =
        provision_delegation(&setup, user_hub_pid, cert, vec![shard_pid], vec![test_pid]);

    assert_provisioned(&response);

    DelegationApi::verify_delegation_proof(&response.proof, setup.root_id)
        .expect("delegation proof must verify");
}

#[test]
fn delegated_token_flow() {
    if !should_run_certified("delegated_token_flow") {
        return;
    }

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

    let tenant = p(7);
    let audiences = vec!["login".to_string()];
    let scopes = vec!["read".to_string()];

    let shard_pid = create_user_shard(&setup, user_hub_pid, tenant);
    let cert = build_cert(shard_pid, audiences.clone(), scopes.clone(), 3600_u64);

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
fn delegated_token_requires_proof() {
    if !should_run_certified("delegated_token_requires_proof") {
        return;
    }

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

fn should_run_certified(test_name: &str) -> bool {
    if NetworkApi::build_network() == Some(BuildNetwork::Ic) {
        true
    } else {
        eprintln!("{test_name}: skipped (non-ic build)");
        false
    }
}

fn create_user_shard(setup: &RootSetup, user_hub_pid: Principal, tenant: Principal) -> Principal {
    let created: Result<Result<Principal, Error>, Error> =
        setup
            .pic
            .update_call(user_hub_pid, "create_user_shard", (tenant,));

    created
        .expect("create_user_shard transport failed")
        .expect("create_user_shard application failed")
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
    let request = DelegationProvisionRequest {
        cert,
        signer_targets,
        verifier_targets,
    };

    let provisioned: Result<Result<DelegationProvisionResponse, Error>, Error> = setup
        .pic
        .update_call(user_hub_pid, "provision_user_shard", (request,));

    provisioned
        .expect("provision_user_shard transport failed")
        .expect("provision_user_shard application failed")
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
