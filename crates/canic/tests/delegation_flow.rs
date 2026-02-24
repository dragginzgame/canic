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
    let test_pid = setup
        .subnet_directory
        .get(&canister::TEST)
        .copied()
        .expect("test canister must exist in subnet directory");

    let now = now_secs();
    let claims = DelegatedTokenClaims {
        sub: p(9),
        shard_pid,
        aud: vec![test_pid],
        scopes: vec!["test:verify".to_string()],
        iat: now,
        exp: now + 60,
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
    let test_pid = setup
        .subnet_directory
        .get(&canister::TEST)
        .copied()
        .expect("test canister must exist in subnet directory");
    let caller = p(9);

    let now = now_secs();
    let claims = DelegatedTokenClaims {
        sub: caller,
        shard_pid,
        aud: vec![test_pid],
        scopes: vec!["test:verify".to_string()],
        iat: now,
        exp: now + 60,
    };

    let minted: Result<Result<DelegatedToken, Error>, Error> =
        setup
            .pic
            .update_call(shard_pid, "user_shard_mint_token", (claims,));

    let token = minted
        .expect("user_shard_mint_token transport failed")
        .expect("user_shard_mint_token application failed");
    log_step(&format!(
        "minted token proof shard={}",
        token.proof.cert.shard_pid
    ));

    let verify: Result<Result<(), Error>, Error> =
        setup
            .pic
            .update_call_as(test_pid, caller, "test_verify_delegated_token", (token,));

    verify
        .expect("test_verify_delegated_token transport failed")
        .expect("test_verify_delegated_token application failed");
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
    let test_pid = setup
        .subnet_directory
        .get(&canister::TEST)
        .copied()
        .expect("test canister must exist in subnet directory");

    let tenant = p(7);
    let shard_pid = create_user_shard(&setup, user_hub_pid, tenant);
    let subject = p(9);
    let mismatched_caller = p(10);

    let now = now_secs();
    let claims = DelegatedTokenClaims {
        sub: subject,
        shard_pid,
        aud: vec![test_pid],
        scopes: vec!["test:verify".to_string()],
        iat: now,
        exp: now + 60,
    };

    let minted: Result<Result<DelegatedToken, Error>, Error> =
        setup
            .pic
            .update_call(shard_pid, "user_shard_mint_token", (claims,));

    let token = minted
        .expect("user_shard_mint_token transport failed")
        .expect("user_shard_mint_token application failed");

    log_step(&format!(
        "calling test_verify_delegated_token via test={test_pid}"
    ));
    let response: Result<Result<(), Error>, Error> = setup.pic.update_call_as(
        test_pid,
        mismatched_caller,
        "test_verify_delegated_token",
        (token,),
    );

    let err = response
        .expect("test_verify_delegated_token transport failed")
        .expect_err("test_verify_delegated_token should fail on subject mismatch");
    assert_eq!(err.code, ErrorCode::Unauthorized);
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
        shard_pid,
        aud: Vec::new(),
        scopes: Vec::new(),
        iat: now,
        exp: now + 60,
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
fn authenticated_guard_rejects_bogus_token_on_local() {
    if !should_run_local("authenticated_guard_rejects_bogus_token_on_local") {
        return;
    }

    ensure_local_artifacts_built();

    log_step("authenticated_guard_rejects_bogus_token_on_local: setup root");
    let setup = setup_root();

    let test_pid = setup
        .subnet_directory
        .get(&canister::TEST)
        .copied()
        .expect("test canister must exist in subnet directory");

    // Intentionally bogus token data: local must reject this.
    let bogus = bogus_delegated_token();
    let verify: Result<Result<(), Error>, Error> =
        setup
            .pic
            .update_call(test_pid, "test_verify_delegated_token", (bogus,));

    let err = verify
        .expect("test_verify_delegated_token transport failed")
        .expect_err("test_verify_delegated_token should reject bogus token");
    assert_eq!(err.code, ErrorCode::Unauthorized);
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
        claims: DelegatedTokenClaims {
            sub: p(31),
            shard_pid: p(30),
            aud: vec![p(32)],
            scopes: vec!["read".to_string()],
            iat: 1,
            exp: 2,
        },
        proof: DelegationProof {
            cert: DelegationCert {
                root_pid: p(29),
                shard_pid: p(30),
                aud: vec![p(32)],
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
