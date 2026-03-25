use super::{DelegatedTokenOps, VerifiedTokenClaims, audience};
use crate::{
    cdk::types::Principal,
    dto::auth::{DelegatedToken, DelegatedTokenClaims, DelegationProof},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BootstrapTokenAudienceSubset {
    Accepted,
    EmptyClaimsAudience,
    OutsideProofAudience,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DelegatedSessionExpiryClamp {
    Accepted(u64),
    InvalidConfiguredMaxTtl,
    InvalidRequestedTtl,
    ExpiredToken,
}

impl DelegatedTokenOps {
    // Check whether a locally cached proof can safely sign the requested claims.
    pub(crate) fn proof_reusable_for_claims(
        proof: &DelegationProof,
        claims: &DelegatedTokenClaims,
        now_secs: u64,
    ) -> bool {
        let claims = VerifiedTokenClaims::from_dto_ref(claims);
        if now_secs > proof.cert.expires_at {
            return false;
        }

        if claims.shard_pid() != proof.cert.shard_pid {
            return false;
        }

        let lifetime = claims.lifetime();
        if lifetime.iat < proof.cert.issued_at || lifetime.exp > proof.cert.expires_at {
            return false;
        }

        let grant = claims.grant();
        audience::principals_subset(grant.aud, &proof.cert.aud)
            && audience::strings_subset(grant.scopes, &proof.cert.scopes)
    }

    // Check whether an externally supplied token audience stays within the proof audience.
    pub(crate) fn bootstrap_token_audience_subset(
        token: &DelegatedToken,
    ) -> BootstrapTokenAudienceSubset {
        if token.claims.aud.is_empty() {
            return BootstrapTokenAudienceSubset::EmptyClaimsAudience;
        }

        if audience::principals_subset(&token.claims.aud, &token.proof.cert.aud) {
            BootstrapTokenAudienceSubset::Accepted
        } else {
            BootstrapTokenAudienceSubset::OutsideProofAudience
        }
    }

    // Clamp delegated-session expiry against token, config, and requested TTL bounds.
    pub(crate) fn clamp_delegated_session_expires_at(
        now_secs: u64,
        token_expires_at: u64,
        configured_max_ttl_secs: u64,
        requested_ttl_secs: Option<u64>,
    ) -> DelegatedSessionExpiryClamp {
        if configured_max_ttl_secs == 0 {
            return DelegatedSessionExpiryClamp::InvalidConfiguredMaxTtl;
        }

        if let Some(ttl_secs) = requested_ttl_secs
            && ttl_secs == 0
        {
            return DelegatedSessionExpiryClamp::InvalidRequestedTtl;
        }

        let mut expires_at = token_expires_at;
        expires_at = expires_at.min(now_secs.saturating_add(configured_max_ttl_secs));
        if let Some(ttl_secs) = requested_ttl_secs {
            expires_at = expires_at.min(now_secs.saturating_add(ttl_secs));
        }

        if expires_at <= now_secs {
            DelegatedSessionExpiryClamp::ExpiredToken
        } else {
            DelegatedSessionExpiryClamp::Accepted(expires_at)
        }
    }

    // Derive canonical verifier fanout targets from token audience while rejecting invalid entries.
    pub(crate) fn required_verifier_targets_from_audience<F>(
        audience: &[Principal],
        signer_pid: Principal,
        root_pid: Principal,
        mut is_valid_target: F,
    ) -> Result<Vec<Principal>, Principal>
    where
        F: FnMut(Principal) -> bool,
    {
        let mut verifier_targets = Vec::new();
        for principal in audience {
            if *principal == signer_pid || *principal == root_pid {
                continue;
            }

            if !is_valid_target(*principal) {
                return Err(*principal);
            }

            if !verifier_targets.contains(principal) {
                verifier_targets.push(*principal);
            }
        }

        Ok(verifier_targets)
    }
}
