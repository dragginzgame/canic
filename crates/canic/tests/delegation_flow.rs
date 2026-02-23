// Category C - Artifact / deployment test (embedded config).
// This test relies on embedded production config by design.

mod root;

use candid::encode_one;
use canic::{
    Error,
    api::{auth::DelegationApi, ic::network::NetworkApi},
    cdk::{types::Principal, utils::time::now_secs},
    dto::{
        auth::{
            DelegatedToken, DelegatedTokenClaims, DelegationCert, DelegationProof,
            DelegationProvisionRequest,
        },
        error::ErrorCode,
        rpc::{AuthenticatedRequest, CyclesRequest, Request, Response},
    },
    ids::BuildNetwork,
    protocol,
};
use canic_internal::canister;
use root::harness::{RootSetup, load_root_wasm_bytes, setup_root};
use std::{path::PathBuf, process::Command, sync::Once};

static DFX_BUILD_ONCE: Once = Once::new();

const fn p(id: u8) -> Principal {
    Principal::from_slice(&[id; 29])
}

// Canonical delegation flow:
// root provision prepare/get/finalize, then signer token prepare/get.

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

    provision_shard_delegation(&setup, shard_pid);
    let token = issue_token_two_step(&setup, shard_pid, claims);

    DelegationApi::verify_delegation_proof(&token.proof, setup.root_id)
        .expect("delegation proof must verify");
}

#[test]
fn delegation_provisioning_survives_root_upgrade() {
    if !should_run_certified("delegation_provisioning_survives_root_upgrade") {
        return;
    }

    log_step("delegation_provisioning_survives_root_upgrade: setup root");
    let setup = setup_root();

    let user_hub_pid = setup
        .subnet_directory
        .get(&canister::USER_HUB)
        .copied()
        .expect("user_hub must exist in subnet directory");
    let shard_pid = create_user_shard(&setup, user_hub_pid, p(7));

    let proof_before = provision_shard_delegation(&setup, shard_pid);
    DelegationApi::verify_delegation_proof(&proof_before, setup.root_id)
        .expect("delegation proof before upgrade must verify");

    let root_wasm = load_root_wasm_bytes();
    setup
        .pic
        .upgrade_canister(
            setup.root_id,
            root_wasm,
            encode_one(()).expect("encode upgrade args"),
            None,
        )
        .expect("root upgrade should succeed");
    wait_for_canister_ready(&setup, setup.root_id);

    let proof_after = provision_shard_delegation(&setup, shard_pid);
    DelegationApi::verify_delegation_proof(&proof_after, setup.root_id)
        .expect("delegation proof after upgrade must verify");
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

    provision_shard_delegation(&setup, shard_pid);
    let token = issue_token_two_step(&setup, shard_pid, claims);
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

    provision_shard_delegation(&setup, shard_pid);
    let token = issue_token_two_step(&setup, shard_pid, claims);

    log_step(&format!(
        "calling canic_response_authenticated via shard={shard_pid}",
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

    provision_shard_delegation(&setup, shard_pid);

    let prepared: Result<Result<(), Error>, Error> =
        setup
            .pic
            .update_call(shard_pid, "user_shard_issue_token_prepare", (claims,));

    let err = prepared
        .expect("user_shard_issue_token_prepare transport failed")
        .expect_err("user_shard_issue_token_prepare should fail on invalid claims");
    assert_eq!(err.code, ErrorCode::InvalidInput);
}

#[test]
fn authenticated_guard_is_bypassed_on_local_with_token_arg() {
    if !should_run_local("authenticated_guard_is_bypassed_on_local_with_token_arg") {
        return;
    }

    ensure_local_artifacts_built();

    log_step("authenticated_guard_is_bypassed_on_local_with_token_arg: setup root");
    let setup = setup_root();

    let test_pid = setup
        .subnet_directory
        .get(&canister::TEST)
        .copied()
        .expect("test canister must exist in subnet directory");

    // Intentionally bogus token data: this should only pass when local auth
    // bypass is active for `auth::authenticated()`.
    let bogus = bogus_delegated_token();
    let verify: Result<Result<(), Error>, Error> =
        setup
            .pic
            .update_call(test_pid, "test_verify_delegated_token", (bogus,));

    verify
        .expect("test_verify_delegated_token transport failed")
        .expect("test_verify_delegated_token application failed");
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

fn should_run_local(test_name: &str) -> bool {
    if NetworkApi::build_network() == Some(BuildNetwork::Ic) {
        log_step(&format!("{test_name}: skipped (ic build)"));
        false
    } else {
        true
    }
}

fn wait_for_canister_ready(setup: &RootSetup, canister_id: Principal) {
    for _ in 0..120 {
        setup.pic.tick();
        let ready: bool = setup
            .pic
            .query_call(canister_id, protocol::CANIC_READY, ())
            .expect("canic_ready query");
        if ready {
            return;
        }
    }

    panic!("canister {canister_id} did not become ready after upgrade");
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

fn provision_shard_delegation(setup: &RootSetup, shard_pid: Principal) -> DelegationProof {
    let now = now_secs();
    let request = DelegationProvisionRequest {
        cert: DelegationCert {
            v: 1,
            signer_pid: shard_pid,
            audiences: vec!["login".to_string()],
            scopes: vec!["read".to_string()],
            issued_at: now,
            expires_at: now + 600,
        },
        signer_targets: vec![shard_pid],
        verifier_targets: Vec::new(),
    };

    let prepared: Result<Result<(), Error>, Error> = setup.pic.update_call_as(
        setup.root_id,
        setup.root_id,
        protocol::CANIC_DELEGATION_PROVISION_PREPARE,
        (request,),
    );
    prepared
        .expect("delegation provision prepare transport failed")
        .expect("delegation provision prepare application failed");

    let proof: Result<Result<DelegationProof, Error>, Error> = setup.pic.query_call_as(
        setup.root_id,
        setup.root_id,
        protocol::CANIC_DELEGATION_PROVISION_GET,
        (),
    );
    let proof = proof
        .expect("delegation provision get transport failed")
        .expect("delegation provision get application failed");

    let finalized: Result<Result<canic::dto::auth::DelegationProvisionResponse, Error>, Error> =
        setup.pic.update_call_as(
            setup.root_id,
            setup.root_id,
            protocol::CANIC_DELEGATION_PROVISION_FINALIZE,
            (proof.clone(),),
        );
    finalized
        .expect("delegation provision finalize transport failed")
        .expect("delegation provision finalize application failed");

    proof
}

fn issue_token_two_step(
    setup: &RootSetup,
    shard_pid: Principal,
    claims: DelegatedTokenClaims,
) -> DelegatedToken {
    let prepared: Result<Result<(), Error>, Error> =
        setup
            .pic
            .update_call(shard_pid, "user_shard_issue_token_prepare", (claims,));
    prepared
        .expect("user_shard_issue_token_prepare transport failed")
        .expect("user_shard_issue_token_prepare application failed");

    let token: Result<Result<DelegatedToken, Error>, Error> =
        setup
            .pic
            .query_call(shard_pid, "user_shard_issue_token_get", ());
    token
        .expect("user_shard_issue_token_get transport failed")
        .expect("user_shard_issue_token_get application failed")
}

fn bogus_delegated_token() -> DelegatedToken {
    DelegatedToken {
        v: 1,
        claims: DelegatedTokenClaims {
            sub: p(31),
            aud: "local-auth-bypass-test".to_string(),
            scopes: vec!["read".to_string()],
            iat: 1,
            exp: 2,
            ext: None,
            nonce: None,
        },
        proof: DelegationProof {
            cert: DelegationCert {
                v: 1,
                signer_pid: p(30),
                audiences: vec!["local-auth-bypass-test".to_string()],
                scopes: vec!["read".to_string()],
                issued_at: 1,
                expires_at: 2,
            },
            cert_sig: vec![0],
        },
        token_sig: vec![0],
    }
}

fn ensure_local_artifacts_built() {
    DFX_BUILD_ONCE.call_once(|| {
        let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|p| p.parent())
            .map(PathBuf::from)
            .expect("workspace root");

        let output = Command::new("dfx")
            .current_dir(&workspace_root)
            .env("DFX_NETWORK", "local")
            .args(["build", "--all"])
            .output()
            .expect("failed to run `dfx build --all`");

        assert!(
            output.status.success(),
            "dfx build --all failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    });
}
