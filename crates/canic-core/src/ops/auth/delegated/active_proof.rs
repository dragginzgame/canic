//! Module: ops::auth::delegated::active_proof
//!
//! Responsibility: validate and materialize issuer-local active delegation proof state.
//! Does not own: active proof storage, root proof construction, or endpoint guards.
//! Boundary: pure installation helper called before auth storage mutation.

use super::canonical::{CanonicalAuthError, cert_hash};
use crate::{
    cdk::types::Principal,
    dto::auth::{ActiveDelegationProof, DelegationCert, DelegationProof, RootProof},
};
use thiserror::Error;

///
/// InstallActiveDelegationProofInput
///
/// Input for validating and materializing one active delegation proof.
///

pub struct InstallActiveDelegationProofInput {
    pub proof: DelegationProof,
    pub installed_by: Principal,
    pub this_canister: Principal,
    pub now_ns: u64,
}

///
/// InstallActiveDelegationProofError
///
/// Typed failure surface for active delegation proof installation.
///

#[derive(Debug, Eq, Error, PartialEq)]
pub enum InstallActiveDelegationProofError {
    #[error("active delegation proof is for another issuer")]
    IssuerMismatch,
    #[error("active delegation proof cert is not yet valid")]
    CertNotYetValid,
    #[error("active delegation proof cert expired")]
    CertExpired,
    #[error("active delegation proof root proof invalid: {0}")]
    RootProofInvalid(String),
    #[error(transparent)]
    Canonical(#[from] CanonicalAuthError),
}

pub fn install_active_delegation_proof<V>(
    input: InstallActiveDelegationProofInput,
    mut verify_root_proof: V,
) -> Result<ActiveDelegationProof, InstallActiveDelegationProofError>
where
    V: FnMut(&DelegationCert, [u8; 32], &RootProof) -> Result<(), String>,
{
    let cert = &input.proof.cert;
    if cert.issuer_pid != input.this_canister {
        return Err(InstallActiveDelegationProofError::IssuerMismatch);
    }
    if input.now_ns < cert.not_before_ns {
        return Err(InstallActiveDelegationProofError::CertNotYetValid);
    }
    if input.now_ns >= cert.expires_at_ns {
        return Err(InstallActiveDelegationProofError::CertExpired);
    }

    let cert_hash = cert_hash(cert)?;
    verify_root_proof(cert, cert_hash, &input.proof.root_proof)
        .map_err(InstallActiveDelegationProofError::RootProofInvalid)?;

    let not_before_ns = cert.not_before_ns;
    let expires_at_ns = cert.expires_at_ns;
    let refresh_after_ns = refresh_after_ns(input.now_ns, expires_at_ns);

    Ok(ActiveDelegationProof {
        proof: input.proof,
        cert_hash,
        not_before_ns,
        expires_at_ns,
        refresh_after_ns,
        installed_at_ns: input.now_ns,
        installed_by: input.installed_by,
    })
}

const fn refresh_after_ns(now_ns: u64, expires_at_ns: u64) -> u64 {
    let remaining_ns = expires_at_ns.saturating_sub(now_ns);
    now_ns + remaining_ns.saturating_sub(remaining_ns / 5)
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        dto::auth::{
            DelegatedRoleGrant, DelegationAudience, DelegationCert, IssuerProofAlgorithm,
            IssuerProofBinding,
        },
        ids::CanisterRole,
        ops::auth::delegated::canonical::issuer_proof_binding_hash,
    };

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn cert() -> DelegationCert {
        let issuer_proof_alg = IssuerProofAlgorithm::IcCanisterSignatureV1;
        let issuer_proof_binding = IssuerProofBinding::IcCanisterSignatureV1 { seed_hash: [2; 32] };
        let issuer_proof_binding_hash =
            issuer_proof_binding_hash(p(2), issuer_proof_alg, issuer_proof_binding);

        DelegationCert {
            root_pid: p(1),
            issuer_pid: p(2),
            issuer_proof_alg,
            issuer_proof_binding_hash,
            issuer_proof_binding,
            issued_at_ns: 10,
            not_before_ns: 20,
            expires_at_ns: 120,
            max_token_ttl_ns: 30,
            aud: DelegationAudience::CanicSubnet(p(7)),
            grants: vec![DelegatedRoleGrant {
                target: CanisterRole::owned("project_instance".to_string()),
                scopes: vec!["read".to_string()],
            }],
        }
    }

    fn proof() -> DelegationProof {
        DelegationProof {
            cert: cert(),
            root_proof: crate::ops::auth::test_fixtures::chain_key_root_proof(8),
        }
    }

    fn input(proof: DelegationProof) -> InstallActiveDelegationProofInput {
        InstallActiveDelegationProofInput {
            proof,
            installed_by: p(10),
            this_canister: p(2),
            now_ns: 20,
        }
    }

    #[test]
    fn install_active_delegation_proof_builds_active_state_after_root_verify() {
        let active = install_active_delegation_proof(input(proof()), |_, _, _| Ok(())).unwrap();

        assert_eq!(active.proof.cert.issuer_pid, p(2));
        assert_eq!(active.not_before_ns, 20);
        assert_eq!(active.expires_at_ns, 120);
        assert_eq!(active.refresh_after_ns, 100);
        assert_eq!(active.installed_at_ns, 20);
        assert_eq!(active.installed_by, p(10));
    }

    #[test]
    fn install_active_delegation_proof_rejects_wrong_issuer() {
        let mut input = input(proof());
        input.this_canister = p(9);

        assert_eq!(
            install_active_delegation_proof(input, |_, _, _| Ok(())),
            Err(InstallActiveDelegationProofError::IssuerMismatch)
        );
    }

    #[test]
    fn install_active_delegation_proof_rejects_time_bounds() {
        let mut early = input(proof());
        early.now_ns = 19;
        assert_eq!(
            install_active_delegation_proof(early, |_, _, _| Ok(())),
            Err(InstallActiveDelegationProofError::CertNotYetValid)
        );

        let mut expired = input(proof());
        expired.now_ns = 120;
        assert_eq!(
            install_active_delegation_proof(expired, |_, _, _| Ok(())),
            Err(InstallActiveDelegationProofError::CertExpired)
        );
    }

    #[test]
    fn install_active_delegation_proof_rejects_root_proof_failure() {
        assert_eq!(
            install_active_delegation_proof(input(proof()), |_, _, _| Err("bad proof".to_string())),
            Err(InstallActiveDelegationProofError::RootProofInvalid(
                "bad proof".to_string()
            ))
        );
    }
}
