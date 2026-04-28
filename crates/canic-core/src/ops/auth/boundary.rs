use super::{DelegatedTokenOps, TokenGrant, TokenLifetime, VerifiedTokenClaims, audience};
use crate::{
    cdk::types::Principal,
    dto::auth::{DelegatedToken, DelegatedTokenClaims, DelegationAudience, DelegationProof},
    ids::CanisterRole,
    ops::{config::ConfigOps, storage::registry::subnet::SubnetRegistryOps},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BootstrapTokenAudienceSubset {
    Accepted,
    EmptyRoleAudience,
    OutsideProofAudience,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DelegatedSessionExpiryClamp {
    Accepted(u64),
    InvalidConfiguredMaxTtl,
    InvalidRequestedTtl,
    ExpiredToken,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DelegationVerifierTargetDerivationError {
    EmptyAudience,
    RoleNotConfigured(CanisterRole),
    RoleNotVerifier(CanisterRole),
}

impl DelegatedTokenOps {
    // Check whether a locally cached proof can safely sign the requested claims.
    pub(crate) fn proof_reusable_for_claims(
        proof: &DelegationProof,
        claims: &DelegatedTokenClaims,
        now_secs: u64,
    ) -> bool {
        let claims = VerifiedTokenClaims::from_dto_ref(claims);
        Self::proof_reusable_for_grant(proof, claims.grant(), claims.lifetime(), now_secs)
    }

    // Check whether a locally cached proof can safely sign one grant/lifetime pair.
    pub(crate) fn proof_reusable_for_grant(
        proof: &DelegationProof,
        grant: TokenGrant<'_>,
        lifetime: TokenLifetime,
        now_secs: u64,
    ) -> bool {
        if now_secs > proof.cert.expires_at {
            return false;
        }

        if grant.shard_pid != proof.cert.shard_pid {
            return false;
        }

        if lifetime.iat < proof.cert.issued_at || lifetime.exp > proof.cert.expires_at {
            return false;
        }

        audience::roles_subset(grant.aud, &proof.cert.aud)
            && audience::strings_subset(grant.scopes, &proof.cert.scopes)
    }

    // Check whether an externally supplied token audience stays within the proof audience.
    pub(crate) fn bootstrap_token_audience_subset(
        token: &DelegatedToken,
    ) -> BootstrapTokenAudienceSubset {
        if audience::has_empty_roles(&token.claims.aud) {
            return BootstrapTokenAudienceSubset::EmptyRoleAudience;
        }

        if audience::roles_subset(&token.claims.aud, &token.proof.cert.aud) {
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
    pub(crate) fn required_verifier_targets_from_audience(
        audience: &DelegationAudience,
        signer_pid: Principal,
        root_pid: Principal,
    ) -> Result<Vec<Principal>, DelegationVerifierTargetDerivationError> {
        let mut verifier_targets = Vec::new();
        match audience {
            DelegationAudience::Any => {
                for (role, pids) in SubnetRegistryOps::role_index() {
                    let cfg = ConfigOps::current_subnet_canister(&role).map_err(|_| {
                        DelegationVerifierTargetDerivationError::RoleNotConfigured(role.clone())
                    })?;
                    if !cfg.delegated_auth.verifier {
                        continue;
                    }

                    append_target_pids(&mut verifier_targets, pids, signer_pid, root_pid);
                }
            }
            DelegationAudience::Roles(roles) if roles.is_empty() => {
                return Err(DelegationVerifierTargetDerivationError::EmptyAudience);
            }
            DelegationAudience::Roles(roles) => {
                for role in roles {
                    let cfg = ConfigOps::current_subnet_canister(role).map_err(|_| {
                        DelegationVerifierTargetDerivationError::RoleNotConfigured(role.clone())
                    })?;
                    if !cfg.delegated_auth.verifier {
                        return Err(DelegationVerifierTargetDerivationError::RoleNotVerifier(
                            role.clone(),
                        ));
                    }

                    let pids = SubnetRegistryOps::role_index()
                        .remove(role)
                        .unwrap_or_default();
                    append_target_pids(&mut verifier_targets, pids, signer_pid, root_pid);
                }
            }
        }

        Ok(verifier_targets)
    }
}

// Append verifier targets while preserving deterministic first-seen order.
fn append_target_pids(
    verifier_targets: &mut Vec<Principal>,
    pids: Vec<Principal>,
    signer_pid: Principal,
    root_pid: Principal,
) {
    for pid in pids {
        if pid == signer_pid || pid == root_pid {
            continue;
        }

        if !verifier_targets.contains(&pid) {
            verifier_targets.push(pid);
        }
    }
}
