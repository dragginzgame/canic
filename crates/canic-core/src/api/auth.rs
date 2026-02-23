use crate::{
    cdk::types::Principal,
    dto::{
        auth::{
            DelegatedToken, DelegatedTokenClaims, DelegationCert, DelegationProof,
            DelegationProvisionRequest, DelegationProvisionResponse, DelegationProvisionTargetKind,
            DelegationRequest,
        },
        error::Error,
    },
    error::InternalErrorClass,
    log,
    log::Topic,
    ops::{
        auth::DelegatedTokenOps,
        config::ConfigOps,
        ic::IcOps,
        runtime::env::EnvOps,
        runtime::metrics::auth::record_signer_mint_without_proof,
        storage::{auth::DelegationStateOps, registry::subnet::SubnetRegistryOps},
    },
    workflow::auth::DelegationWorkflow,
};

///
/// DelegationApi
///
/// Requires auth.delegated_tokens.enabled = true in config.
///

pub struct DelegationApi;

impl DelegationApi {
    const DELEGATED_TOKENS_DISABLED: &str =
        "delegated token auth disabled; set auth.delegated_tokens.enabled=true in canic.toml";

    fn map_delegation_error(err: crate::InternalError) -> Error {
        match err.class() {
            InternalErrorClass::Infra | InternalErrorClass::Ops | InternalErrorClass::Workflow => {
                Error::internal(err.to_string())
            }
            _ => Error::from(err),
        }
    }

    /// Full delegation proof verification (structure + signature).
    ///
    /// Purely local verification; does not read certified data or require a
    /// query context.
    pub fn verify_delegation_proof(
        proof: &DelegationProof,
        authority_pid: Principal,
    ) -> Result<(), Error> {
        DelegatedTokenOps::verify_delegation_proof(proof, authority_pid)
            .map_err(Self::map_delegation_error)
    }

    pub fn prepare_delegation_cert_signature(cert: &DelegationCert) -> Result<(), Error> {
        DelegatedTokenOps::prepare_delegation_cert_signature(cert)
            .map_err(Self::map_delegation_error)
    }

    pub fn get_delegation_cert_signature(cert: DelegationCert) -> Result<DelegationProof, Error> {
        DelegatedTokenOps::get_delegation_cert_signature(cert).map_err(Self::map_delegation_error)
    }

    pub fn prepare_token_signature(
        token_version: u16,
        claims: &DelegatedTokenClaims,
        proof: &DelegationProof,
    ) -> Result<(), Error> {
        DelegatedTokenOps::prepare_token_signature(token_version, claims, proof)
            .map_err(Self::map_delegation_error)
    }

    pub fn get_token_signature(
        token_version: u16,
        claims: DelegatedTokenClaims,
        proof: DelegationProof,
    ) -> Result<DelegatedToken, Error> {
        DelegatedTokenOps::get_token_signature(token_version, claims, proof)
            .map_err(Self::map_delegation_error)
    }

    pub fn sign_token(
        token_version: u16,
        claims: DelegatedTokenClaims,
        proof: DelegationProof,
    ) -> Result<DelegatedToken, Error> {
        DelegatedTokenOps::sign_token(token_version, claims, proof)
            .map_err(Self::map_delegation_error)
    }

    /// Full delegated token verification (structure + signature).
    ///
    /// Purely local verification; does not read certified data or require a
    /// query context.
    pub fn verify_token(
        token: &DelegatedToken,
        authority_pid: Principal,
        now_secs: u64,
    ) -> Result<(), Error> {
        DelegatedTokenOps::verify_token(token, authority_pid, now_secs)
            .map(|_| ())
            .map_err(Self::map_delegation_error)
    }

    /// Verify a delegated token and return verified contents.
    ///
    /// This is intended for application-layer session construction.
    /// It performs full verification and returns verified claims and cert.
    pub fn verify_token_verified(
        token: &DelegatedToken,
        authority_pid: Principal,
        now_secs: u64,
    ) -> Result<(DelegatedTokenClaims, DelegationCert), Error> {
        DelegatedTokenOps::verify_token(token, authority_pid, now_secs)
            .map(|verified| (verified.claims, verified.cert))
            .map_err(Self::map_delegation_error)
    }

    /// admin-only delegation provisioning (root-only escape hatch).
    ///
    /// Not part of canonical delegation flow.
    /// Used for tests / tooling due to PocketIC limitations.
    ///
    /// Root does not infer targets; callers must supply them.
    pub async fn provision(
        request: DelegationProvisionRequest,
    ) -> Result<DelegationProvisionResponse, Error> {
        let cfg = ConfigOps::delegated_tokens_config().map_err(Error::from)?;
        if !cfg.enabled {
            return Err(Error::forbidden(Self::DELEGATED_TOKENS_DISABLED));
        }

        let root_pid = EnvOps::root_pid().map_err(Error::from)?;
        let caller = IcOps::msg_caller();
        if caller != root_pid {
            return Err(Error::forbidden(
                "delegation provision requires root caller",
            ));
        }

        validate_issuance_policy(&request.cert)?;
        log!(
            Topic::Auth,
            Info,
            "delegation provision start signer={} signer_targets={:?} verifier_targets={:?}",
            request.cert.signer_pid,
            request.signer_targets,
            request.verifier_targets
        );
        DelegationWorkflow::provision(request)
            .await
            .map_err(Self::map_delegation_error)
    }

    /// Canonical signer-initiated delegation request (user_shard -> root).
    ///
    /// Caller must match signer_pid and be registered to the subnet.
    pub async fn request_delegation(
        request: DelegationRequest,
    ) -> Result<DelegationProvisionResponse, Error> {
        let cfg = ConfigOps::delegated_tokens_config().map_err(Error::from)?;
        if !cfg.enabled {
            return Err(Error::forbidden(Self::DELEGATED_TOKENS_DISABLED));
        }

        let root_pid = EnvOps::root_pid().map_err(Error::from)?;
        if root_pid != IcOps::canister_self() {
            return Err(Error::forbidden("delegation request must target root"));
        }

        let caller = IcOps::msg_caller();
        if caller != request.signer_pid {
            return Err(Error::forbidden(
                "delegation request signer must match caller",
            ));
        }

        if request.ttl_secs == 0 {
            return Err(Error::invalid(
                "delegation ttl_secs must be greater than zero",
            ));
        }

        let now_secs = IcOps::now_secs();
        let cert = DelegationCert {
            v: 1,
            signer_pid: request.signer_pid,
            audiences: request.audiences,
            scopes: request.scopes,
            issued_at: now_secs,
            expires_at: now_secs.saturating_add(request.ttl_secs),
        };

        validate_issuance_policy(&cert)?;

        let response = DelegationWorkflow::provision(DelegationProvisionRequest {
            cert,
            signer_targets: vec![caller],
            verifier_targets: request.verifier_targets,
        })
        .await
        .map_err(Self::map_delegation_error)?;

        if request.include_root_verifier {
            DelegationStateOps::set_proof_from_dto(response.proof.clone());
        }

        Ok(response)
    }

    pub fn store_proof(
        proof: DelegationProof,
        kind: DelegationProvisionTargetKind,
    ) -> Result<(), Error> {
        let cfg = ConfigOps::delegated_tokens_config().map_err(Error::from)?;
        if !cfg.enabled {
            return Err(Error::forbidden(Self::DELEGATED_TOKENS_DISABLED));
        }

        let root_pid = EnvOps::root_pid().map_err(Error::from)?;
        let caller = IcOps::msg_caller();
        if caller != root_pid {
            return Err(Error::forbidden(
                "delegation proof store requires root caller",
            ));
        }

        if let Err(err) = DelegatedTokenOps::verify_delegation_proof(&proof, root_pid) {
            let local = IcOps::canister_self();
            log!(
                Topic::Auth,
                Warn,
                "delegation proof rejected kind={:?} local={} signer={} issued_at={} expires_at={} error={}",
                kind,
                local,
                proof.cert.signer_pid,
                proof.cert.issued_at,
                proof.cert.expires_at,
                err
            );
            return Err(Self::map_delegation_error(err));
        }

        DelegationStateOps::set_proof_from_dto(proof);
        let local = IcOps::canister_self();
        let stored = DelegationStateOps::proof_dto()
            .ok_or_else(|| Error::invariant("delegation proof missing after store"))?;
        log!(
            Topic::Auth,
            Info,
            "delegation proof stored kind={:?} local={} signer={} issued_at={} expires_at={}",
            kind,
            local,
            stored.cert.signer_pid,
            stored.cert.issued_at,
            stored.cert.expires_at
        );

        Ok(())
    }

    pub fn require_proof() -> Result<DelegationProof, Error> {
        let cfg = ConfigOps::delegated_tokens_config().map_err(Error::from)?;
        if !cfg.enabled {
            return Err(Error::forbidden(Self::DELEGATED_TOKENS_DISABLED));
        }

        DelegationStateOps::proof_dto().ok_or_else(|| {
            record_signer_mint_without_proof();
            Error::not_found("delegation proof not set")
        })
    }
}

fn validate_issuance_policy(cert: &DelegationCert) -> Result<(), Error> {
    if cert.expires_at <= cert.issued_at {
        return Err(Error::invalid(
            "delegation expires_at must be greater than issued_at",
        ));
    }

    if cert.audiences.is_empty() {
        return Err(Error::invalid("delegation audiences must not be empty"));
    }

    if cert.scopes.is_empty() {
        return Err(Error::invalid("delegation scopes must not be empty"));
    }

    if cert.audiences.iter().any(String::is_empty) {
        return Err(Error::invalid("delegation audience must not be empty"));
    }

    if cert.scopes.iter().any(String::is_empty) {
        return Err(Error::invalid("delegation scope must not be empty"));
    }

    let root_pid = EnvOps::root_pid().map_err(Error::from)?;
    if cert.signer_pid == root_pid {
        return Err(Error::invalid("delegation signer must not be root"));
    }

    let record = SubnetRegistryOps::get(cert.signer_pid)
        .ok_or_else(|| Error::invalid("delegation signer must be registered to subnet"))?;
    if record.role.is_root() {
        return Err(Error::invalid("delegation signer role must not be root"));
    }

    Ok(())
}
