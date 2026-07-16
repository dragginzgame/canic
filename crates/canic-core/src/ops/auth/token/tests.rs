//! Tests for delegated-token preparation, verification, configuration, and typed errors.

use super::*;
use crate::{
    config::{
        Config,
        schema::{CanisterAuthConfig, CanisterKind, ChainKeyRootProofConfig},
    },
    domain::auth::MAINNET_IC_ROOT_PUBLIC_KEY_RAW,
    dto::error::ErrorCode,
    ids::SubnetRole,
    ops::auth::delegated::chain_key::ChainKeySignatureVerificationInput,
    storage::stable::env::{Env, EnvData, EnvRecord},
    test::config::ConfigTestBuilder,
};
use k256::ecdsa::{
    Signature as K256TestSignature, SigningKey as K256SigningKey, signature::hazmat::PrehashSigner,
};
use std::fmt::Write as _;

fn root_pid() -> Principal {
    Principal::from_slice(&[1; 29])
}

fn cfg(network: &str, root_key: Option<Vec<u8>>) -> DelegatedTokenConfig {
    let mut cfg = DelegatedTokenConfig {
        enabled: true,
        root_canister_id: Some(root_pid().to_string()),
        ic_root_public_key_raw_hex: root_key.map(hex),
        root_proof_mode: "chain_key_batch".to_string(),
        chain_key_root_proof: ChainKeyRootProofConfig::default(),
        network: network.to_string(),
        max_ttl_secs: None,
    };
    install_chain_key_policy(&mut cfg, "key_1");
    cfg
}

fn chain_key_cfg(network: &str, root_key: Vec<u8>, key_id: &str) -> DelegatedTokenConfig {
    let mut cfg = cfg(network, Some(root_key));
    install_chain_key_policy(&mut cfg, key_id);
    cfg
}

fn install_chain_key_policy(cfg: &mut DelegatedTokenConfig, key_id: &str) {
    cfg.root_proof_mode = "chain_key_batch".to_string();
    cfg.chain_key_root_proof.key_id = Some(key_id.to_string());
    cfg.chain_key_root_proof.derivation_path_hash_hex =
        Some("fe51a87b988d221227b134c48f36787e891a902dcb5d48ea5f94cff8bfed5a16".to_string());
    cfg.chain_key_root_proof.derivation_path_hex = Some(vec![
        "63616e6963".to_string(),
        "64656c65676174696f6e".to_string(),
    ]);
    cfg.chain_key_root_proof.public_key_hex = Some("02".repeat(33));
    cfg.chain_key_root_proof.key_version = Some(4);
    cfg.chain_key_root_proof.min_accepted_key_version = Some(4);
    cfg.chain_key_root_proof.min_accepted_proof_epoch = Some(7);
    cfg.chain_key_root_proof.min_accepted_registry_epoch = Some(8);
    cfg.chain_key_root_proof.valid_from_ns = Some(10);
    cfg.chain_key_root_proof.accept_until_ns = Some(1_000);
    cfg.chain_key_root_proof.max_revocation_latency_ns = Some(600);
}

fn hex(bytes: Vec<u8>) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(&mut out, "{byte:02x}").expect("hex write should not fail");
    }
    out
}

fn local_key() -> Vec<u8> {
    vec![7; IC_ROOT_PUBLIC_KEY_RAW_LENGTH]
}

fn mainnet_key() -> Vec<u8> {
    MAINNET_IC_ROOT_PUBLIC_KEY_RAW.to_vec()
}

fn p(id: u8) -> Principal {
    Principal::from_slice(&[id; 29])
}

#[test]
fn chain_key_ecdsa_signature_verifier_accepts_valid_prehash_signature() {
    let signing_key = K256SigningKey::from_slice(&[7; 32]).expect("test signing key should parse");
    let public_key = signing_key.verifying_key().to_sec1_point(true);
    let message_hash = [42; 32];
    let signature: K256TestSignature = signing_key
        .sign_prehash(&message_hash)
        .expect("test prehash signature should sign");
    let signature_bytes = signature.to_bytes();
    verify_chain_key_ecdsa_signature(ChainKeySignatureVerificationInput {
        algorithm: ChainKeyAlgorithm::EcdsaSecp256k1,
        public_key: public_key.as_bytes(),
        message_hash,
        signature: signature_bytes.as_ref(),
    })
    .expect("valid chain-key ECDSA prehash signature should verify");
}

#[test]
fn chain_key_ecdsa_signature_verifier_rejects_altered_signature() {
    let signing_key = K256SigningKey::from_slice(&[7; 32]).expect("test signing key should parse");
    let public_key = signing_key.verifying_key().to_sec1_point(true);
    let message_hash = [42; 32];
    let signature: K256TestSignature = signing_key
        .sign_prehash(&message_hash)
        .expect("test prehash signature should sign");
    let mut signature_bytes = signature.to_bytes().to_vec();
    signature_bytes[0] ^= 1;
    verify_chain_key_ecdsa_signature(ChainKeySignatureVerificationInput {
        algorithm: ChainKeyAlgorithm::EcdsaSecp256k1,
        public_key: public_key.as_bytes(),
        message_hash,
        signature: &signature_bytes,
    })
    .expect_err("altered chain-key ECDSA signature must reject");
}

#[test]
fn active_delegation_proof_unavailable_maps_to_auth_material_stale() {
    crate::ops::storage::auth::AuthStateOps::clear_active_delegation_proof();

    let err = active_delegation_proof_unavailable_error(20);
    let public = err
        .public_error()
        .expect("missing active proof must be public");

    assert_eq!(public.code, ErrorCode::AuthMaterialStale);
}

#[test]
fn token_prepare_outliving_active_proof_maps_to_auth_material_stale() {
    let err = map_prepare_delegated_token_error(PrepareDelegatedTokenError::TokenOutlivesCert);
    let public = err
        .public_error()
        .expect("stale active proof must be public");

    assert_eq!(public.code, ErrorCode::AuthMaterialStale);
}

#[test]
fn token_prepare_expired_active_proof_maps_to_auth_proof_expired() {
    let err = map_prepare_delegated_token_error(PrepareDelegatedTokenError::CertExpired);
    let public = err
        .public_error()
        .expect("expired active proof must be public");

    assert_eq!(public.code, ErrorCode::AuthProofExpired);
}

#[test]
fn delegated_token_verify_expiry_preserves_machine_readable_codes() {
    let cases: [(VerifyDelegatedTokenError, ErrorCode); 2] = [
        (
            VerifyDelegatedTokenError::TokenExpired,
            ErrorCode::AuthTokenExpired,
        ),
        (
            VerifyDelegatedTokenError::CertExpired,
            ErrorCode::AuthProofExpired,
        ),
    ];

    for (err, expected) in cases {
        let internal = map_verify_delegated_token_error(err);
        let public = internal
            .public_error()
            .expect("expiry verification failures must be public");
        assert_eq!(public.code, expected);
    }
}

#[test]
fn delegated_token_verify_preserves_typed_proof_callback_causes() {
    let root = map_verify_delegated_token_error(VerifyDelegatedTokenError::<
        InternalError,
        InternalError,
    >::RootProofInvalid(
        InternalError::auth_material_stale("root policy changed"),
    ));
    let issuer = map_verify_delegated_token_error(VerifyDelegatedTokenError::<
        InternalError,
        InternalError,
    >::IssuerProofInvalid(
        InternalError::invalid_input("issuer signature invalid"),
    ));

    assert_eq!(
        root.public_error().map(|err| err.code),
        Some(ErrorCode::AuthMaterialStale)
    );
    assert_eq!(
        issuer.public_error().map(|err| err.code),
        Some(ErrorCode::InvalidInput)
    );
}

#[test]
fn chain_key_root_proof_failures_keep_boundary_specific_codes() {
    let cases = [
        (
            ChainKeyRootProofError::Expired { target: "batch" },
            ErrorCode::AuthProofExpired,
        ),
        (
            ChainKeyRootProofError::NotYetValid { target: "batch" },
            ErrorCode::AuthProofPending,
        ),
        (
            ChainKeyRootProofError::Expired {
                target: "root_key_policy",
            },
            ErrorCode::AuthMaterialStale,
        ),
        (
            ChainKeyRootProofError::ProofEpochTooOld { min: 8, found: 7 },
            ErrorCode::AuthMaterialStale,
        ),
        (
            ChainKeyRootProofError::InvalidSignatureLength { len: 3 },
            ErrorCode::InvalidInput,
        ),
    ];

    for (err, expected) in cases {
        let mapped = map_chain_key_root_proof_error(err);
        assert_eq!(mapped.public_error().map(|err| err.code), Some(expected));
    }
}

#[test]
fn auth_proof_verifier_config_accepts_mainnet_with_known_mainnet_root_key() {
    let cfg = cfg("mainnet", Some(mainnet_key()));

    let verifier = AuthOps::auth_proof_verifier_config_from(&cfg).expect("mainnet key should pass");

    assert_eq!(verifier.network, DelegatedAuthNetwork::Mainnet);
    assert_eq!(verifier.root_canister_id, root_pid());
    assert_eq!(verifier.ic_root_public_key_raw, mainnet_key());
    assert_eq!(verifier.root_proof_mode, RootProofMode::ChainKeyBatch);
    assert!(verifier.chain_key_root.is_some());
}

#[test]
fn auth_proof_verifier_config_rejects_non_chain_key_root_proof_mode() {
    let mut cfg = cfg("mainnet", Some(mainnet_key()));
    cfg.root_proof_mode = "canister_signature".to_string();

    AuthOps::auth_proof_verifier_config_from(&cfg)
        .expect_err("must reject non-chain-key root proof mode");
}

#[test]
fn auth_proof_verifier_config_rejects_mainnet_without_root_key() {
    let cfg = cfg("mainnet", None);

    AuthOps::auth_proof_verifier_config_from(&cfg).expect_err("mainnet requires explicit root key");
}

#[test]
fn auth_proof_verifier_config_rejects_mainnet_with_local_root_key() {
    let cfg = cfg("mainnet", Some(local_key()));

    AuthOps::auth_proof_verifier_config_from(&cfg).expect_err("mainnet must reject local root key");
}

#[test]
fn auth_proof_verifier_config_local_requires_explicit_root_key() {
    let cfg = cfg("local", None);

    AuthOps::auth_proof_verifier_config_from(&cfg)
        .expect_err("local verifier requires explicit root key");
}

#[test]
fn auth_proof_verifier_config_local_accepts_explicit_local_root_key() {
    let cfg = cfg("local", Some(local_key()));

    let verifier = AuthOps::auth_proof_verifier_config_from(&cfg).expect("local key should pass");

    assert_eq!(verifier.network, DelegatedAuthNetwork::Local);
    assert_eq!(verifier.ic_root_public_key_raw, local_key());
}

#[test]
fn auth_proof_verifier_config_pocketic_requires_explicit_root_key() {
    let cfg = cfg("pocketic", None);

    AuthOps::auth_proof_verifier_config_from(&cfg)
        .expect_err("pocketic verifier requires explicit root key");
}

#[test]
fn auth_proof_verifier_config_pocketic_rejects_explicit_mainnet_root_key() {
    let cfg = cfg("pocketic", Some(mainnet_key()));

    AuthOps::auth_proof_verifier_config_from(&cfg)
        .expect_err("pocketic must not accept mainnet root key");
}

#[test]
fn auth_proof_verifier_config_pocketic_accepts_explicit_pocketic_root_key() {
    let cfg = cfg("pocketic", Some(local_key()));

    let verifier =
        AuthOps::auth_proof_verifier_config_from(&cfg).expect("pocketic key should pass");

    assert_eq!(verifier.network, DelegatedAuthNetwork::PocketIc);
    assert_eq!(verifier.ic_root_public_key_raw, local_key());
}

#[test]
fn auth_proof_verifier_config_local_rejects_explicit_mainnet_root_key() {
    let cfg = cfg("local", Some(mainnet_key()));

    AuthOps::auth_proof_verifier_config_from(&cfg)
        .expect_err("local must reject explicit mainnet root key");
}

#[test]
fn auth_proof_verifier_config_testnet_requires_explicit_root_key() {
    let cfg = cfg("testnet", None);

    AuthOps::auth_proof_verifier_config_from(&cfg)
        .expect_err("testnet verifier requires explicit root key");
}

#[test]
fn auth_proof_verifier_config_testnet_accepts_explicit_test_root_key() {
    let cfg = cfg("testnet", Some(local_key()));

    let verifier = AuthOps::auth_proof_verifier_config_from(&cfg).expect("testnet key should pass");

    assert_eq!(verifier.network, DelegatedAuthNetwork::Testnet);
    assert_eq!(verifier.ic_root_public_key_raw, local_key());
}

#[test]
fn auth_proof_verifier_config_chain_key_local_accepts_test_key_when_allowed() {
    let mut cfg = chain_key_cfg("local", local_key(), "test_key_1");
    cfg.chain_key_root_proof.allow_test_key = true;

    let verifier =
        AuthOps::auth_proof_verifier_config_from(&cfg).expect("chain-key config should pass");
    let chain_key_root = verifier
        .chain_key_root
        .as_ref()
        .expect("chain-key policy should be configured");

    assert_eq!(verifier.root_proof_mode, RootProofMode::ChainKeyBatch);
    assert_eq!(chain_key_root.policy.root_canister_id, root_pid());
    assert_eq!(chain_key_root.policy.key_id.name, "test_key_1");
    assert_eq!(
        chain_key_root.policy.derivation_path_hash,
        [
            0xfe, 0x51, 0xa8, 0x7b, 0x98, 0x8d, 0x22, 0x12, 0x27, 0xb1, 0x34, 0xc4, 0x8f, 0x36,
            0x78, 0x7e, 0x89, 0x1a, 0x90, 0x2d, 0xcb, 0x5d, 0x48, 0xea, 0x5f, 0x94, 0xcf, 0xf8,
            0xbf, 0xed, 0x5a, 0x16,
        ]
    );
    assert_eq!(chain_key_root.policy.public_key, vec![0x02; 33]);
    assert_eq!(chain_key_root.policy.max_revocation_latency_ns, 600);
    assert_eq!(chain_key_root.policy.build_network, BuildNetwork::Local);
    assert!(chain_key_root.allow_test_chain_key);
}

#[test]
fn auth_proof_verifier_config_chain_key_rejects_invalid_public_key() {
    let mut cfg = chain_key_cfg("local", local_key(), "test_key_1");
    cfg.chain_key_root_proof.allow_test_key = true;
    cfg.chain_key_root_proof.public_key_hex = Some("00".repeat(33));

    AuthOps::auth_proof_verifier_config_from(&cfg)
        .expect_err("invalid chain-key public key must reject");
}

#[test]
fn auth_proof_verifier_config_chain_key_rejects_mainnet_test_key() {
    let cfg = chain_key_cfg("mainnet", mainnet_key(), "test_key_1");

    AuthOps::auth_proof_verifier_config_from(&cfg).expect_err("mainnet must reject test_key_1");
}

#[test]
fn auth_proof_verifier_config_chain_key_rejects_unapproved_local_test_key() {
    let cfg = chain_key_cfg("local", local_key(), "test_key_1");

    AuthOps::auth_proof_verifier_config_from(&cfg)
        .expect_err("local test key requires explicit opt-in");
}

#[test]
fn delegated_token_verifier_gate_rejects_issuer_only_canister() {
    install_verifier_test_config(false, true, false);

    require_current_canister_delegated_token_verifier()
        .expect_err("issuer-only canister must not verify delegated tokens");
}

#[test]
fn delegated_token_verifier_gate_rejects_role_attestation_cache_without_verifier_flag() {
    install_verifier_test_config(false, false, true);

    require_current_canister_delegated_token_verifier()
        .expect_err("role-attestation cache must not enable delegated-token verification");
}

#[test]
fn delegated_token_verifier_gate_accepts_current_canister_verifier() {
    install_verifier_test_config(true, false, false);

    require_current_canister_delegated_token_verifier()
        .expect("explicit verifier canister should pass the execution gate");
}

fn install_verifier_test_config(
    delegated_token_verifier: bool,
    delegated_token_issuer: bool,
    role_attestation_cache: bool,
) {
    let mut canister_cfg = ConfigTestBuilder::canister_config(CanisterKind::Service);
    canister_cfg.auth = CanisterAuthConfig {
        delegated_token_issuer,
        delegated_token_verifier,
        role_attestation_cache,
    };

    let mut cfg = ConfigTestBuilder::new()
        .with_prime_canister("project_instance", canister_cfg)
        .build();
    cfg.auth.delegated_tokens.network = "local".to_string();
    cfg.auth.delegated_tokens.root_proof_mode = "chain_key_batch".to_string();
    cfg.auth.delegated_tokens.root_canister_id = Some(root_pid().to_string());
    cfg.auth.delegated_tokens.ic_root_public_key_raw_hex = Some(hex(local_key()));
    cfg.auth.delegated_tokens.chain_key_root_proof.key_id = Some("key_1".to_string());
    cfg.auth
        .delegated_tokens
        .chain_key_root_proof
        .derivation_path_hash_hex =
        Some("fe51a87b988d221227b134c48f36787e891a902dcb5d48ea5f94cff8bfed5a16".to_string());
    cfg.auth
        .delegated_tokens
        .chain_key_root_proof
        .derivation_path_hex = Some(vec![
        "63616e6963".to_string(),
        "64656c65676174696f6e".to_string(),
    ]);
    cfg.auth
        .delegated_tokens
        .chain_key_root_proof
        .public_key_hex = Some("02".repeat(33));
    cfg.auth.delegated_tokens.chain_key_root_proof.key_version = Some(4);
    cfg.auth
        .delegated_tokens
        .chain_key_root_proof
        .min_accepted_key_version = Some(4);
    cfg.auth
        .delegated_tokens
        .chain_key_root_proof
        .min_accepted_proof_epoch = Some(7);
    cfg.auth
        .delegated_tokens
        .chain_key_root_proof
        .min_accepted_registry_epoch = Some(8);
    cfg.auth.delegated_tokens.chain_key_root_proof.valid_from_ns = Some(10);
    cfg.auth
        .delegated_tokens
        .chain_key_root_proof
        .accept_until_ns = Some(1_000);
    cfg.auth
        .delegated_tokens
        .chain_key_root_proof
        .max_revocation_latency_ns = Some(600);
    Config::reset_for_tests();
    Config::init_from_model_for_tests(cfg).expect("test config should install");

    Env::import(EnvData {
        record: EnvRecord {
            prime_root_pid: Some(root_pid()),
            subnet_role: Some(SubnetRole::PRIME),
            subnet_pid: Some(p(9)),
            root_pid: Some(root_pid()),
            canister_role: Some(CanisterRole::new("project_instance")),
            parent_pid: Some(root_pid()),
        },
    });
}
