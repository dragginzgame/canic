use super::canonical::{CanonicalAuthError, cert_hash};
use crate::{
    cdk::types::Principal,
    dto::auth::{ActiveDelegationProof, DelegationProof, RootProof},
};
use thiserror::Error;

pub struct InstallActiveDelegationProofInput {
    pub proof: DelegationProof,
    pub installed_by: Principal,
    pub this_canister: Principal,
    pub now_ns: u64,
}

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
    V: FnMut([u8; 32], &RootProof, Principal) -> Result<(), String>,
{
    let cert = &input.proof.cert;
    if cert.shard_pid != input.this_canister {
        return Err(InstallActiveDelegationProofError::IssuerMismatch);
    }
    if input.now_ns < cert.not_before_ns {
        return Err(InstallActiveDelegationProofError::CertNotYetValid);
    }
    if input.now_ns >= cert.expires_at_ns {
        return Err(InstallActiveDelegationProofError::CertExpired);
    }

    let cert_hash = cert_hash(cert)?;
    verify_root_proof(cert_hash, &input.proof.root_proof, cert.root_pid)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        dto::auth::{
            DelegatedRoleGrant, DelegationAudience, DelegationCert, IcCanisterSignatureProofV1,
            RootProof, ShardKeyBinding, ShardSignatureAlgorithm,
        },
        ids::CanisterRole,
        ops::auth::delegated::canonical::shard_key_hash,
    };

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn cert() -> DelegationCert {
        let shard_sig_alg = ShardSignatureAlgorithm::IcThresholdEcdsaSecp256k1;
        let shard_public_key_sec1 = vec![1; 33];
        let shard_key_binding = ShardKeyBinding::IcThresholdEcdsaSecp256k1 {
            key_name_hash: [2; 32],
            derivation_path_hash: [3; 32],
        };

        DelegationCert {
            root_pid: p(1),
            shard_pid: p(2),
            shard_key_id: "issuer-key".to_string(),
            shard_sig_alg,
            shard_public_key_sec1: shard_public_key_sec1.clone(),
            shard_key_hash: shard_key_hash(
                shard_sig_alg,
                &shard_public_key_sec1,
                shard_key_binding,
            ),
            shard_key_binding,
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
            root_proof: RootProof::IcCanisterSignatureV1(IcCanisterSignatureProofV1 {
                signature_cbor: vec![8; 64],
                public_key_der: vec![9; 32],
            }),
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

        assert_eq!(active.proof.cert.shard_pid, p(2));
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
