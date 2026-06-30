//! Module: infra::ic::mgmt::signing
//!
//! Responsibility: perform raw threshold-signing management canister calls.
//! Does not own: signing policy, proof assembly, or retry orchestration.
//! Boundary: extends `MgmtInfra` with chain-key signing effects.

use crate::{
    cdk::{api, candid::Principal},
    infra::{InfraError, ic::IcInfraError, ic::call::Call},
};

use super::{
    MgmtInfra, MgmtInfraError,
    types::{
        InfraEcdsaPublicKeyArgs, InfraEcdsaPublicKeyResult, InfraSignWithEcdsaArgs,
        InfraSignWithEcdsaResult,
    },
};

impl MgmtInfra {
    /// Fetch the caller-derived ECDSA public key for one root signing policy.
    pub async fn ecdsa_public_key(
        args: &InfraEcdsaPublicKeyArgs,
    ) -> Result<InfraEcdsaPublicKeyResult, InfraError> {
        let response = Call::bounded_wait(Principal::management_canister(), "ecdsa_public_key")
            .with_arg(args.clone())?
            .execute()
            .await?;
        response.candid()
    }

    /// Sign one 32-byte ECDSA message hash through the management canister.
    pub async fn sign_with_ecdsa(
        args: &InfraSignWithEcdsaArgs,
    ) -> Result<InfraSignWithEcdsaResult, InfraError> {
        let cycles = api::cost_sign_with_ecdsa(&args.key_id.name, args.key_id.curve.into())
            .map_err(MgmtInfraError::from)
            .map_err(IcInfraError::from)?;
        let response = Call::unbounded_wait(Principal::management_canister(), "sign_with_ecdsa")
            .with_arg(args.clone())?
            .with_cycles(cycles)
            .execute()
            .await?;
        response.candid()
    }
}
