use crate::{
    cdk::types::Principal,
    dto::auth::{DelegatedToken, DelegatedTokenClaims, DelegationCert, DelegationProof},
    dto::error::Error,
    ops::auth::DelegatedTokenOps,
};

///
/// DelegatedTokenApi
///

pub struct DelegatedTokenApi;

impl DelegatedTokenApi {
    pub fn sign_delegation_cert(cert: DelegationCert) -> Result<DelegationProof, Error> {
        DelegatedTokenOps::sign_delegation_cert(cert).map_err(Error::from)
    }

    pub fn verify_delegation_proof(
        proof: &DelegationProof,
        authority_pid: Principal,
    ) -> Result<(), Error> {
        DelegatedTokenOps::verify_delegation_proof(proof, authority_pid).map_err(Error::from)
    }

    pub fn sign_token(
        token_version: u16,
        claims: DelegatedTokenClaims,
        proof: DelegationProof,
    ) -> Result<DelegatedToken, Error> {
        DelegatedTokenOps::sign_token(token_version, claims, proof).map_err(Error::from)
    }

    pub fn verify_token(
        token: &DelegatedToken,
        authority_pid: Principal,
        now_secs: u64,
    ) -> Result<(), Error> {
        DelegatedTokenOps::verify_token(token, authority_pid, now_secs)
            .map(|_| ())
            .map_err(Error::from)
    }

    pub fn verify_token_claims(
        token: &DelegatedToken,
        authority_pid: Principal,
        now_secs: u64,
    ) -> Result<DelegatedTokenClaims, Error> {
        DelegatedTokenOps::verify_token(token, authority_pid, now_secs)
            .map(|verified| verified.claims)
            .map_err(Error::from)
    }
}
