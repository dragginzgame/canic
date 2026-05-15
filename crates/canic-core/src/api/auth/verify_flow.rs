use crate::{
    cdk::types::Principal,
    dto::auth::SignedRoleAttestation,
    format::display_optional,
    log,
    log::Topic,
    ops::{
        auth::{AuthExpiryError, AuthOpsError, AuthValidationError},
        runtime::metrics::auth::{
            record_attestation_epoch_rejected, record_attestation_unknown_key_id,
            record_attestation_verify_failed,
        },
    },
};
use std::future::Future;

#[derive(Debug)]
pub(super) enum RoleAttestationVerifyFlowError {
    Initial(AuthOpsError),
    Refresh {
        trigger: AuthOpsError,
        source: crate::InternalError,
    },
    PostRefresh(AuthOpsError),
}

pub(super) async fn verify_role_attestation_with_single_refresh<Verify, Refresh, RefreshFuture>(
    mut verify: Verify,
    mut refresh: Refresh,
) -> Result<(), RoleAttestationVerifyFlowError>
where
    Verify: FnMut() -> Result<(), AuthOpsError>,
    Refresh: FnMut() -> RefreshFuture,
    RefreshFuture: Future<Output = Result<(), crate::InternalError>>,
{
    match verify() {
        Ok(()) => Ok(()),
        Err(
            err @ AuthOpsError::Validation(AuthValidationError::AttestationUnknownKeyId { .. }),
        ) => {
            refresh()
                .await
                .map_err(|source| RoleAttestationVerifyFlowError::Refresh {
                    trigger: err,
                    source,
                })?;
            verify().map_err(RoleAttestationVerifyFlowError::PostRefresh)
        }
        Err(err) => Err(RoleAttestationVerifyFlowError::Initial(err)),
    }
}

pub(super) fn resolve_min_accepted_epoch(explicit: u64, configured: Option<u64>) -> u64 {
    if explicit > 0 {
        explicit
    } else {
        configured.unwrap_or(0)
    }
}

pub(super) fn record_attestation_verifier_rejection(err: &AuthOpsError) {
    record_attestation_verify_failed();
    match err {
        AuthOpsError::Validation(AuthValidationError::AttestationUnknownKeyId { .. }) => {
            record_attestation_unknown_key_id();
        }
        AuthOpsError::Expiry(AuthExpiryError::AttestationEpochRejected { .. }) => {
            record_attestation_epoch_rejected();
        }
        _ => {}
    }
}

pub(super) fn log_attestation_verifier_rejection(
    err: &AuthOpsError,
    attestation: &SignedRoleAttestation,
    caller: Principal,
    self_pid: Principal,
    phase: &str,
) {
    log!(
        Topic::Auth,
        Warn,
        "role attestation rejected phase={} local={} caller={} subject={} role={} key_id={} audience={} subnet={} issued_at={} expires_at={} epoch={} error={}",
        phase,
        self_pid,
        caller,
        attestation.payload.subject,
        attestation.payload.role,
        attestation.key_id,
        attestation.payload.audience,
        display_optional(attestation.payload.subnet_id),
        attestation.payload.issued_at,
        attestation.payload.expires_at,
        attestation.payload.epoch,
        err
    );
}

#[cfg(test)]
mod tests {
    use super::{RoleAttestationVerifyFlowError, verify_role_attestation_with_single_refresh};
    use crate::{
        InternalError, InternalErrorOrigin,
        ops::auth::{AuthOpsError, AuthSignatureError, AuthValidationError},
    };
    use futures::executor::block_on;
    use std::cell::Cell;

    #[test]
    fn role_attestation_refreshes_once_for_unknown_key_id() {
        let verify_calls = Cell::new(0);
        let refresh_calls = Cell::new(0);

        let result = block_on(verify_role_attestation_with_single_refresh(
            || {
                let call = verify_calls.get();
                verify_calls.set(call + 1);
                if call == 0 {
                    Err(unknown_key())
                } else {
                    Ok(())
                }
            },
            || {
                refresh_calls.set(refresh_calls.get() + 1);
                async { Ok(()) }
            },
        ));

        assert!(result.is_ok());
        assert_eq!(verify_calls.get(), 2);
        assert_eq!(refresh_calls.get(), 1);
    }

    #[test]
    fn role_attestation_does_not_refresh_signature_failures() {
        let verify_calls = Cell::new(0);
        let refresh_calls = Cell::new(0);

        let result = block_on(verify_role_attestation_with_single_refresh(
            || {
                verify_calls.set(verify_calls.get() + 1);
                Err(AuthOpsError::Signature(
                    AuthSignatureError::AttestationSignatureUnavailable,
                ))
            },
            || {
                refresh_calls.set(refresh_calls.get() + 1);
                async { Ok(()) }
            },
        ));

        assert!(matches!(
            result,
            Err(RoleAttestationVerifyFlowError::Initial(
                AuthOpsError::Signature(AuthSignatureError::AttestationSignatureUnavailable)
            ))
        ));
        assert_eq!(verify_calls.get(), 1);
        assert_eq!(refresh_calls.get(), 0);
    }

    #[test]
    fn role_attestation_reports_post_refresh_failure_after_single_retry() {
        let verify_calls = Cell::new(0);
        let refresh_calls = Cell::new(0);

        let result = block_on(verify_role_attestation_with_single_refresh(
            || {
                verify_calls.set(verify_calls.get() + 1);
                Err(unknown_key())
            },
            || {
                refresh_calls.set(refresh_calls.get() + 1);
                async { Ok(()) }
            },
        ));

        assert!(matches!(
            result,
            Err(RoleAttestationVerifyFlowError::PostRefresh(
                AuthOpsError::Validation(AuthValidationError::AttestationUnknownKeyId {
                    key_id: 7
                })
            ))
        ));
        assert_eq!(verify_calls.get(), 2);
        assert_eq!(refresh_calls.get(), 1);
    }

    #[test]
    fn role_attestation_reports_refresh_failure_without_second_verify() {
        let verify_calls = Cell::new(0);
        let refresh_calls = Cell::new(0);

        let result = block_on(verify_role_attestation_with_single_refresh(
            || {
                verify_calls.set(verify_calls.get() + 1);
                Err(unknown_key())
            },
            || {
                refresh_calls.set(refresh_calls.get() + 1);
                async {
                    Err(InternalError::ops(
                        InternalErrorOrigin::Ops,
                        "refresh failed",
                    ))
                }
            },
        ));

        assert!(matches!(
            result,
            Err(RoleAttestationVerifyFlowError::Refresh {
                trigger: AuthOpsError::Validation(AuthValidationError::AttestationUnknownKeyId {
                    key_id: 7
                }),
                ..
            })
        ));
        assert_eq!(verify_calls.get(), 1);
        assert_eq!(refresh_calls.get(), 1);
    }

    fn unknown_key() -> AuthOpsError {
        AuthOpsError::Validation(AuthValidationError::AttestationUnknownKeyId { key_id: 7 })
    }
}
