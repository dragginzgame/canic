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
    ids::{BuildNetwork, cap},
};
use canic_internal::canister;
use root::harness::{RootSetup, setup_root};
use std::time::Duration;

const fn p(id: u8) -> Principal {
    Principal::from_slice(&[id; 29])
}

///
/// DelegationFixture
///

struct DelegationFixture {
    setup: RootSetup,
    test_pid: Principal,
    shard_pid: Principal,
}

// Canonical signer-initiated delegation flow:
// user_shard requests delegation from root (no admin provisioning).

#[test]
fn delegation_provisioning_flow() {
    if !should_run_certified("delegation_provisioning_flow") {
        return;
    }

    let fixture = setup_delegation_fixture("delegation_provisioning_flow");
    let token = issue_test_token(
        &fixture,
        p(9),
        vec![fixture.test_pid],
        vec![cap::VERIFY.to_string()],
        60,
    );

    DelegationApi::verify_delegation_proof(&token.proof, fixture.setup.root_id)
        .expect("delegation proof must verify");
}

#[test]
fn delegated_token_flow() {
    if !should_run_certified("delegated_token_flow") {
        return;
    }

    let fixture = setup_delegation_fixture("delegated_token_flow");
    let caller = p(9);
    let token = issue_test_token(
        &fixture,
        caller,
        vec![fixture.test_pid],
        vec![cap::VERIFY.to_string()],
        60,
    );
    log_step(&format!(
        "issued token proof shard={}",
        token.proof.cert.shard_pid
    ));

    let verify: Result<Result<(), Error>, Error> = fixture.setup.pic.update_call_as(
        fixture.test_pid,
        caller,
        "test_verify_delegated_token",
        (token,),
    );

    verify
        .expect("test_verify_delegated_token transport failed")
        .expect("test_verify_delegated_token application failed");
}

#[test]
fn authenticated_rpc_flow() {
    if !should_run_certified("authenticated_rpc_flow") {
        return;
    }

    let fixture = setup_delegation_fixture("authenticated_rpc_flow");
    let subject = p(9);
    let mismatched_caller = p(10);
    let token = issue_test_token(
        &fixture,
        subject,
        vec![fixture.test_pid],
        vec![cap::VERIFY.to_string()],
        60,
    );

    // Establish that the token is otherwise valid in the same request pipeline.
    let ok_response: Result<Result<(), Error>, Error> = fixture.setup.pic.update_call_as(
        fixture.test_pid,
        subject,
        "test_verify_delegated_token",
        (token.clone(),),
    );
    ok_response
        .expect("test_verify_delegated_token transport failed for subject caller")
        .expect("test_verify_delegated_token should succeed for subject caller");

    log_step(&format!(
        "calling test_verify_delegated_token via test={}",
        fixture.test_pid
    ));
    let response: Result<Result<(), Error>, Error> = fixture.setup.pic.update_call_as(
        fixture.test_pid,
        mismatched_caller,
        "test_verify_delegated_token",
        (token,),
    );

    let err = response
        .expect("test_verify_delegated_token transport failed")
        .expect_err("test_verify_delegated_token should fail on subject mismatch");
    assert_eq!(err.code, ErrorCode::Unauthorized);
    assert!(
        err.message.contains("does not match caller"),
        "expected caller-subject binding rejection, got: {err:?}"
    );
}

#[test]
fn authenticated_rpc_flow_rejects_valid_token_missing_required_scope() {
    if !should_run_certified("authenticated_rpc_flow_rejects_valid_token_missing_required_scope") {
        return;
    }

    let fixture = setup_delegation_fixture(
        "authenticated_rpc_flow_rejects_valid_token_missing_required_scope",
    );
    let caller = p(9);
    let token = issue_test_token(
        &fixture,
        caller,
        vec![fixture.test_pid],
        vec![cap::READ.to_string()],
        60,
    );

    let response: Result<Result<(), Error>, Error> = fixture.setup.pic.update_call_as(
        fixture.test_pid,
        caller,
        "test_verify_delegated_token",
        (token,),
    );

    let err = response
        .expect("test_verify_delegated_token transport failed")
        .expect_err("missing required scope must deny");
    assert_eq!(err.code, ErrorCode::Unauthorized);
    assert!(
        err.message.contains("missing required scope"),
        "expected missing scope rejection, got: {err:?}"
    );
}

#[test]
fn authenticated_rpc_flow_rejects_expired_token() {
    if !should_run_certified("authenticated_rpc_flow_rejects_expired_token") {
        return;
    }

    let fixture = setup_delegation_fixture("authenticated_rpc_flow_rejects_expired_token");
    let caller = p(9);
    let token = issue_test_token(
        &fixture,
        caller,
        vec![fixture.test_pid],
        vec![cap::VERIFY.to_string()],
        1,
    );

    fixture.setup.pic.advance_time(Duration::from_secs(2));
    fixture.setup.pic.tick();

    let response: Result<Result<(), Error>, Error> = fixture.setup.pic.update_call_as(
        fixture.test_pid,
        caller,
        "test_verify_delegated_token",
        (token,),
    );

    let err = response
        .expect("test_verify_delegated_token transport failed")
        .expect_err("expired token must deny");
    assert_eq!(err.code, ErrorCode::Unauthorized);
    assert!(
        err.message.contains("expired"),
        "expected expired-token rejection, got: {err:?}"
    );
}

#[test]
fn delegated_token_request_rejected_on_invalid_claims() {
    if !should_run_certified("delegated_token_request_rejected_on_invalid_claims") {
        return;
    }

    let fixture = setup_delegation_fixture("delegated_token_request_rejected_on_invalid_claims");

    let now = now_secs();
    let claims = DelegatedTokenClaims {
        sub: p(9),
        shard_pid: fixture.shard_pid,
        aud: Vec::new(),
        scopes: Vec::new(),
        iat: now,
        exp: now + 60,
    };

    let issued: Result<Result<DelegatedToken, Error>, Error> =
        fixture
            .setup
            .pic
            .update_call(fixture.shard_pid, "user_shard_issue_token", (claims,));

    let err = issued
        .expect("user_shard_issue_token transport failed")
        .expect_err("user_shard_issue_token should fail on invalid claims");
    assert_eq!(err.code, ErrorCode::InvalidInput);
}

#[test]
fn authenticated_guard_rejects_bogus_token_on_local() {
    if !should_run_local("authenticated_guard_rejects_bogus_token_on_local") {
        return;
    }

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

// Build the standard certified delegation fixture used by most PocketIC flow tests.
fn setup_delegation_fixture(test_name: &str) -> DelegationFixture {
    log_step(&format!("{test_name}: setup root"));
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

    log_step(&format!("user_hub={user_hub_pid} root={}", setup.root_id));

    let shard_pid = create_user_shard(&setup, user_hub_pid, p(7));

    DelegationFixture {
        setup,
        test_pid,
        shard_pid,
    }
}

// Issue one delegated token from the test shard with caller-selected claims.
fn issue_test_token(
    fixture: &DelegationFixture,
    subject: Principal,
    aud: Vec<Principal>,
    scopes: Vec<String>,
    ttl_secs: u64,
) -> DelegatedToken {
    let now = now_secs();
    let claims = DelegatedTokenClaims {
        sub: subject,
        shard_pid: fixture.shard_pid,
        aud,
        scopes,
        iat: now,
        exp: now + ttl_secs,
    };

    let issued: Result<Result<DelegatedToken, Error>, Error> =
        fixture
            .setup
            .pic
            .update_call(fixture.shard_pid, "user_shard_issue_token", (claims,));

    issued
        .expect("user_shard_issue_token transport failed")
        .expect("user_shard_issue_token application failed")
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
            scopes: vec![cap::READ.to_string()],
            iat: 1,
            exp: 2,
        },
        proof: DelegationProof {
            cert: DelegationCert {
                root_pid: p(29),
                shard_pid: p(30),
                aud: vec![p(32)],
                scopes: vec![cap::READ.to_string()],
                issued_at: 1,
                expires_at: 2,
            },
            cert_sig: vec![0],
        },
        token_sig: vec![0],
    }
}
