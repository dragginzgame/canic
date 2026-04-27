use std::sync::{Mutex, OnceLock};

use super::*;

static DELEGATION_ADMIN_FIXTURE_CACHE: OnceLock<Mutex<Option<DelegationAdminCachedData>>> =
    OnceLock::new();

///
/// DelegationAdminCachedData
///

#[derive(Clone)]
pub struct DelegationAdminCachedData {
    pub root_id: Principal,
    pub signer_id: Principal,
    pub verifier_id: Principal,
    pub delegated_subject: Principal,
    pub stale_token: DelegatedToken,
    pub current_token: DelegatedToken,
    pub root_public_key: Vec<u8>,
    pub shard_public_key: Vec<u8>,
}

///
/// DelegationAdminFixture
///

pub struct DelegationAdminFixture {
    pub setup: CachedInstalledRoot,
    pub root_id: Principal,
    pub signer_id: Principal,
    pub verifier_id: Principal,
    pub delegated_subject: Principal,
    pub stale_token: DelegatedToken,
    pub current_token: DelegatedToken,
    pub root_public_key: Vec<u8>,
    pub shard_public_key: Vec<u8>,
}

// Build a reusable root/signer/verifier setup with two proof generations.
pub fn delegation_admin_fixture(_subject_seed: u8) -> DelegationAdminFixture {
    let setup = install_test_root_with_verifier_cached();
    let root_id = setup.root_id;
    let signer_id = setup.signer_id;
    let verifier_id = setup.verifier_id.expect("cached verifier must exist");
    let cached = delegation_admin_cached_data(setup.pic.pic(), root_id, signer_id, verifier_id);

    DelegationAdminFixture {
        setup,
        root_id,
        signer_id,
        verifier_id,
        delegated_subject: cached.delegated_subject,
        stale_token: cached.stale_token,
        current_token: cached.current_token,
        root_public_key: cached.root_public_key,
        shard_public_key: cached.shard_public_key,
    }
}

// Reuse the same issued admin tokens and public keys across restored verifier baselines.
fn delegation_admin_cached_data(
    pic: &Pic,
    root_id: Principal,
    signer_id: Principal,
    verifier_id: Principal,
) -> DelegationAdminCachedData {
    let cache = DELEGATION_ADMIN_FIXTURE_CACHE.get_or_init(|| Mutex::new(None));
    let mut cache = cache
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

    if let Some(cached) = cache.as_ref()
        && cached.root_id == root_id
        && cached.signer_id == signer_id
        && cached.verifier_id == verifier_id
    {
        return cached.clone();
    }

    let delegated_subject = Principal::from_slice(&[83; 29]);
    let stale_token =
        issue_test_delegated_token(pic, root_id, signer_id, verifier_id, delegated_subject, 60);
    let current_token =
        issue_test_delegated_token(pic, root_id, signer_id, verifier_id, delegated_subject, 120);
    let (root_public_key, shard_public_key) = delegation_public_keys(pic, root_id);

    let generated = DelegationAdminCachedData {
        root_id,
        signer_id,
        verifier_id,
        delegated_subject,
        stale_token,
        current_token,
        root_public_key,
        shard_public_key,
    };
    *cache = Some(generated.clone());
    generated
}

// Issue a test delegated token for the requested verifier audience and TTL.
pub fn issue_test_delegated_token(
    pic: &Pic,
    root_id: Principal,
    signer_id: Principal,
    _verifier_id: Principal,
    delegated_subject: Principal,
    ttl_seconds: u64,
) -> DelegatedToken {
    let now: Result<u64, Error> =
        query_call_as(pic, root_id, Principal::anonymous(), "root_now_secs", ());
    let now = now.expect("query root now_secs failed");
    let claims = DelegatedTokenClaims {
        sub: delegated_subject,
        shard_pid: signer_id,
        scopes: vec![cap::VERIFY.to_string()],
        aud: DelegationAudience::Any,
        iat: now,
        exp: now + ttl_seconds,
        ext: None,
    };
    let issued_token: Result<DelegatedToken, Error> = update_call_as(
        pic,
        root_id,
        root_id,
        "root_issue_test_delegated_token",
        (claims,),
    );

    issued_token.expect("delegated token issuance failed")
}

// Query the root test public keys used for proof installation hooks.
pub fn delegation_public_keys(pic: &Pic, root_id: Principal) -> (Vec<u8>, Vec<u8>) {
    let keys: Result<(Vec<u8>, Vec<u8>), Error> = query_call_as(
        pic,
        root_id,
        Principal::anonymous(),
        "root_test_delegation_public_keys",
        (),
    );

    keys.expect("query test delegation keys failed")
}

// Install proof material into the root verifier test hook.
pub fn install_root_test_delegation_material(
    pic: &Pic,
    root_id: Principal,
    proof: canic_core::dto::auth::DelegationProof,
    root_public_key: Vec<u8>,
    shard_public_key: Vec<u8>,
) {
    let install: Result<(), Error> = update_call_as(
        pic,
        root_id,
        root_id,
        "root_install_test_delegation_material",
        (proof, root_public_key, shard_public_key),
    );

    install.expect("root test delegation material install must succeed");
}

// Install proof material into a signer/verifier test hook.
pub fn install_signer_test_delegation_material(
    pic: &Pic,
    canister_id: Principal,
    caller: Principal,
    proof: canic_core::dto::auth::DelegationProof,
    root_public_key: Vec<u8>,
    shard_public_key: Vec<u8>,
) {
    let install: Result<(), Error> = update_call_as(
        pic,
        canister_id,
        caller,
        "signer_install_test_delegation_material",
        (proof, root_public_key, shard_public_key),
    );

    install.expect("signer delegation material install must succeed");
}

// Verify that keyed lookup fails as a proof miss before any prewarm repair.
pub fn assert_token_verify_proof_missing(
    pic: &Pic,
    verifier_id: Principal,
    delegated_subject: Principal,
    token: DelegatedToken,
) {
    let denied: Result<(), Error> = update_call_as(
        pic,
        verifier_id,
        delegated_subject,
        "signer_verify_token",
        (token,),
    );
    let err = denied.expect_err("stale verifier proof must fail closed");
    assert_eq!(err.code, ErrorCode::Unauthorized);
    assert!(
        err.message.contains("delegation proof miss"),
        "expected proof-miss denial, got: {err:?}"
    );
}

// Dispatch a root prewarm admin command and decode the typed response.
pub fn prewarm_verifiers(
    pic: &Pic,
    root_id: Principal,
    proof: canic_core::dto::auth::DelegationProof,
    verifier_targets: Vec<Principal>,
) -> DelegationAdminResponse {
    let prewarm: Result<DelegationAdminResponse, Error> = update_call_as(
        pic,
        root_id,
        Principal::anonymous(),
        "canic_delegation_admin",
        (DelegationAdminCommand::PrewarmVerifiers(
            DelegationVerifierProofPushRequest {
                proof,
                verifier_targets,
            },
        ),),
    );

    prewarm.expect("prewarm admin call must succeed")
}

// Dispatch a root repair admin command and preserve the typed error surface.
pub fn repair_verifiers(
    pic: &Pic,
    root_id: Principal,
    proof: canic_core::dto::auth::DelegationProof,
    verifier_targets: Vec<Principal>,
) -> Result<DelegationAdminResponse, Error> {
    update_call_as(
        pic,
        root_id,
        Principal::anonymous(),
        "canic_delegation_admin",
        (DelegationAdminCommand::RepairVerifiers(
            DelegationVerifierProofPushRequest {
                proof,
                verifier_targets,
            },
        ),),
    )
}

pub fn bogus_delegated_token(root_pid: Principal, shard_pid: Principal) -> DelegatedToken {
    let user = Principal::from_slice(&[77; 29]);
    DelegatedToken {
        claims: DelegatedTokenClaims {
            sub: user,
            shard_pid,
            aud: DelegationAudience::Any,
            scopes: vec![cap::VERIFY.to_string()],
            iat: 1,
            exp: 2,
            ext: None,
        },
        proof: canic_core::dto::auth::DelegationProof {
            cert: canic_core::dto::auth::DelegationCert {
                root_pid,
                shard_pid,
                issued_at: 1,
                expires_at: 2,
                scopes: vec![cap::VERIFY.to_string()],
                aud: DelegationAudience::Any,
            },
            cert_sig: vec![0],
        },
        token_sig: vec![0],
    }
}
