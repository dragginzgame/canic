//! Delegation issuance workflow.
//!
//! This module defines the **operational workflow** for:
//! - issuing delegated signing authority
//!
//! It is intentionally *thin* and orchestration-only.
//! All cryptographic validation, authorization, and policy enforcement
//! occur elsewhere.

use crate::{
    InternalError, InternalErrorOrigin,
    cdk::types::Principal,
    dto::{
        auth::{
            DelegationProof, DelegationProvisionRequest, DelegationProvisionResponse,
            DelegationProvisionStatus, DelegationProvisionTargetKind,
            DelegationProvisionTargetResponse,
        },
        error::Error as ErrorDto,
    },
    log,
    log::Topic,
    ops::{
        auth::DelegatedTokenOps,
        ic::{IcOps, call::CallOps},
        runtime::env::EnvOps,
        storage::auth::DelegationStateOps,
    },
    protocol,
};
use std::{cell::RefCell, collections::BTreeMap};

thread_local! {
    static PENDING_DELEGATION_PROVISIONS: RefCell<BTreeMap<Principal, PendingDelegationProvision>> =
        RefCell::new(BTreeMap::new());
}

///
/// DelegationWorkflow
///
/// WHY THIS MODULE EXISTS
/// ----------------------
/// This module coordinates **delegation issuance** as a workflow,
/// separating *orchestration* from:
/// - cryptographic operations
/// - storage details
/// - authorization policy
///
/// Responsibilities:
/// - Call cryptographic primitives in the correct order
/// - Coordinate persistence and publication
///
/// Explicit non-responsibilities:
/// - Authorization (caller must enforce)
/// - Validation (delegation certs are assumed valid inputs)
/// - Retry or recovery logic
/// - Token verification
///
/// This separation ensures delegation remains auditable and predictable.
///

pub struct DelegationWorkflow;

#[derive(Clone, Debug)]
struct PendingDelegationProvision {
    request: DelegationProvisionRequest,
    include_root_verifier: bool,
}

// -------------------------------------------------------------------------
// Logging context
// -------------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
pub enum DelegationPushOrigin {
    Provisioning,
}

impl DelegationPushOrigin {
    const fn label(self) -> &'static str {
        match self {
            Self::Provisioning => "provisioning",
        }
    }
}

impl DelegationWorkflow {
    // -------------------------------------------------------------------------
    // Issuance
    // -------------------------------------------------------------------------

    pub(crate) fn provision_prepare(
        request: DelegationProvisionRequest,
        include_root_verifier: bool,
    ) -> Result<(), InternalError> {
        DelegatedTokenOps::prepare_delegation_cert_signature(&request.cert)?;

        let caller = IcOps::msg_caller();
        Self::set_pending(
            caller,
            PendingDelegationProvision {
                request,
                include_root_verifier,
            },
        );

        Ok(())
    }

    pub(crate) fn provision_get() -> Result<DelegationProof, InternalError> {
        let caller = IcOps::msg_caller();
        let pending = Self::pending(caller).ok_or_else(|| Self::pending_missing(caller))?;

        DelegatedTokenOps::get_delegation_cert_signature(pending.request.cert.clone())
    }

    pub(crate) async fn provision_finalize(
        proof: DelegationProof,
    ) -> Result<DelegationProvisionResponse, InternalError> {
        let caller = IcOps::msg_caller();
        let pending = Self::pending(caller).ok_or_else(|| Self::pending_missing(caller))?;

        if proof.cert != pending.request.cert {
            return Err(InternalError::workflow(
                InternalErrorOrigin::Workflow,
                "delegation finalize proof cert does not match prepared request",
            ));
        }

        let root_pid = EnvOps::root_pid()?;
        DelegatedTokenOps::verify_delegation_proof(&proof, root_pid)?;

        log!(
            Topic::Auth,
            Info,
            "delegation provision issued proof signer={} issued_at={} expires_at={}",
            proof.cert.signer_pid,
            proof.cert.issued_at,
            proof.cert.expires_at
        );
        let mut results = Vec::new();

        for target in &pending.request.signer_targets {
            let result = Self::push_proof(
                *target,
                &proof,
                DelegationProvisionTargetKind::Signer,
                DelegationPushOrigin::Provisioning,
            )
            .await;
            results.push(result);
        }

        for target in &pending.request.verifier_targets {
            let result = Self::push_proof(
                *target,
                &proof,
                DelegationProvisionTargetKind::Verifier,
                DelegationPushOrigin::Provisioning,
            )
            .await;
            results.push(result);
        }

        if pending.include_root_verifier {
            DelegationStateOps::set_proof_from_dto(proof.clone());
        }

        Self::clear_pending(caller);

        Ok(DelegationProvisionResponse { proof, results })
    }

    pub(crate) async fn push_proof(
        target: Principal,
        proof: &DelegationProof,
        kind: DelegationProvisionTargetKind,
        origin: DelegationPushOrigin,
    ) -> DelegationProvisionTargetResponse {
        log!(
            Topic::Auth,
            Info,
            "delegation push attempt origin={} kind={:?} target={} signer={} issued_at={} expires_at={}",
            origin.label(),
            kind,
            target,
            proof.cert.signer_pid,
            proof.cert.issued_at,
            proof.cert.expires_at
        );

        let method = match kind {
            DelegationProvisionTargetKind::Signer => protocol::CANIC_DELEGATION_SET_SIGNER_PROOF,
            DelegationProvisionTargetKind::Verifier => {
                protocol::CANIC_DELEGATION_SET_VERIFIER_PROOF
            }
        };

        let call = match CallOps::unbounded_wait(target, method).with_arg(proof.clone()) {
            Ok(call) => call,
            Err(err) => {
                let response = Self::failure(target, kind, ErrorDto::from(err));
                Self::log_push_result(&response, origin);
                return response;
            }
        };

        let result = match call.execute().await {
            Ok(result) => result,
            Err(err) => {
                let response = Self::failure(target, kind, ErrorDto::from(err));
                Self::log_push_result(&response, origin);
                return response;
            }
        };

        let response: Result<(), ErrorDto> = match result.candid() {
            Ok(response) => response,
            Err(err) => {
                let response = Self::failure(target, kind, ErrorDto::from(err));
                Self::log_push_result(&response, origin);
                return response;
            }
        };

        let response = match response {
            Ok(()) => DelegationProvisionTargetResponse {
                target,
                kind,
                status: DelegationProvisionStatus::Ok,
                error: None,
            },
            Err(err) => Self::failure(target, kind, err),
        };

        Self::log_push_result(&response, origin);
        response
    }

    const fn failure(
        target: Principal,
        kind: DelegationProvisionTargetKind,
        err: ErrorDto,
    ) -> DelegationProvisionTargetResponse {
        DelegationProvisionTargetResponse {
            target,
            kind,
            status: DelegationProvisionStatus::Failed,
            error: Some(err),
        }
    }

    fn log_push_result(response: &DelegationProvisionTargetResponse, origin: DelegationPushOrigin) {
        match response.status {
            DelegationProvisionStatus::Ok => {
                log!(
                    Topic::Auth,
                    Info,
                    "delegation push ok origin={} kind={:?} target={}",
                    origin.label(),
                    response.kind,
                    response.target
                );
            }
            DelegationProvisionStatus::Failed => {
                let err = response
                    .error
                    .as_ref()
                    .map_or_else(|| "unknown error".to_string(), ToString::to_string);
                log!(
                    Topic::Auth,
                    Warn,
                    "delegation push failed origin={} kind={:?} target={} error={}",
                    origin.label(),
                    response.kind,
                    response.target,
                    err
                );
            }
        }
    }

    fn pending(caller: Principal) -> Option<PendingDelegationProvision> {
        PENDING_DELEGATION_PROVISIONS.with_borrow(|pending| pending.get(&caller).cloned())
    }

    fn set_pending(caller: Principal, pending: PendingDelegationProvision) {
        PENDING_DELEGATION_PROVISIONS.with_borrow_mut(|all| {
            all.insert(caller, pending);
        });
    }

    fn clear_pending(caller: Principal) {
        PENDING_DELEGATION_PROVISIONS.with_borrow_mut(|all| {
            all.remove(&caller);
        });
    }

    fn pending_missing(caller: Principal) -> InternalError {
        InternalError::workflow(
            InternalErrorOrigin::Workflow,
            format!("no pending delegation provision for caller {caller}"),
        )
    }
}
