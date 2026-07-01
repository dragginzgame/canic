//! Module: ops::auth::delegated::chain_key_signing
//!
//! Responsibility: sign chain-key batch headers through an injectable signer.
//! Does not own: renewal orchestration, issuer installation, or stable storage.
//! Boundary: auth-internal helper between root renewal workflow and management ops.

use super::{
    canonical::{chain_key_batch_header_hash, chain_key_derivation_path_hash},
    chain_key::{
        ChainKeySignatureVerificationInput, verify_chain_key_ecdsa_public_key_shape,
        verify_chain_key_ecdsa_signature, verify_chain_key_ecdsa_signature_shape,
    },
};
use crate::{
    InternalError,
    cdk::{types::Principal, utils::hash::decode_hex},
    config::schema::DelegatedTokenConfig,
    dto::auth::{ChainKeyAlgorithm, ChainKeyBatchHeaderV1, ChainKeyKeyId, ChainKeyRootSignatureV1},
    ids::BuildNetwork,
    ops::{
        auth::AuthValidationError,
        ic::mgmt::{
            EcdsaCurve, EcdsaKeyId, EcdsaPublicKeyArgs, EcdsaPublicKeyResult, MgmtOps,
            SignWithEcdsaArgs, SignWithEcdsaResult,
        },
    },
};
#[cfg(any(feature = "auth-chain-key-ecdsa", test))]
use k256::ecdsa::Signature as K256EcdsaSignature;
use std::{future::Future, pin::Pin};
use thiserror::Error;

const PRODUCTION_ECDSA_KEY_ID: &str = "key_1";
const TEST_ECDSA_KEY_ID: &str = "test_key_1";

///
/// ChainKeySigningPolicy
///
/// Root-local policy for producing one chain-key batch header signature.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::ops::auth) struct ChainKeySigningPolicy {
    pub root_canister_id: Principal,
    pub algorithm: ChainKeyAlgorithm,
    pub key_id: ChainKeyKeyId,
    pub derivation_path: Vec<Vec<u8>>,
    pub public_key: Vec<u8>,
    pub key_version: u64,
    pub build_network: BuildNetwork,
    pub allow_test_chain_key: bool,
}

///
/// SignChainKeyBatchHeaderInput
///
/// Input for producing a defensively verified root chain-key batch signature.
///

pub(in crate::ops::auth) struct SignChainKeyBatchHeaderInput<'a> {
    pub header: &'a ChainKeyBatchHeaderV1,
    pub policy: &'a ChainKeySigningPolicy,
}

///
/// ChainKeySigner
///
/// Injectable signing boundary for production management calls and focused tests.
///

pub(in crate::ops::auth) trait ChainKeySigner: Send {
    fn ecdsa_public_key(
        &mut self,
        args: EcdsaPublicKeyArgs,
    ) -> ChainKeySignerFuture<'_, EcdsaPublicKeyResult>;

    fn sign_with_ecdsa(
        &mut self,
        args: SignWithEcdsaArgs,
    ) -> ChainKeySignerFuture<'_, SignWithEcdsaResult>;
}

pub(in crate::ops::auth) type ChainKeySignerFuture<'a, T> =
    Pin<Box<dyn Future<Output = Result<T, ChainKeySignerError>> + Send + 'a>>;

///
/// ManagementCanisterChainKeySigner
///
/// Production signer backed by the IC management canister.
///

pub(in crate::ops::auth) struct ManagementCanisterChainKeySigner;

impl ChainKeySigner for ManagementCanisterChainKeySigner {
    fn ecdsa_public_key(
        &mut self,
        args: EcdsaPublicKeyArgs,
    ) -> ChainKeySignerFuture<'_, EcdsaPublicKeyResult> {
        Box::pin(async move {
            MgmtOps::ecdsa_public_key(&args)
                .await
                .map_err(ChainKeySignerError::Management)
        })
    }

    fn sign_with_ecdsa(
        &mut self,
        args: SignWithEcdsaArgs,
    ) -> ChainKeySignerFuture<'_, SignWithEcdsaResult> {
        Box::pin(async move {
            MgmtOps::sign_with_ecdsa(&args)
                .await
                .map_err(ChainKeySignerError::Management)
        })
    }
}

///
/// ChainKeySignerError
///
/// Failure surface for root chain-key batch signing.
///

#[derive(Debug, Error)]
pub(in crate::ops::auth) enum ChainKeySignerError {
    #[error("chain-key signer header/policy mismatch: {field}")]
    HeaderPolicyMismatch { field: &'static str },
    #[error("chain-key signer test key is rejected for this build network")]
    TestKeyRejected,
    #[error("chain-key signer derived public key does not match configured root key")]
    PublicKeyMismatch,
    #[error("chain-key signer returned an invalid signature: {0}")]
    SignatureVerification(String),
    #[error("chain-key management signer failed: {0}")]
    Management(#[source] InternalError),
}

pub(in crate::ops::auth) fn chain_key_signing_policy_from_config(
    config: &DelegatedTokenConfig,
    root_canister_id: Principal,
    build_network: BuildNetwork,
) -> Result<ChainKeySigningPolicy, InternalError> {
    if config.root_proof_mode.trim() != "chain_key_batch" {
        return Err(AuthValidationError::Auth(
            "auth.delegated_tokens.root_proof_mode must be chain_key_batch in 0.76".to_string(),
        )
        .into());
    }

    let chain_key = &config.chain_key_root_proof;
    let key_id = required_chain_key_field(chain_key.key_id.as_deref(), "key_id")?;
    let derivation_path =
        required_chain_key_derivation_path(chain_key.derivation_path_hex.as_deref())?;
    let derivation_path_hash = required_fixed_32_chain_key_hex(
        chain_key.derivation_path_hash_hex.as_deref(),
        "derivation_path_hash_hex",
    )?;
    let actual_derivation_path_hash = chain_key_derivation_path_hash(&derivation_path);
    if actual_derivation_path_hash != derivation_path_hash {
        return Err(AuthValidationError::Auth(
            "auth.delegated_tokens.chain_key_root_proof.derivation_path_hash_hex does not match derivation_path_hex"
                .to_string(),
        )
        .into());
    }
    let public_key_hex =
        required_chain_key_field(chain_key.public_key_hex.as_deref(), "public_key_hex")?;
    let public_key = decode_hex(public_key_hex).map_err(|err| {
        AuthValidationError::Auth(format!(
            "auth.delegated_tokens.chain_key_root_proof.public_key_hex is not valid hex: {err}"
        ))
    })?;
    verify_chain_key_ecdsa_public_key_shape(&public_key).map_err(|err| {
        AuthValidationError::Auth(format!(
            "auth.delegated_tokens.chain_key_root_proof.public_key_hex must be a secp256k1 SEC1 public key: {err}"
        ))
    })?;
    let key_version = required_chain_key_u64(chain_key.key_version, "key_version")?;

    Ok(ChainKeySigningPolicy {
        root_canister_id,
        algorithm: ChainKeyAlgorithm::EcdsaSecp256k1,
        key_id: ChainKeyKeyId {
            name: key_id.to_string(),
        },
        derivation_path,
        public_key,
        key_version,
        build_network,
        allow_test_chain_key: chain_key.allow_test_key,
    })
}

pub(in crate::ops::auth) async fn sign_chain_key_batch_header<S>(
    input: SignChainKeyBatchHeaderInput<'_>,
    signer: &mut S,
) -> Result<ChainKeyRootSignatureV1, ChainKeySignerError>
where
    S: ChainKeySigner,
{
    validate_signing_policy(input.header, input.policy)?;

    let key_id = EcdsaKeyId {
        curve: EcdsaCurve::Secp256k1,
        name: input.policy.key_id.name.clone(),
    };
    let derivation_path = input.policy.derivation_path.clone();
    let public_key = signer
        .ecdsa_public_key(EcdsaPublicKeyArgs {
            canister_id: Some(input.policy.root_canister_id),
            derivation_path: derivation_path.clone(),
            key_id: key_id.clone(),
        })
        .await?;

    if public_key.public_key != input.policy.public_key {
        return Err(ChainKeySignerError::PublicKeyMismatch);
    }

    let message_hash = chain_key_batch_header_hash(input.header);
    let signature = signer
        .sign_with_ecdsa(SignWithEcdsaArgs {
            message_hash,
            derivation_path: derivation_path.clone(),
            key_id,
        })
        .await?;

    let signature_bytes = normalize_chain_key_ecdsa_signature(&signature.signature)?;

    verify_chain_key_ecdsa_signature_shape(&signature_bytes)
        .map_err(|err| ChainKeySignerError::SignatureVerification(err.to_string()))?;
    verify_chain_key_ecdsa_signature(ChainKeySignatureVerificationInput {
        algorithm: input.policy.algorithm,
        key_id: &input.policy.key_id,
        derivation_path: &derivation_path,
        public_key: &public_key.public_key,
        message_hash,
        signature: &signature_bytes,
    })
    .map_err(ChainKeySignerError::SignatureVerification)?;

    Ok(ChainKeyRootSignatureV1 {
        algorithm: input.policy.algorithm,
        key_id: input.policy.key_id.clone(),
        derivation_path,
        public_key: public_key.public_key,
        signature: signature_bytes,
    })
}

fn normalize_chain_key_ecdsa_signature(signature: &[u8]) -> Result<Vec<u8>, ChainKeySignerError> {
    normalize_chain_key_ecdsa_signature_enabled(signature)
}

#[cfg(any(feature = "auth-chain-key-ecdsa", test))]
fn normalize_chain_key_ecdsa_signature_enabled(
    signature: &[u8],
) -> Result<Vec<u8>, ChainKeySignerError> {
    let parsed = K256EcdsaSignature::from_slice(signature).map_err(|err| {
        ChainKeySignerError::SignatureVerification(format!(
            "invalid chain-key ECDSA signature encoding: {err}"
        ))
    })?;
    Ok(parsed.normalize_s().unwrap_or(parsed).to_bytes().to_vec())
}

#[cfg(not(any(feature = "auth-chain-key-ecdsa", test)))]
fn normalize_chain_key_ecdsa_signature_enabled(
    _signature: &[u8],
) -> Result<Vec<u8>, ChainKeySignerError> {
    Err(ChainKeySignerError::SignatureVerification(
        "chain-key ECDSA signing support is not enabled; enable the `auth-chain-key-root-sign` feature"
            .to_string(),
    ))
}

fn validate_signing_policy(
    header: &ChainKeyBatchHeaderV1,
    policy: &ChainKeySigningPolicy,
) -> Result<(), ChainKeySignerError> {
    if header.root_canister_id != policy.root_canister_id {
        return Err(ChainKeySignerError::HeaderPolicyMismatch {
            field: "root_canister_id",
        });
    }
    if header.algorithm != policy.algorithm {
        return Err(ChainKeySignerError::HeaderPolicyMismatch { field: "algorithm" });
    }
    if header.key_id != policy.key_id {
        return Err(ChainKeySignerError::HeaderPolicyMismatch { field: "key_id" });
    }
    if header.derivation_path_hash != chain_key_derivation_path_hash(&policy.derivation_path) {
        return Err(ChainKeySignerError::HeaderPolicyMismatch {
            field: "derivation_path_hash",
        });
    }
    if header.key_version != policy.key_version {
        return Err(ChainKeySignerError::HeaderPolicyMismatch {
            field: "key_version",
        });
    }
    validate_signing_key_network(policy)
}

fn validate_signing_key_network(policy: &ChainKeySigningPolicy) -> Result<(), ChainKeySignerError> {
    if policy.build_network == BuildNetwork::Ic {
        if policy.key_id.name != PRODUCTION_ECDSA_KEY_ID {
            return Err(ChainKeySignerError::TestKeyRejected);
        }
        return Ok(());
    }

    if policy.key_id.name == TEST_ECDSA_KEY_ID && !policy.allow_test_chain_key {
        return Err(ChainKeySignerError::TestKeyRejected);
    }
    Ok(())
}

fn required_chain_key_field<'a>(
    value: Option<&'a str>,
    field: &'static str,
) -> Result<&'a str, InternalError> {
    let Some(value) = value else {
        return Err(AuthValidationError::Auth(format!(
            "auth.delegated_tokens.chain_key_root_proof.{field} is required when root_proof_mode=\"chain_key_batch\""
        ))
        .into());
    };
    let value = value.trim();
    if value.is_empty() {
        return Err(AuthValidationError::Auth(format!(
            "auth.delegated_tokens.chain_key_root_proof.{field} must not be empty"
        ))
        .into());
    }
    Ok(value)
}

fn required_chain_key_derivation_path(
    value: Option<&[String]>,
) -> Result<Vec<Vec<u8>>, InternalError> {
    let Some(path) = value else {
        return Err(AuthValidationError::Auth(
            "auth.delegated_tokens.chain_key_root_proof.derivation_path_hex is required when root_proof_mode=\"chain_key_batch\""
                .to_string(),
        )
        .into());
    };
    path.iter()
        .enumerate()
        .map(|(index, component)| {
            decode_hex(component.trim()).map_err(|err| {
                AuthValidationError::Auth(format!(
                    "auth.delegated_tokens.chain_key_root_proof.derivation_path_hex[{index}] is not valid hex: {err}"
                ))
                .into()
            })
        })
        .collect()
}

fn required_fixed_32_chain_key_hex(
    value: Option<&str>,
    field: &'static str,
) -> Result<[u8; 32], InternalError> {
    let value = required_chain_key_field(value, field)?;
    let decoded = decode_hex(value).map_err(|err| {
        AuthValidationError::Auth(format!(
            "auth.delegated_tokens.chain_key_root_proof.{field} is not valid hex: {err}"
        ))
    })?;
    decoded.try_into().map_err(|decoded: Vec<u8>| {
        AuthValidationError::Auth(format!(
            "auth.delegated_tokens.chain_key_root_proof.{field} must decode to 32 bytes, got {}",
            decoded.len()
        ))
        .into()
    })
}

fn required_chain_key_u64(value: Option<u64>, field: &'static str) -> Result<u64, InternalError> {
    value.ok_or_else(|| {
        AuthValidationError::Auth(format!(
            "auth.delegated_tokens.chain_key_root_proof.{field} is required when root_proof_mode=\"chain_key_batch\""
        ))
        .into()
    })
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::schema::DelegatedTokenConfig;
    use crate::dto::auth::ChainKeyBatchHeaderV1;
    use futures::executor::block_on;
    use k256::ecdsa::{
        Signature as K256TestSignature, SigningKey as K256SigningKey,
        signature::hazmat::PrehashSigner,
    };

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn derivation_path() -> Vec<Vec<u8>> {
        vec![b"canic".to_vec(), b"delegation".to_vec()]
    }

    fn signing_key() -> K256SigningKey {
        K256SigningKey::from_slice(&[7; 32]).expect("test signing key should parse")
    }

    fn high_s_signature(header: &ChainKeyBatchHeaderV1) -> Vec<u8> {
        let signature: K256TestSignature = signing_key()
            .sign_prehash(&chain_key_batch_header_hash(header))
            .expect("test prehash signature should sign");
        if signature.normalize_s().is_some() {
            return signature.to_bytes().to_vec();
        }

        let (r, s) = signature.split_scalars();
        K256TestSignature::from_scalars(r.to_bytes(), (-s).to_bytes())
            .expect("high-s counterpart should parse")
            .to_bytes()
            .to_vec()
    }

    fn hex(bytes: &[u8]) -> String {
        use std::fmt::Write as _;

        let mut out = String::with_capacity(bytes.len() * 2);
        for byte in bytes {
            write!(&mut out, "{byte:02x}").expect("hex write should not fail");
        }
        out
    }

    fn config() -> DelegatedTokenConfig {
        let signing_key = signing_key();
        let derivation_path = derivation_path();
        let derivation_path_hash = chain_key_derivation_path_hash(&derivation_path);

        let mut config = DelegatedTokenConfig {
            root_proof_mode: "chain_key_batch".to_string(),
            ..Default::default()
        };
        config.chain_key_root_proof.key_id = Some("test_key_1".to_string());
        config.chain_key_root_proof.derivation_path_hex = Some(vec![
            "63616e6963".to_string(),
            "64656c65676174696f6e".to_string(),
        ]);
        config.chain_key_root_proof.derivation_path_hash_hex = Some(hex(&derivation_path_hash));
        config.chain_key_root_proof.public_key_hex = Some(hex(signing_key
            .verifying_key()
            .to_encoded_point(true)
            .as_bytes()));
        config.chain_key_root_proof.key_version = Some(4);
        config.chain_key_root_proof.allow_test_key = true;
        config
    }

    fn policy() -> ChainKeySigningPolicy {
        let signing_key = signing_key();
        ChainKeySigningPolicy {
            root_canister_id: p(1),
            algorithm: ChainKeyAlgorithm::EcdsaSecp256k1,
            key_id: ChainKeyKeyId {
                name: "test_key_1".to_string(),
            },
            derivation_path: derivation_path(),
            public_key: signing_key
                .verifying_key()
                .to_encoded_point(true)
                .as_bytes()
                .to_vec(),
            key_version: 4,
            build_network: BuildNetwork::Local,
            allow_test_chain_key: true,
        }
    }

    fn header(policy: &ChainKeySigningPolicy) -> ChainKeyBatchHeaderV1 {
        ChainKeyBatchHeaderV1 {
            schema_version: 1,
            root_canister_id: policy.root_canister_id,
            batch_id: [1; 32],
            proof_epoch: 10,
            registry_epoch: 11,
            registry_hash: [2; 32],
            tree_root: [3; 32],
            not_before_ns: 100,
            expires_at_ns: 500,
            algorithm: policy.algorithm,
            key_id: policy.key_id.clone(),
            derivation_path_hash: chain_key_derivation_path_hash(&policy.derivation_path),
            key_version: policy.key_version,
        }
    }

    #[test]
    fn chain_key_signing_policy_decodes_derivation_path_config() {
        let config = config();

        let policy = chain_key_signing_policy_from_config(&config, p(1), BuildNetwork::Local)
            .expect("signing policy should decode");

        assert_eq!(policy.root_canister_id, p(1));
        assert_eq!(policy.key_id.name, "test_key_1");
        assert_eq!(policy.derivation_path, derivation_path());
        assert_eq!(policy.key_version, 4);
        assert_eq!(policy.build_network, BuildNetwork::Local);
        assert!(policy.allow_test_chain_key);
    }

    #[test]
    fn chain_key_signing_policy_rejects_derivation_path_hash_mismatch() {
        let mut config = config();
        config.chain_key_root_proof.derivation_path_hash_hex = Some("11".repeat(32));

        let err = chain_key_signing_policy_from_config(&config, p(1), BuildNetwork::Local)
            .expect_err("mismatched derivation path hash must reject");

        assert!(
            err.to_string()
                .contains("does not match derivation_path_hex"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn chain_key_signing_policy_rejects_invalid_public_key() {
        let mut config = config();
        config.chain_key_root_proof.public_key_hex = Some("00".repeat(33));

        let err = chain_key_signing_policy_from_config(&config, p(1), BuildNetwork::Local)
            .expect_err("invalid public key must reject before signing");

        assert!(
            err.to_string()
                .contains("must be a secp256k1 SEC1 public key"),
            "unexpected error: {err}"
        );
    }

    struct MockSigner {
        public_key: Vec<u8>,
        signature: Vec<u8>,
        public_key_calls: usize,
        sign_calls: usize,
        last_public_key_args: Option<EcdsaPublicKeyArgs>,
        last_sign_args: Option<SignWithEcdsaArgs>,
    }

    impl MockSigner {
        fn valid(policy: &ChainKeySigningPolicy, header: &ChainKeyBatchHeaderV1) -> Self {
            let signature: K256TestSignature = signing_key()
                .sign_prehash(&chain_key_batch_header_hash(header))
                .expect("test prehash signature should sign");
            Self {
                public_key: policy.public_key.clone(),
                signature: signature.to_bytes().to_vec(),
                public_key_calls: 0,
                sign_calls: 0,
                last_public_key_args: None,
                last_sign_args: None,
            }
        }
    }

    impl ChainKeySigner for MockSigner {
        fn ecdsa_public_key(
            &mut self,
            args: EcdsaPublicKeyArgs,
        ) -> ChainKeySignerFuture<'_, EcdsaPublicKeyResult> {
            self.public_key_calls += 1;
            self.last_public_key_args = Some(args);
            Box::pin(async move {
                Ok(EcdsaPublicKeyResult {
                    public_key: self.public_key.clone(),
                    chain_code: vec![9; 32],
                })
            })
        }

        fn sign_with_ecdsa(
            &mut self,
            args: SignWithEcdsaArgs,
        ) -> ChainKeySignerFuture<'_, SignWithEcdsaResult> {
            self.sign_calls += 1;
            self.last_sign_args = Some(args);
            Box::pin(async move {
                Ok(SignWithEcdsaResult {
                    signature: self.signature.clone(),
                })
            })
        }
    }

    #[test]
    fn chain_key_signer_rejects_mainnet_test_key_before_management_calls() {
        let mut policy = policy();
        policy.build_network = BuildNetwork::Ic;
        let header = header(&policy);
        let mut signer = MockSigner::valid(&policy, &header);

        let err = block_on(sign_chain_key_batch_header(
            SignChainKeyBatchHeaderInput {
                header: &header,
                policy: &policy,
            },
            &mut signer,
        ))
        .expect_err("mainnet test key must reject before signing");

        assert!(matches!(err, ChainKeySignerError::TestKeyRejected));
        assert_eq!(signer.public_key_calls, 0);
        assert_eq!(signer.sign_calls, 0);
    }

    #[test]
    fn chain_key_signer_queries_root_public_key_and_signs_once() {
        let policy = policy();
        let header = header(&policy);
        let mut signer = MockSigner::valid(&policy, &header);

        let signature = block_on(sign_chain_key_batch_header(
            SignChainKeyBatchHeaderInput {
                header: &header,
                policy: &policy,
            },
            &mut signer,
        ))
        .expect("valid mock signer should produce a signature");

        assert_eq!(signer.public_key_calls, 1);
        assert_eq!(signer.sign_calls, 1);
        assert_eq!(signature.algorithm, policy.algorithm);
        assert_eq!(signature.key_id, policy.key_id);
        assert_eq!(signature.derivation_path, policy.derivation_path);
        assert_eq!(signature.public_key, policy.public_key);
        let public_key_args = signer
            .last_public_key_args
            .expect("public-key args should be captured");
        assert_eq!(public_key_args.canister_id, Some(policy.root_canister_id));
        let sign_args = signer.last_sign_args.expect("sign args should be captured");
        assert_eq!(sign_args.message_hash, chain_key_batch_header_hash(&header));
    }

    #[test]
    fn chain_key_signer_rejects_unexpected_public_key_before_signing() {
        let policy = policy();
        let header = header(&policy);
        let mut signer = MockSigner::valid(&policy, &header);
        signer.public_key[0] ^= 1;

        let err = block_on(sign_chain_key_batch_header(
            SignChainKeyBatchHeaderInput {
                header: &header,
                policy: &policy,
            },
            &mut signer,
        ))
        .expect_err("public key mismatch must reject");

        assert!(matches!(err, ChainKeySignerError::PublicKeyMismatch));
        assert_eq!(signer.public_key_calls, 1);
        assert_eq!(signer.sign_calls, 0);
    }

    #[test]
    fn chain_key_signer_verifies_returned_signature() {
        let policy = policy();
        let header = header(&policy);
        let mut signer = MockSigner::valid(&policy, &header);
        signer.signature[0] ^= 1;

        let err = block_on(sign_chain_key_batch_header(
            SignChainKeyBatchHeaderInput {
                header: &header,
                policy: &policy,
            },
            &mut signer,
        ))
        .expect_err("altered signature must reject");

        assert!(matches!(err, ChainKeySignerError::SignatureVerification(_)));
        assert_eq!(signer.public_key_calls, 1);
        assert_eq!(signer.sign_calls, 1);
    }

    #[test]
    fn chain_key_signer_normalizes_high_s_returned_signature() {
        let policy = policy();
        let header = header(&policy);
        let mut signer = MockSigner::valid(&policy, &header);
        signer.signature = high_s_signature(&header);
        assert!(
            verify_chain_key_ecdsa_signature_shape(&signer.signature).is_err(),
            "test fixture must start in high-s form",
        );

        let signature = block_on(sign_chain_key_batch_header(
            SignChainKeyBatchHeaderInput {
                header: &header,
                policy: &policy,
            },
            &mut signer,
        ))
        .expect("high-s management signature should normalize before proof storage");

        verify_chain_key_ecdsa_signature_shape(&signature.signature)
            .expect("stored signature should be low-s");
        assert_eq!(signer.public_key_calls, 1);
        assert_eq!(signer.sign_calls, 1);
    }
}
