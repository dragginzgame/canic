//! Delegation issuance and rotation workflow.

use crate::{
    InternalError,
    dto::auth::{DelegationCert, DelegationProof},
    log,
    log::Topic,
    ops::auth::DelegatedTokenOps,
    ops::storage::auth::DelegationStateOps,
    workflow::runtime::timer::{TimerId, TimerWorkflow},
};
use std::{cell::RefCell, sync::Arc, time::Duration};

thread_local! {
    static ROTATION_TIMER: RefCell<Option<TimerId>> = const {
        RefCell::new(None)
    };
}

///
/// DelegationWorkflow
///

pub struct DelegationWorkflow;

impl DelegationWorkflow {
    // -------------------------------------------------------------------------
    // Issuance
    // -------------------------------------------------------------------------

    /// Issue a root-signed delegation proof for a delegated signer key.
    ///
    /// Authority is enforced by the caller if required.
    pub fn issue_delegation(cert: DelegationCert) -> Result<DelegationProof, InternalError> {
        DelegatedTokenOps::sign_delegation_cert(cert)
    }

    /// Issue and persist the delegation proof in stable state.
    ///
    /// Authority is enforced by the caller if required.
    pub fn issue_and_store(cert: DelegationCert) -> Result<DelegationProof, InternalError> {
        let proof = Self::issue_delegation(cert)?;
        DelegationStateOps::set_proof(proof.clone());
        Ok(proof)
    }

    // -------------------------------------------------------------------------
    // Rotation
    // -------------------------------------------------------------------------

    /// Start a periodic delegation rotation task.
    ///
    /// The caller supplies:
    /// - `build_cert`: constructs the next delegation cert.
    /// - `publish`: persists or distributes the resulting proof.
    ///
    /// Authority and key ownership are enforced by the caller.
    pub fn start_rotation(
        interval: Duration,
        build_cert: Arc<dyn Fn() -> Result<DelegationCert, InternalError> + Send + Sync>,
        publish: Arc<dyn Fn(DelegationProof) -> Result<(), InternalError> + Send + Sync>,
    ) -> bool {
        let init_build = build_cert.clone();
        let init_publish = publish.clone();
        let tick_build = build_cert;
        let tick_publish = publish;

        TimerWorkflow::set_guarded_interval(
            &ROTATION_TIMER,
            Duration::ZERO,
            "delegation:rotate:init",
            move || async move {
                Self::rotate_once(init_build.as_ref(), init_publish.as_ref());
            },
            interval,
            "delegation:rotate:interval",
            move || {
                let build = tick_build.clone();
                let publish = tick_publish.clone();
                async move {
                    Self::rotate_once(build.as_ref(), publish.as_ref());
                }
            },
        )
    }

    pub fn stop_rotation() -> bool {
        TimerWorkflow::clear_guarded(&ROTATION_TIMER)
    }

    fn rotate_once(
        build_cert: &dyn Fn() -> Result<DelegationCert, InternalError>,
        publish: &dyn Fn(DelegationProof) -> Result<(), InternalError>,
    ) {
        let cert = match build_cert() {
            Ok(cert) => cert,
            Err(err) => {
                log!(Topic::Auth, Warn, "delegation rotation build failed: {err}");
                return;
            }
        };

        let proof = match Self::issue_delegation(cert) {
            Ok(proof) => proof,
            Err(err) => {
                log!(
                    Topic::Auth,
                    Warn,
                    "delegation rotation signing failed: {err}"
                );
                return;
            }
        };

        if let Err(err) = publish(proof) {
            log!(
                Topic::Auth,
                Warn,
                "delegation rotation publish failed: {err}"
            );
        }
    }
}
