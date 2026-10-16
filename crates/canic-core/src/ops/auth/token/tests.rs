//! Tests for Root proof verification and configuration.

use super::*;
use crate::{
    config::schema::ChainKeyRootProofConfig,
    domain::auth::MAINNET_IC_ROOT_PUBLIC_KEY_RAW,
    ops::auth::delegated::{
        chain_key::{ChainKeySignatureVerificationInput, verify_chain_key_ecdsa_signature},
        root_proof::RootDelegationProofError,
    },
};
use k256::ecdsa::{
    Signature as K256TestSignature, SigningKey as K256SigningKey, signature::hazmat::PrehashSigner,
};
use std::fmt::Write as _;

fn root_pid() -> Principal {
    Principal::from_slice(&[1; 29])
}

fn cfg(build_network: BuildNetwork, root_key: Option<Vec<u8>>) -> DelegatedTokenConfig {
    let mut cfg = DelegatedTokenConfig {
        enabled: true,
        root_canister_id: Some(root_pid().to_string()),
        ic_root_public_key_raw_hex: root_key.map(hex),
        chain_key_root_proof: ChainKeyRootProofConfig::default(),
        build_network,
        max_ttl_secs: None,
    };
    install_chain_key_policy(&mut cfg, "key_1");
    cfg
}

fn chain_key_cfg(
    build_network: BuildNetwork,
    root_key: Vec<u8>,
    key_id: &str,
) -> DelegatedTokenConfig {
    let mut cfg = cfg(build_network, Some(root_key));
    install_chain_key_policy(&mut cfg, key_id);
    cfg
}

fn install_chain_key_policy(cfg: &mut DelegatedTokenConfig, key_id: &str) {
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

#[test]
fn chain_key_ecdsa_signature_verifier_accepts_valid_prehash_signature() {
    let signing_key = K256SigningKey::from_slice(&[7; 32]).expect("test signing key should parse");
    let public_key = signing_key.verifying_key().to_encoded_point(true);
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
    let public_key = signing_key.verifying_key().to_encoded_point(true);
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
fn chain_key_root_proof_failures_keep_boundary_specific_codes() {
    let cases = [
        (
            RootDelegationProofError::Verification(
                ic_auth::token::TokenVerificationError::Expired { target: "batch" },
            ),
            crate::diagnostics::codes::AUTH_CERT_EXPIRED.raw_code(),
        ),
        (
            RootDelegationProofError::Verification(
                ic_auth::token::TokenVerificationError::NotYetValid { target: "batch" },
            ),
            crate::diagnostics::codes::SECURITY_UNAVAILABLE.raw_code(),
        ),
        (
            RootDelegationProofError::Verification(
                ic_auth::token::TokenVerificationError::Expired {
                    target: "root_key_policy",
                },
            ),
            crate::diagnostics::codes::SECURITY_CONFLICT.raw_code(),
        ),
        (
            RootDelegationProofError::Verification(
                ic_auth::token::TokenVerificationError::StaleAuthority {
                    field: "proof_epoch",
                },
            ),
            crate::diagnostics::codes::SECURITY_CONFLICT.raw_code(),
        ),
        (
            RootDelegationProofError::Verification(
                ic_auth::token::TokenVerificationError::RootSignatureInvalid,
            ),
            crate::diagnostics::codes::REQUEST_INVALID.raw_code(),
        ),
        (
            RootDelegationProofError::Verification(
                ic_auth::token::TokenVerificationError::RootSignatureBindingMismatch {
                    field: "key_id",
                },
            ),
            crate::diagnostics::codes::REQUEST_INVALID.raw_code(),
        ),
        (
            RootDelegationProofError::Verification(
                ic_auth::token::TokenVerificationError::RootKeyPolicyBindingMismatch {
                    field: "key_id",
                },
            ),
            crate::diagnostics::codes::SECURITY_CONFLICT.raw_code(),
        ),
    ];

    for (err, expected) in cases {
        let mapped = map_root_delegation_proof_error(err);
        assert_eq!(mapped.public_error().code(), expected);
    }
}

#[test]
fn auth_proof_verifier_config_accepts_ic_with_known_mainnet_root_key() {
    let cfg = cfg(BuildNetwork::Ic, Some(mainnet_key()));

    let verifier = AuthOps::auth_proof_verifier_config_from(&cfg).expect("IC key should pass");

    assert_eq!(verifier.build_network, BuildNetwork::Ic);
    assert_eq!(verifier.root_canister_id, root_pid());
    assert_eq!(verifier.ic_root_public_key_raw, mainnet_key());
    assert!(verifier.chain_key_root.is_some());
}

#[test]
fn auth_proof_verifier_config_rejects_ic_without_root_key() {
    let cfg = cfg(BuildNetwork::Ic, None);

    AuthOps::auth_proof_verifier_config_from(&cfg).expect_err("IC requires explicit root key");
}

#[test]
fn auth_proof_verifier_config_rejects_ic_with_local_root_key() {
    let cfg = cfg(BuildNetwork::Ic, Some(local_key()));

    AuthOps::auth_proof_verifier_config_from(&cfg).expect_err("IC must reject local root key");
}

#[test]
fn auth_proof_verifier_config_local_requires_explicit_root_key() {
    let cfg = cfg(BuildNetwork::Local, None);

    AuthOps::auth_proof_verifier_config_from(&cfg)
        .expect_err("local verifier requires explicit root key");
}

#[test]
fn auth_proof_verifier_config_local_accepts_explicit_local_root_key() {
    let cfg = cfg(BuildNetwork::Local, Some(local_key()));

    let verifier = AuthOps::auth_proof_verifier_config_from(&cfg).expect("local key should pass");

    assert_eq!(verifier.build_network, BuildNetwork::Local);
    assert_eq!(verifier.ic_root_public_key_raw, local_key());
}

#[test]
fn auth_proof_verifier_config_local_rejects_explicit_mainnet_root_key() {
    let cfg = cfg(BuildNetwork::Local, Some(mainnet_key()));

    AuthOps::auth_proof_verifier_config_from(&cfg)
        .expect_err("local must reject explicit mainnet root key");
}

#[test]
fn auth_proof_verifier_config_chain_key_local_accepts_test_key_when_allowed() {
    let mut cfg = chain_key_cfg(BuildNetwork::Local, local_key(), "test_key_1");
    cfg.chain_key_root_proof.allow_test_key = true;

    let verifier =
        AuthOps::auth_proof_verifier_config_from(&cfg).expect("chain-key config should pass");
    let chain_key_root = verifier
        .chain_key_root
        .as_ref()
        .expect("chain-key policy should be configured");

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
    let mut cfg = chain_key_cfg(BuildNetwork::Local, local_key(), "test_key_1");
    cfg.chain_key_root_proof.allow_test_key = true;
    cfg.chain_key_root_proof.public_key_hex = Some("00".repeat(33));

    AuthOps::auth_proof_verifier_config_from(&cfg)
        .expect_err("invalid chain-key public key must reject");
}

#[test]
fn auth_proof_verifier_config_chain_key_rejects_ic_test_key() {
    let cfg = chain_key_cfg(BuildNetwork::Ic, mainnet_key(), "test_key_1");

    AuthOps::auth_proof_verifier_config_from(&cfg).expect_err("IC must reject test_key_1");
}

#[test]
fn auth_proof_verifier_config_chain_key_rejects_unapproved_local_test_key() {
    let cfg = chain_key_cfg(BuildNetwork::Local, local_key(), "test_key_1");

    AuthOps::auth_proof_verifier_config_from(&cfg)
        .expect_err("local test key requires explicit opt-in");
}
