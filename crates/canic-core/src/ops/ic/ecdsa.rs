#[cfg(feature = "auth-threshold-ecdsa-public-key")]
use crate::cdk::mgmt::{EcdsaCurve, EcdsaKeyId, EcdsaPublicKeyArgs, ecdsa_public_key};
#[cfg(feature = "auth-threshold-ecdsa-sign")]
use crate::cdk::mgmt::{SignWithEcdsaArgs, sign_with_ecdsa};
use crate::{
    InternalError,
    cdk::types::Principal,
    ops::{
        cost_guard::CostGuardPermit,
        runtime::metrics::platform_call::{
            PlatformCallMetricMode, PlatformCallMetricOutcome, PlatformCallMetricReason,
            PlatformCallMetricSurface, PlatformCallMetrics,
        },
    },
};
#[cfg(feature = "auth-shard-secp256k1-verify")]
use k256::ecdsa::{Signature, VerifyingKey, signature::hazmat::PrehashVerifier};
use thiserror::Error as ThisError;

///
/// EcdsaOpsError
///

#[derive(Debug, ThisError)]
pub enum EcdsaOpsError {
    #[error("threshold ecdsa support is not enabled in this canister build")]
    ThresholdEcdsaUnavailable,

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

#[cfg(feature = "auth-threshold-ecdsa-sign")]
impl EcdsaOps {
    // Sign a pre-hashed payload using the configured threshold ECDSA key.
    #[allow(dead_code)]
    pub async fn sign_bytes(
        _permit: &CostGuardPermit,
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

        record_ecdsa_call(
            PlatformCallMetricMode::Update,
            PlatformCallMetricOutcome::Started,
            PlatformCallMetricReason::Ok,
        );
        let response = match sign_with_ecdsa(&args).await {
            Ok(response) => response,
            Err(err) => {
                record_ecdsa_call(
                    PlatformCallMetricMode::Update,
                    PlatformCallMetricOutcome::Failed,
                    PlatformCallMetricReason::Infra,
                );
                return Err(EcdsaOpsError::SignCall(err.to_string()).into());
            }
        };
        record_ecdsa_call(
            PlatformCallMetricMode::Update,
            PlatformCallMetricOutcome::Completed,
            PlatformCallMetricReason::Ok,
        );

        Ok(response.signature)
    }
}

#[cfg(not(feature = "auth-threshold-ecdsa-sign"))]
impl EcdsaOps {
    // Fail closed when threshold ECDSA signing support is not compiled in.
    #[allow(dead_code)]
    #[expect(clippy::unused_async)]
    pub async fn sign_bytes(
        _permit: &CostGuardPermit,
        _key_name: &str,
        _derivation_path: Vec<Vec<u8>>,
        _msg_hash: [u8; 32],
    ) -> Result<Vec<u8>, InternalError> {
        record_ecdsa_call(
            PlatformCallMetricMode::Update,
            PlatformCallMetricOutcome::Failed,
            threshold_sign_availability_reason(),
        );
        Err(EcdsaOpsError::ThresholdEcdsaUnavailable.into())
    }
}

#[cfg(feature = "auth-threshold-ecdsa-public-key")]
impl EcdsaOps {
    // Fetch a SEC1-encoded threshold ECDSA public key for the requested path.
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

        record_ecdsa_call(
            PlatformCallMetricMode::Query,
            PlatformCallMetricOutcome::Started,
            PlatformCallMetricReason::Ok,
        );
        let response = match ecdsa_public_key(&args).await {
            Ok(response) => response,
            Err(err) => {
                record_ecdsa_call(
                    PlatformCallMetricMode::Query,
                    PlatformCallMetricOutcome::Failed,
                    PlatformCallMetricReason::Infra,
                );
                return Err(EcdsaOpsError::PublicKeyCall(err.to_string()).into());
            }
        };
        record_ecdsa_call(
            PlatformCallMetricMode::Query,
            PlatformCallMetricOutcome::Completed,
            PlatformCallMetricReason::Ok,
        );

        Ok(response.public_key)
    }
}

#[cfg(not(feature = "auth-threshold-ecdsa-public-key"))]
impl EcdsaOps {
    // Fail closed when threshold ECDSA public-key fetch support is not compiled in.
    #[expect(clippy::unused_async)]
    pub async fn public_key_sec1(
        _key_name: &str,
        _derivation_path: Vec<Vec<u8>>,
        _canister_id: Principal,
    ) -> Result<Vec<u8>, InternalError> {
        record_ecdsa_call(
            PlatformCallMetricMode::Query,
            PlatformCallMetricOutcome::Failed,
            threshold_public_key_availability_reason(),
        );
        Err(EcdsaOpsError::ThresholdEcdsaUnavailable.into())
    }
}

impl EcdsaOps {
    // Report whether threshold ECDSA public-key fetch support is compiled in.
    #[must_use]
    pub const fn threshold_public_key_fetch_enabled() -> bool {
        matches!(
            threshold_public_key_availability_reason(),
            PlatformCallMetricReason::Ok
        )
    }

    // Report whether threshold ECDSA signing support is compiled in.
    #[must_use]
    pub const fn threshold_sign_enabled() -> bool {
        matches!(
            threshold_sign_availability_reason(),
            PlatformCallMetricReason::Ok
        )
    }

    // Verify a pre-hashed signature locally with k256 on every canister build.
    #[cfg(feature = "auth-shard-secp256k1-verify")]
    pub fn verify_signature(
        public_key_sec1: &[u8],
        msg_hash: [u8; 32],
        signature_bytes: &[u8],
    ) -> Result<(), InternalError> {
        record_ecdsa_call(
            PlatformCallMetricMode::LocalVerify,
            PlatformCallMetricOutcome::Started,
            PlatformCallMetricReason::Ok,
        );
        let verifying_key = match VerifyingKey::from_sec1_bytes(public_key_sec1) {
            Ok(key) => key,
            Err(err) => {
                record_ecdsa_call(
                    PlatformCallMetricMode::LocalVerify,
                    PlatformCallMetricOutcome::Failed,
                    PlatformCallMetricReason::InvalidPublicKey,
                );
                return Err(EcdsaOpsError::InvalidPublicKey(err.to_string()).into());
            }
        };
        let signature = match Signature::try_from(signature_bytes) {
            Ok(signature) => signature,
            Err(err) => {
                record_ecdsa_call(
                    PlatformCallMetricMode::LocalVerify,
                    PlatformCallMetricOutcome::Failed,
                    PlatformCallMetricReason::InvalidSignature,
                );
                return Err(EcdsaOpsError::InvalidSignature(err.to_string()).into());
            }
        };

        if let Err(err) = verifying_key.verify_prehash(&msg_hash, &signature) {
            record_ecdsa_call(
                PlatformCallMetricMode::LocalVerify,
                PlatformCallMetricOutcome::Failed,
                PlatformCallMetricReason::InvalidSignature,
            );
            return Err(EcdsaOpsError::InvalidSignature(err.to_string()).into());
        }

        record_ecdsa_call(
            PlatformCallMetricMode::LocalVerify,
            PlatformCallMetricOutcome::Completed,
            PlatformCallMetricReason::Ok,
        );
        Ok(())
    }

    #[cfg(not(feature = "auth-shard-secp256k1-verify"))]
    pub fn verify_signature(
        _public_key_sec1: &[u8],
        _msg_hash: [u8; 32],
        _signature_bytes: &[u8],
    ) -> Result<(), InternalError> {
        record_ecdsa_call(
            PlatformCallMetricMode::LocalVerify,
            PlatformCallMetricOutcome::Failed,
            PlatformCallMetricReason::Unavailable,
        );
        Err(EcdsaOpsError::ThresholdEcdsaUnavailable.into())
    }
}

// Return the metric reason for compiled threshold ECDSA public-key availability.
const fn threshold_public_key_availability_reason() -> PlatformCallMetricReason {
    if cfg!(feature = "auth-threshold-ecdsa-public-key") {
        PlatformCallMetricReason::Ok
    } else {
        PlatformCallMetricReason::Unavailable
    }
}

// Return the metric reason for compiled threshold ECDSA signing availability.
const fn threshold_sign_availability_reason() -> PlatformCallMetricReason {
    if cfg!(feature = "auth-threshold-ecdsa-sign") {
        PlatformCallMetricReason::Ok
    } else {
        PlatformCallMetricReason::Unavailable
    }
}

// Record one ECDSA metric with no key name or derivation path labels.
fn record_ecdsa_call(
    mode: PlatformCallMetricMode,
    outcome: PlatformCallMetricOutcome,
    reason: PlatformCallMetricReason,
) {
    PlatformCallMetrics::record(PlatformCallMetricSurface::Ecdsa, mode, outcome, reason);
}

#[cfg(all(test, feature = "auth-shard-secp256k1-verify"))]
mod tests {
    use super::EcdsaOps;
    use k256::ecdsa::{SigningKey, signature::hazmat::PrehashSigner};

    #[test]
    fn verify_signature_accepts_valid_prehash_without_signing_feature() {
        let hash = [7u8; 32];
        let signing_key = SigningKey::from_bytes((&[9u8; 32]).into()).expect("signing key");
        let signature: k256::ecdsa::Signature =
            signing_key.sign_prehash(&hash).expect("prehash signature");
        let public_key = signing_key
            .verifying_key()
            .to_encoded_point(true)
            .as_bytes()
            .to_vec();

        EcdsaOps::verify_signature(&public_key, hash, &signature.to_bytes())
            .expect("local k256 verification must work in default builds");
    }
}
