//! Module: infra::ic::mgmt::signing
//!
//! Responsibility: perform raw threshold-signing management canister calls.
//! Does not own: signing policy, proof assembly, or retry orchestration.
//! Boundary: extends `MgmtInfra` with chain-key signing effects.

use super::{MgmtInfra, MgmtInfraError};
use crate::infra::ic::{IcInfraError, external_candid::decode_one};
use ic_call::{Call, WaitPolicy};
use ic_platform::candid::contract::encode_one;
use ic_platform::cdk::api;
use ic_platform::management_canister_types::{
    EcdsaPublicKeyArgs, EcdsaPublicKeyResult, SignWithEcdsaArgs, SignWithEcdsaResult,
};
use ic_platform::principal::ExternalPrincipal as Principal;

impl MgmtInfra {
    /// Fetch the caller-derived ECDSA public key for one root signing policy.
    pub async fn ecdsa_public_key(
        args: &EcdsaPublicKeyArgs,
    ) -> Result<EcdsaPublicKeyResult, IcInfraError> {
        let response = Call::new(
            Principal::management_canister().into(),
            "ecdsa_public_key",
            WaitPolicy::Bounded {
                timeout_seconds: 300,
            },
        )
        .take_raw_args(encode_one(args.clone())?)
        .execute()
        .await?;
        decode_one(&response.into_bytes())
    }

    /// Sign one 32-byte ECDSA message hash through the management canister.
    pub async fn sign_with_ecdsa(
        args: &SignWithEcdsaArgs,
    ) -> Result<SignWithEcdsaResult, IcInfraError> {
        let cycles = api::cost_sign_with_ecdsa(&args.key_id.name, args.key_id.curve.into())
            .map_err(MgmtInfraError::from)
            .map_err(IcInfraError::from)?;
        let response = Call::new(
            Principal::management_canister().into(),
            "sign_with_ecdsa",
            WaitPolicy::Unbounded,
        )
        .take_raw_args(encode_one(args.clone())?)
        .with_cycles(cycles)
        .execute()
        .await?;
        decode_one(&response.into_bytes())
    }
}
