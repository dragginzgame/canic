use super::{DelegatedTokenOps, crypto, keys, verify};
use crate::{
    InternalError,
    cdk::types::Principal,
    dto::auth::{DelegatedToken, DelegatedTokenClaims, DelegationProof},
    ops::{
        auth::{DelegationScopeError, DelegationValidationError, VerifiedDelegatedToken},
        config::ConfigOps,
        ic::{IcOps, ecdsa::EcdsaOps},
    },
};

impl DelegatedTokenOps {
    pub async fn sign_token(
        claims: DelegatedTokenClaims,
        proof: DelegationProof,
    ) -> Result<DelegatedToken, InternalError> {
        verify::validate_claims_against_cert(&claims, &proof.cert)?;

        let local = IcOps::canister_self();
        if claims.shard_pid != local {
            return Err(DelegationScopeError::ShardPidMismatch {
                expected: local,
                found: claims.shard_pid,
            }
            .into());
        }

        let key_name = keys::delegated_tokens_key_name()?;
        keys::ensure_shard_public_key_cached(&key_name, claims.shard_pid).await?;
        let token_hash = crypto::token_signing_hash(&claims, &proof.cert)?;
        let token_sig = EcdsaOps::sign_bytes(
            &key_name,
            keys::shard_derivation_path(claims.shard_pid),
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

        Self::verify_token_structure(token, authority_pid, now_secs, self_pid)?;
        if let Some(max_ttl_secs) = cfg.max_ttl_secs {
            verify::verify_max_ttl(token, max_ttl_secs)?;
        }

        verify::verify_current_proof(&token.proof)?;
        Self::verify_token_signature(token)?;

        Ok(VerifiedDelegatedToken {
            claims: token.claims.clone(),
            cert: token.proof.cert.clone(),
        })
    }

    fn verify_token_structure(
        token: &DelegatedToken,
        authority_pid: Principal,
        now_secs: u64,
        self_pid: Principal,
    ) -> Result<(), InternalError> {
        Self::verify_delegation_structure(&token.proof, Some(authority_pid))?;
        verify::verify_time_bounds(&token.claims, &token.proof.cert, now_secs)?;
        verify::validate_claims_against_cert(&token.claims, &token.proof.cert)?;
        verify::verify_self_audience(&token.claims, self_pid)?;

        Ok(())
    }

    fn verify_token_signature(token: &DelegatedToken) -> Result<(), InternalError> {
        Self::verify_delegation_signature(&token.proof)?;
        verify::verify_token_sig(token)?;
        Ok(())
    }
}
