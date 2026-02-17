// Category C - Artifact / deployment test (embedded config).
// This test relies on embedded production config by design.

mod root;

use canic::{
    Error,
    api::{auth::DelegationApi, ic::network::NetworkApi},
    cdk::{types::Principal, utils::time::now_secs},
    dto::{
        auth::{DelegatedToken, DelegatedTokenClaims, DelegationCert, DelegationProof},
        error::ErrorCode,
        rpc::{AuthenticatedRequest, CyclesRequest, Request, Response},
    },
    ids::BuildNetwork,
};
use canic_internal::canister;
use root::harness::{RootSetup, setup_root};
use std::{path::PathBuf, process::Command, sync::Once};

static DFX_BUILD_ONCE: Once = Once::new();

const fn p(id: u8) -> Principal {
    Principal::from_slice(&[id; 29])
}

// Canonical signer-initiated delegation flow:
// user_shard requests delegation from root (no admin provisioning).

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

    let minted: Result<Result<DelegatedToken, Error>, Error> =
        setup
            .pic
            .update_call(shard_pid, "user_shard_mint_token", (claims,));

    let err = minted
        .expect("user_shard_mint_token transport failed")
        .expect_err("user_shard_mint_token should fail on invalid claims");
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
