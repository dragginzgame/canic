use super::{DelegatedTokenOps, crypto, keys, verify};
use crate::{
    InternalError,
    cdk::types::Principal,
    dto::auth::{DelegatedToken, DelegatedTokenClaims, DelegationProof},
    ops::{
        auth::{
            DelegationExpiryError, DelegationScopeError, DelegationValidationError,
            VerifiedDelegatedToken, VerifiedTokenClaims,
        },
        config::ConfigOps,
        ic::{IcOps, ecdsa::EcdsaOps},
    },
};

impl DelegatedTokenOps {
    const MAX_TOKEN_CLOCK_SKEW_SECS: u64 = 0;

    // Reject structurally invalid or temporally unusable lifetimes before deeper verification.
    pub(super) const fn validate_claim_invariants(
        lifetime: super::TokenLifetime,
        now_secs: u64,
    ) -> Result<(), DelegationExpiryError> {
        if lifetime.exp < lifetime.iat {
            return Err(DelegationExpiryError::TokenExpiryBeforeIssued);
        }

        if lifetime.exp < now_secs {
            return Err(DelegationExpiryError::TokenExpired { exp: lifetime.exp });
        }

        if lifetime.iat > now_secs.saturating_add(Self::MAX_TOKEN_CLOCK_SKEW_SECS) {
            return Err(DelegationExpiryError::TokenNotYetValid { iat: lifetime.iat });
        }

        Ok(())
    }

    pub async fn sign_token(
        claims: DelegatedTokenClaims,
        proof: DelegationProof,
    ) -> Result<DelegatedToken, InternalError> {
        let verified_claims = VerifiedTokenClaims::from_dto_ref(&claims);
        let lifetime = verified_claims.lifetime();
        Self::validate_claim_invariants(lifetime, IcOps::now_secs())
            .map_err(InternalError::from)?;
        if let Some(max_ttl_secs) = ConfigOps::delegated_tokens_config()?.max_ttl_secs {
            verify::verify_max_ttl(lifetime, max_ttl_secs).map_err(InternalError::from)?;
        }
        verify::validate_claims_against_cert(verified_claims.grant(), &proof.cert)?;

        let local = IcOps::canister_self();
        if verified_claims.shard_pid() != local {
            return Err(DelegationScopeError::ShardPidMismatch {
                expected: local,
                found: verified_claims.shard_pid(),
            }
            .into());
        }

        let key_name = keys::delegated_tokens_key_name()?;
        keys::ensure_shard_public_key_cached(&key_name, verified_claims.shard_pid()).await?;
        let token_hash = crypto::token_signing_hash(&verified_claims, &proof.cert)?;
        let token_sig = EcdsaOps::sign_bytes(
            &key_name,
            keys::shard_derivation_path(verified_claims.shard_pid()),
            token_hash,
        )
        .await?;

        Ok(DelegatedToken {
            claims,
            proof,
            token_sig,
        })
    }

    pub fn verify_token(
        token: &DelegatedToken,
        authority_pid: Principal,
        now_secs: u64,
        self_pid: Principal,
    ) -> Result<VerifiedDelegatedToken, InternalError> {
        let cfg = ConfigOps::delegated_tokens_config()?;
        if !cfg.enabled {
            return Err(DelegationValidationError::DelegatedTokenAuthDisabled.into());
        }

        let claims = VerifiedTokenClaims::from_dto_ref(&token.claims);
        let lifetime = claims.lifetime();
        Self::validate_claim_invariants(lifetime, now_secs).map_err(InternalError::from)?;

        if let Some(max_ttl_secs) = cfg.max_ttl_secs {
            verify::verify_max_ttl(lifetime, max_ttl_secs).map_err(InternalError::from)?;
        }

        verify::verify_token_trust_chain(token, authority_pid, now_secs, self_pid)?;

        Ok(VerifiedDelegatedToken {
            claims,
            cert: token.proof.cert.clone(),
        })
    }

    // Verify a token for issuer-side reissue where the old audience may be stale.
    pub fn verify_token_for_reissue(
        token: &DelegatedToken,
        authority_pid: Principal,
        now_secs: u64,
    ) -> Result<VerifiedDelegatedToken, InternalError> {
        let cfg = ConfigOps::delegated_tokens_config()?;
        if !cfg.enabled {
            return Err(DelegationValidationError::DelegatedTokenAuthDisabled.into());
        }

        let claims = VerifiedTokenClaims::from_dto_ref(&token.claims);
        let lifetime = claims.lifetime();
        Self::validate_claim_invariants(lifetime, now_secs).map_err(InternalError::from)?;

        if let Some(max_ttl_secs) = cfg.max_ttl_secs {
            verify::verify_max_ttl(lifetime, max_ttl_secs).map_err(InternalError::from)?;
        }

        verify::verify_token_trust_chain_for_reissue(token, authority_pid, now_secs)?;

        Ok(VerifiedDelegatedToken {
            claims,
            cert: token.proof.cert.clone(),
        })
    }
}
