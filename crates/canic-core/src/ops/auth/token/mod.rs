//! Module: ops::auth::token
//!
//! Responsibility: resolve configured Root proof verification authority.
//! Does not own: endpoint authorization, active-proof storage, or auth policy records.
//! Boundary: auth ops facade for Root proof configuration and verification helpers.

#[cfg(test)]
mod verification;
mod verifier_config;

use super::{AuthChainKeyRootVerifierConfig, AuthOps, AuthProofVerifierConfig};
use crate::{
    InternalError,
    cdk::{types::Principal, utils::hash::decode_hex},
    config::schema::DelegatedTokenConfig,
    domain::auth::{IC_ROOT_PUBLIC_KEY_RAW_LENGTH, is_mainnet_ic_root_public_key_raw},
    dto::auth::{ChainKeyAlgorithm, ChainKeyKeyId, RootKeyPolicyV1},
    ops::{auth::AuthValidationError, config::ConfigOps},
};
use canic_contracts::ids::BuildNetwork;
#[cfg(test)]
use verification::map_root_delegation_proof_error;
use verifier_config::{
    configured_chain_key_root_verifier, configured_ic_root_public_key_raw,
    configured_root_canister_id,
};

impl AuthOps {
    /// Resolve verifier-local trust anchors for canister-signature auth proofs.
    pub(crate) fn auth_proof_verifier_config() -> Result<AuthProofVerifierConfig, InternalError> {
        let cfg = ConfigOps::delegated_tokens_config()?;
        Self::auth_proof_verifier_config_from(&cfg)
    }

    fn auth_proof_verifier_config_from(
        cfg: &DelegatedTokenConfig,
    ) -> Result<AuthProofVerifierConfig, InternalError> {
        let build_network = cfg.build_network;
        let root_canister_id = configured_root_canister_id(cfg)?;
        let chain_key_root =
            configured_chain_key_root_verifier(cfg, root_canister_id, build_network)?;
        Ok(AuthProofVerifierConfig {
            #[cfg(test)]
            build_network,
            root_canister_id,
            ic_root_public_key_raw: configured_ic_root_public_key_raw(cfg, build_network)?,
            chain_key_root,
        })
    }
}

#[cfg(test)]
mod tests;
