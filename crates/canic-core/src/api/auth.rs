use crate::{
    api::access::auth::AuthAccessApi,
    cdk::types::Principal,
    dto::{
        auth::{DelegationAdminCommand, DelegationAdminResponse, DelegationCert, DelegationProof},
        error::Error,
    },
    error::InternalErrorClass,
    ops::{
        auth::DelegatedTokenOps, config::ConfigOps, ic::IcOps, runtime::env::EnvOps,
        storage::auth::DelegationStateOps,
    },
    workflow::auth::DelegationWorkflow,
};
use std::{sync::Arc, time::Duration};

///
/// DelegationApi
///
/// Requires delegation.enabled = true in config.
///

pub struct DelegationApi;

impl DelegationApi {
    fn map_delegation_error(err: crate::InternalError) -> Error {
        match err.class() {
            InternalErrorClass::Infra | InternalErrorClass::Ops | InternalErrorClass::Workflow => {
                Error::internal(err.to_string())
            }
            _ => Error::from(err),
        }
    }

    pub fn prepare_issue(cert: DelegationCert) -> Result<(), Error> {
        let cfg = ConfigOps::delegation_config().map_err(Error::from)?;
        if !cfg.enabled {
            return Err(Error::forbidden("delegation disabled"));
        }

        // Update-only step for certified delegation signatures.
        DelegationWorkflow::prepare_delegation(&cert).map_err(Self::map_delegation_error)
    }

    pub fn get_issue(cert: DelegationCert) -> Result<DelegationProof, Error> {
        let cfg = ConfigOps::delegation_config().map_err(Error::from)?;
        if !cfg.enabled {
            return Err(Error::forbidden("delegation disabled"));
        }

        // Query-only step; requires a data certificate in the query context.
        DelegationWorkflow::get_delegation(cert).map_err(Self::map_delegation_error)
    }

    pub fn issue_and_store(cert: DelegationCert) -> Result<DelegationProof, Error> {
        let cfg = ConfigOps::delegation_config().map_err(Error::from)?;
        if !cfg.enabled {
            return Err(Error::forbidden("delegation disabled"));
        }

        DelegationWorkflow::issue_and_store(cert).map_err(Self::map_delegation_error)
    }

    pub fn store_proof(proof: DelegationProof) -> Result<(), Error> {
        let cfg = ConfigOps::delegation_config().map_err(Error::from)?;
        if !cfg.enabled {
            return Err(Error::forbidden("delegation disabled"));
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
        let cfg = ConfigOps::delegation_config().map_err(Error::from)?;
        if !cfg.enabled {
            return Err(Error::forbidden("delegation disabled"));
        }

        DelegationStateOps::proof_dto().ok_or_else(|| Error::not_found("delegation proof not set"))
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

    pub async fn start_rotation(interval_secs: u64) -> Result<bool, Error> {
        AuthAccessApi::caller_is_root(IcOps::msg_caller()).await?;

        let cfg = ConfigOps::delegation_config().map_err(Error::from)?;
        if !cfg.enabled {
            return Err(Error::forbidden("delegation disabled"));
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
                    Ok(build_rotation_cert(template.as_ref(), now_secs))
                }
            }),
            Arc::new(|proof| {
                DelegationStateOps::set_proof_from_dto(proof);
                Ok(())
            }),
        );

        Ok(started)
    }

    pub async fn stop_rotation() -> Result<bool, Error> {
        AuthAccessApi::caller_is_root(IcOps::msg_caller()).await?;
        Ok(DelegationWorkflow::stop_rotation())
    }
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
