use crate::{
    InternalError,
    cdk::{
        mgmt::{
            EcdsaCurve, EcdsaKeyId, EcdsaPublicKeyArgs, SignWithEcdsaArgs, ecdsa_public_key,
            sign_with_ecdsa,
        },
        types::Principal,
    },
};
use k256::ecdsa::{Signature, VerifyingKey, signature::hazmat::PrehashVerifier};
use thiserror::Error as ThisError;

///
/// EcdsaOpsError
///

#[derive(Debug, ThisError)]
pub enum EcdsaOpsError {
    #[error("ecdsa public key call failed: {0}")]
    PublicKeyCall(String),

    #[error("ecdsa sign call failed: {0}")]
    SignCall(String),

    #[error("invalid ecdsa public key: {0}")]
    InvalidPublicKey(String),

    #[error("invalid ecdsa signature: {0}")]
    InvalidSignature(String),
}

impl From<EcdsaOpsError> for InternalError {
    fn from(err: EcdsaOpsError) -> Self {
        crate::ops::ic::IcOpsError::from(err).into()
    }
}

///
/// EcdsaOps
///

pub struct EcdsaOps;

impl EcdsaOps {
    pub async fn sign_bytes(
        key_name: &str,
        derivation_path: Vec<Vec<u8>>,
        msg_hash: [u8; 32],
    ) -> Result<Vec<u8>, InternalError> {
        let args = SignWithEcdsaArgs {
            message_hash: msg_hash.to_vec(),
            derivation_path,
            key_id: EcdsaKeyId {
                curve: EcdsaCurve::Secp256k1,
                name: key_name.to_string(),
            },
        };

        let response = sign_with_ecdsa(&args)
            .await
            .map_err(|err| EcdsaOpsError::SignCall(err.to_string()))?;

        Ok(response.signature)
    }

    pub async fn public_key_sec1(
        key_name: &str,
        derivation_path: Vec<Vec<u8>>,
        canister_id: Principal,
    ) -> Result<Vec<u8>, InternalError> {
        let args = EcdsaPublicKeyArgs {
            canister_id: Some(canister_id),
            derivation_path,
            key_id: EcdsaKeyId {
                curve: EcdsaCurve::Secp256k1,
                name: key_name.to_string(),
            },
        };

        let response = ecdsa_public_key(&args)
            .await
            .map_err(|err| EcdsaOpsError::PublicKeyCall(err.to_string()))?;

        Ok(response.public_key)
    }

    pub fn verify_signature(
        public_key_sec1: &[u8],
        msg_hash: [u8; 32],
        signature_bytes: &[u8],
    ) -> Result<(), InternalError> {
        let verifying_key = VerifyingKey::from_sec1_bytes(public_key_sec1)
            .map_err(|err| EcdsaOpsError::InvalidPublicKey(err.to_string()))?;
        let signature = Signature::try_from(signature_bytes)
            .map_err(|err| EcdsaOpsError::InvalidSignature(err.to_string()))?;

        verifying_key
            .verify_prehash(&msg_hash, &signature)
            .map_err(|err| EcdsaOpsError::InvalidSignature(err.to_string()))?;

        Ok(())
    }
}
