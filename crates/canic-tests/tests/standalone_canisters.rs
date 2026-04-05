// Category C - Artifact / deployment test (embedded config).
// These checks intentionally avoid the root hierarchy when one standalone
// canister is enough to exercise the behavior under test.

use canic::{
    Error,
    cdk::types::Principal,
    dto::{
        auth::{DelegatedToken, DelegatedTokenClaims, DelegationCert, DelegationProof},
        error::ErrorCode,
    },
    ids::cap,
};
use canic_internal::canister::{SCALE_HUB, TEST};
use canic_testing_internal::pic::install_standalone_canister;
use canic_testkit::artifacts::WasmBuildProfile;

const fn p(id: u8) -> Principal {
    Principal::from_slice(&[id; 29])
}

#[test]
fn standalone_scale_hub_perf_probe_succeeds() {
    let fixture =
        install_standalone_canister("canister_scale_hub", SCALE_HUB, WasmBuildProfile::Fast);

    let response: Result<(bool, u64), Error> = fixture
        .pic
        .query_call(fixture.canister_id, "plan_create_worker_perf_test", ())
        .expect("plan_create_worker_perf_test transport query failed");
    let (_plan, perf) = response.expect("plan_create_worker_perf_test application query failed");

    assert!(perf > 0, "expected positive local instruction count");
}

#[test]
fn standalone_test_auth_guard_rejects_bogus_token() {
    let fixture = install_standalone_canister("canister_test", TEST, WasmBuildProfile::Fast);

    let verify: Result<Result<(), Error>, Error> = fixture.pic.update_call(
        fixture.canister_id,
        "test_verify_delegated_token",
        (bogus_delegated_token(),),
    );

    let err = verify
        .expect("test_verify_delegated_token transport failed")
        .expect_err("test_verify_delegated_token should reject bogus token");
    assert_eq!(err.code, ErrorCode::Unauthorized);
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
