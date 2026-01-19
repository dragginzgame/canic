use crate::{
    cdk::types::Principal,
    dto::{
        auth::{DelegatedToken, DelegatedTokenClaims, DelegationCert, DelegationProof},
        error::Error,
    },
    error::InternalErrorClass,
    ops::auth::DelegatedTokenOps,
};

///
/// DelegatedTokenApi
///

pub struct DelegatedTokenApi;

impl DelegatedTokenApi {
    fn map_token_error(err: crate::InternalError) -> Error {
        if err.to_string().contains("certified query required") {
            return Error::invalid("certified query required");
        }

        match err.class() {
            InternalErrorClass::Infra | InternalErrorClass::Ops | InternalErrorClass::Workflow => {
                Error::internal(err.to_string())
            }
            _ => Error::from(err),
        }
    }

    /// Sign a delegation cert.
    ///
    /// Requires a certified query context; will fail under PocketIC or
    /// uncertified query engines.
    pub fn sign_delegation_cert(cert: DelegationCert) -> Result<DelegationProof, Error> {
        DelegatedTokenOps::sign_delegation_cert(cert).map_err(Self::map_token_error)
    }

    /// Structural verification for a delegation proof.
    pub fn verify_delegation_structure(
        proof: &DelegationProof,
        expected_signer: Option<Principal>,
    ) -> Result<(), Error> {
        DelegatedTokenOps::verify_delegation_structure(proof, expected_signer)
            .map_err(Self::map_token_error)
    }

    /// Cryptographic verification for a delegation proof.
    ///
    /// Requires a certified query context; will fail under PocketIC or
    /// uncertified query engines.
    pub fn verify_delegation_signature(
        proof: &DelegationProof,
        authority_pid: Principal,
    ) -> Result<(), Error> {
        DelegatedTokenOps::verify_delegation_signature(proof, authority_pid)
            .map_err(Self::map_token_error)
    }

    /// Full delegation proof verification (structure + signature).
    ///
    /// Signature verification requires a certified query context and will fail
    /// under PocketIC or uncertified query engines.
    pub fn verify_delegation_proof(
        proof: &DelegationProof,
        authority_pid: Principal,
    ) -> Result<(), Error> {
        DelegatedTokenOps::verify_delegation_proof(proof, authority_pid)
            .map_err(Self::map_token_error)
    }

    pub fn sign_token(
        token_version: u16,
        claims: DelegatedTokenClaims,
        proof: DelegationProof,
    ) -> Result<DelegatedToken, Error> {
        DelegatedTokenOps::sign_token(token_version, claims, proof).map_err(Self::map_token_error)
    }

    /// Structural verification for a delegated token.
    pub fn verify_token_structure(token: &DelegatedToken, now_secs: u64) -> Result<(), Error> {
        DelegatedTokenOps::verify_token_structure(token, now_secs).map_err(Self::map_token_error)
    }

    /// Cryptographic verification for a delegated token.
    ///
    /// Requires a certified query context; will fail under PocketIC or
    /// uncertified query engines.
    pub fn verify_token_signature(
        token: &DelegatedToken,
        authority_pid: Principal,
    ) -> Result<(), Error> {
        DelegatedTokenOps::verify_token_signature(token, authority_pid)
            .map_err(Self::map_token_error)
    }

    /// Full delegated token verification (structure + signature).
    ///
    /// Signature verification requires a certified query context and will fail
    /// under PocketIC or uncertified query engines.
    pub fn verify_token(
        token: &DelegatedToken,
        authority_pid: Principal,
        now_secs: u64,
    ) -> Result<(), Error> {
        DelegatedTokenOps::verify_token(token, authority_pid, now_secs)
            .map(|_| ())
            .map_err(Self::map_token_error)
    }

    /// Return verified claims after full token verification.
    ///
    /// Signature verification requires a certified query context and will fail
    /// under PocketIC or uncertified query engines.
    pub fn verify_token_claims(
        token: &DelegatedToken,
        authority_pid: Principal,
        now_secs: u64,
    ) -> Result<DelegatedTokenClaims, Error> {
        DelegatedTokenOps::verify_token(token, authority_pid, now_secs)
            .map(|verified| verified.claims)
            .map_err(Self::map_token_error)
    }
}
