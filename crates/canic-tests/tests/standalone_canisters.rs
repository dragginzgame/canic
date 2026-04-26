// Category C - Artifact / deployment test (embedded config).
// These checks intentionally avoid the root hierarchy when one standalone
// canister is enough to exercise the behavior under test.

use candid::encode_one;
use canic::{
    Error,
    cdk::types::Principal,
    dto::{
        auth::AttestationKeySet,
        auth::{DelegatedToken, DelegatedTokenClaims, DelegationCert, DelegationProof},
        error::ErrorCode,
    },
    ids::cap,
};
use canic_internal::canister::TEST;
use canic_testing_internal::pic::{install_audit_scaling_probe, install_standalone_canister};
use canic_testkit::{
    artifacts::{
        WasmBuildProfile, build_wasm_canisters, read_wasm, test_target_dir, workspace_root_for,
    },
    pic::install_prebuilt_canister,
};
use std::sync::OnceLock;

const fn p(id: u8) -> Principal {
    Principal::from_slice(&[id; 29])
}

static SHARDING_ROOT_STUB_WASM: OnceLock<Vec<u8>> = OnceLock::new();

#[test]
fn standalone_scale_hub_perf_probe_succeeds() {
    let fixture = install_audit_scaling_probe(WasmBuildProfile::Fast);

    let response: Result<(bool, u64), Error> = fixture
        .pic()
        .query_call(fixture.canister_id(), "audit_plan_create_worker_probe", ())
        .expect("audit_plan_create_worker_probe transport query failed");
    let (_plan, perf) = response.expect("audit_plan_create_worker_probe application query failed");

    assert!(perf > 0, "expected positive local instruction count");
}

#[test]
fn standalone_test_auth_guard_rejects_bogus_token() {
    let fixture = install_standalone_canister("canister_test", TEST, WasmBuildProfile::Fast);

    let verify: Result<Result<(), Error>, Error> = fixture.pic().update_call(
        fixture.canister_id(),
        "test_verify_delegated_token",
        (bogus_delegated_token(),),
    );

    let err = verify
        .expect("test_verify_delegated_token transport failed")
        .expect_err("test_verify_delegated_token should reject bogus token");
    assert_eq!(err.code, ErrorCode::Unauthorized);
}

#[test]
fn prebuilt_canister_helper_installs_non_canic_wasm() {
    let fixture = install_prebuilt_canister(
        sharding_root_stub_wasm(),
        encode_one(()).expect("encode empty init"),
    );

    let key_set: Result<Result<AttestationKeySet, Error>, Error> =
        fixture
            .pic()
            .update_call(fixture.canister_id(), "canic_attestation_key_set", ());

    let key_set = key_set
        .expect("canic_attestation_key_set transport failed")
        .expect("canic_attestation_key_set application failed");
    assert_eq!(key_set.keys.len(), 1);
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
            ext: None,
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

fn sharding_root_stub_wasm() -> Vec<u8> {
    SHARDING_ROOT_STUB_WASM
        .get_or_init(|| {
            let workspace_root = workspace_root_for(env!("CARGO_MANIFEST_DIR"));
            let target_dir = test_target_dir(&workspace_root, "standalone-prebuilt-wasm");
            let canister = "sharding_root_stub";

            build_wasm_canisters(
                &workspace_root,
                &target_dir,
                &[canister],
                WasmBuildProfile::Fast,
                &[],
            );

            read_wasm(&target_dir, canister, WasmBuildProfile::Fast)
        })
        .clone()
}
