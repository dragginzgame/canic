use crate::{
    InternalError, InternalErrorOrigin,
    cdk::types::Principal,
    dto::{
        auth::{
            DelegatedToken, DelegatedTokenClaims, DelegationAdminCommand, DelegationAdminResponse,
            DelegationCert, DelegationProof, DelegationProvisionRequest,
            DelegationProvisionResponse, DelegationProvisionTargetKind,
        },
        error::Error,
    },
    error::InternalErrorClass,
    ops::{
        auth::DelegatedTokenOps,
        config::ConfigOps,
        ic::IcOps,
        runtime::env::EnvOps,
        runtime::metrics::auth::record_signer_mint_without_proof,
        storage::{
            auth::DelegationStateOps, placement::sharding_lifecycle::ShardingLifecycleOps,
            registry::subnet::SubnetRegistryOps,
        },
    },
    workflow::auth::DelegationWorkflow,
};
use std::{sync::Arc, time::Duration};

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

    pub async fn provision(
        request: DelegationProvisionRequest,
    ) -> Result<DelegationProvisionResponse, Error> {
        let cfg = ConfigOps::delegated_tokens_config().map_err(Error::from)?;
        if !cfg.enabled {
            return Err(Error::forbidden(Self::DELEGATED_TOKENS_DISABLED));
        }

        validate_issuance_policy(&request.cert)?;
        DelegationWorkflow::provision(request)
            .await
            .map_err(Self::map_delegation_error)
    }

    pub fn store_proof(proof: DelegationProof) -> Result<(), Error> {
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

        DelegatedTokenOps::verify_delegation_proof(&proof, root_pid)
            .map_err(Self::map_delegation_error)?;

        DelegationStateOps::set_proof_from_dto(proof);

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

///
/// DelegationAdminApi
///
/// Admin façade for delegation rotation control.
///

pub struct DelegationAdminApi;

impl DelegationAdminApi {
    pub async fn admin(cmd: DelegationAdminCommand) -> Result<DelegationAdminResponse, Error> {
        match cmd {
            DelegationAdminCommand::StartRotation { interval_secs } => {
                let started = Self::start_rotation(interval_secs).await?;
                Ok(if started {
                    DelegationAdminResponse::RotationStarted
                } else {
                    DelegationAdminResponse::RotationAlreadyRunning
                })
            }
            DelegationAdminCommand::StopRotation => {
                let stopped = Self::stop_rotation().await?;
                Ok(if stopped {
                    DelegationAdminResponse::RotationStopped
                } else {
                    DelegationAdminResponse::RotationNotRunning
                })
            }
        }
    }

    #[allow(clippy::unused_async)]
    pub async fn start_rotation(interval_secs: u64) -> Result<bool, Error> {
        let cfg = ConfigOps::delegated_tokens_config().map_err(Error::from)?;
        if !cfg.enabled {
            return Err(Error::forbidden(DelegationApi::DELEGATED_TOKENS_DISABLED));
        }

        if interval_secs == 0 {
            return Err(Error::invalid(
                "rotation interval must be greater than zero",
            ));
        }

        let template = rotation_template()?;
        let template = Arc::new(template);
        let interval = Duration::from_secs(interval_secs);

        let started = DelegationWorkflow::start_rotation(
            interval,
            Arc::new({
                let template = Arc::clone(&template);
                move || {
                    let now_secs = IcOps::now_secs();
                    let cert = build_rotation_cert(template.as_ref(), now_secs);
                    validate_issuance_policy_internal(&cert)?;
                    Ok(cert)
                }
            }),
            Arc::new(|proof| {
                DelegationStateOps::set_proof_from_dto(proof.clone());

                let targets = ShardingLifecycleOps::rotation_targets();
                if !targets.is_empty() {
                    IcOps::spawn(async move {
                        for target in targets {
                            let _ = DelegationWorkflow::push_proof(
                                target,
                                &proof,
                                DelegationProvisionTargetKind::Signer,
                            )
                            .await;
                        }
                    });
                }

                Ok(())
            }),
        );

        Ok(started)
    }

    #[allow(clippy::unused_async)]
    pub async fn stop_rotation() -> Result<bool, Error> {
        Ok(DelegationWorkflow::stop_rotation())
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

fn validate_issuance_policy_internal(cert: &DelegationCert) -> Result<(), InternalError> {
    validate_issuance_policy(cert)
        .map_err(|err| InternalError::domain(InternalErrorOrigin::Domain, err.message))
}

///
/// DelegationRotationTemplate
///

struct DelegationRotationTemplate {
    v: u16,
    signer_pid: Principal,
    audiences: Vec<String>,
    scopes: Vec<String>,
    ttl_secs: u64,
}

fn rotation_template() -> Result<DelegationRotationTemplate, Error> {
    let proof = DelegationStateOps::proof_dto()
        .ok_or_else(|| Error::not_found("delegation proof not set"))?;
    let cert = proof.cert;

    if cert.expires_at <= cert.issued_at {
        return Err(Error::invalid(
            "delegation cert expires_at must be greater than issued_at",
        ));
    }

    let ttl_secs = cert.expires_at - cert.issued_at;

    Ok(DelegationRotationTemplate {
        v: cert.v,
        signer_pid: cert.signer_pid,
        audiences: cert.audiences,
        scopes: cert.scopes,
        ttl_secs,
    })
}

fn build_rotation_cert(template: &DelegationRotationTemplate, now_secs: u64) -> DelegationCert {
    DelegationCert {
        v: template.v,
        signer_pid: template.signer_pid,
        audiences: template.audiences.clone(),
        scopes: template.scopes.clone(),
        issued_at: now_secs,
        expires_at: now_secs.saturating_add(template.ttl_secs),
    }
}
