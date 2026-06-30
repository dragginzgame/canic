//! Module: ops::ic::mgmt::signing
//!
//! Responsibility: expose management-canister threshold-signing calls.
//! Does not own: proof assembly, signing policy, or renewal orchestration.
//! Boundary: `MgmtOps` extension for chain-key signing calls.

use super::*;

impl MgmtOps {
    /// Fetch the ECDSA public key for one root canister derivation path.
    pub async fn ecdsa_public_key(
        args: &EcdsaPublicKeyArgs,
    ) -> Result<EcdsaPublicKeyResult, InternalError> {
        let infra_args = ecdsa_public_key_args_to_infra(args);
        let result = management_call(
            ManagementCallMetricOperation::EcdsaPublicKey,
            MgmtInfra::ecdsa_public_key(&infra_args),
        )
        .await?;

        Ok(ecdsa_public_key_from_infra(result))
    }

    /// Sign one 32-byte ECDSA message hash through the management canister.
    pub async fn sign_with_ecdsa(
        args: &SignWithEcdsaArgs,
    ) -> Result<SignWithEcdsaResult, InternalError> {
        let infra_args = sign_with_ecdsa_args_to_infra(args);
        let result = management_call(
            ManagementCallMetricOperation::SignWithEcdsa,
            MgmtInfra::sign_with_ecdsa(&infra_args),
        )
        .await?;

        Ok(sign_with_ecdsa_from_infra(result))
    }
}
