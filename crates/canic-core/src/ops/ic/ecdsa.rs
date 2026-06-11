#[cfg(feature = "auth-threshold-ecdsa-sign")]
use crate::cdk::mgmt::{EcdsaCurve, EcdsaKeyId, SignWithEcdsaArgs, sign_with_ecdsa};
use crate::{
    InternalError,
    ops::{
        cost_guard::CostGuardPermit,
        runtime::metrics::platform_call::{
            PlatformCallMetricMode, PlatformCallMetricOutcome, PlatformCallMetricReason,
            PlatformCallMetricSurface, PlatformCallMetrics,
        },
    },
};
use thiserror::Error as ThisError;

///
/// EcdsaOpsError
///

#[derive(Debug, ThisError)]
pub enum EcdsaOpsError {
    #[error("threshold ecdsa support is not enabled in this canister build")]
    ThresholdEcdsaUnavailable,

    #[error("ecdsa sign call failed: {0}")]
    SignCall(String),
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
            PlatformCallMetricReason::Unavailable,
        );
        Err(EcdsaOpsError::ThresholdEcdsaUnavailable.into())
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
