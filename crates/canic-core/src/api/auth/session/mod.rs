use super::AuthApi;
use crate::{
    access::auth::validate_delegated_session_subject,
    cdk::types::Principal,
    dto::{auth::DelegatedToken, error::Error},
    ops::{
        auth::{AuthOps, DelegatedSessionExpiryClamp},
        config::ConfigOps,
        ic::IcOps,
        runtime::metrics::auth::{
            record_session_bootstrap_rejected_capacity, record_session_bootstrap_rejected_disabled,
            record_session_bootstrap_rejected_replay_conflict,
            record_session_bootstrap_rejected_replay_reused,
            record_session_bootstrap_rejected_subject_mismatch,
            record_session_bootstrap_rejected_subject_rejected,
            record_session_bootstrap_rejected_token_invalid,
            record_session_bootstrap_rejected_ttl_invalid,
            record_session_bootstrap_rejected_wallet_caller_rejected,
            record_session_bootstrap_replay_idempotent, record_session_cleared,
            record_session_created, record_session_pruned, record_session_replaced,
        },
        storage::auth::{
            AuthStateOps, DelegatedSession, DelegatedSessionBootstrapBinding,
            DelegatedSessionUpsertResult,
        },
    },
};
use sha2::{Digest, Sha256};

impl AuthApi {
    /// Persist a temporary delegated session subject for the caller wallet.
    pub fn set_delegated_session_subject(
        delegated_subject: Principal,
        bootstrap_token: DelegatedToken,
        requested_ttl_secs: Option<u64>,
    ) -> Result<(), Error> {
        let cfg = ConfigOps::delegated_tokens_config().map_err(Error::from)?;
        if !cfg.enabled {
            record_session_bootstrap_rejected_disabled();
            return Err(Error::forbidden(Self::DELEGATED_TOKENS_DISABLED));
        }

        let wallet_caller = IcOps::msg_caller();
        if let Err(reason) = validate_delegated_session_subject(wallet_caller) {
            record_session_bootstrap_rejected_wallet_caller_rejected();
            return Err(Error::forbidden(format!(
                "delegated session wallet caller rejected: {reason}"
            )));
        }

        if let Err(reason) = validate_delegated_session_subject(delegated_subject) {
            record_session_bootstrap_rejected_subject_rejected();
            return Err(Error::forbidden(format!(
                "delegated session subject rejected: {reason}"
            )));
        }

        let issued_at = IcOps::now_secs();
        let max_ttl_secs = Self::delegated_token_max_ttl_secs()?;
        let verified_subject = Self::verify_token_material(
            &bootstrap_token,
            max_ttl_secs,
            max_ttl_secs,
            &[],
            issued_at,
        )
        .inspect_err(|_| record_session_bootstrap_rejected_token_invalid())?;

        if verified_subject != delegated_subject {
            record_session_bootstrap_rejected_subject_mismatch();
            return Err(Error::forbidden(format!(
                "delegated session subject mismatch: requested={delegated_subject} token_subject={verified_subject}"
            )));
        }

        let configured_max_ttl_secs = cfg
            .max_ttl_secs
            .unwrap_or(Self::MAX_DELEGATED_SESSION_TTL_SECS);
        let expires_at = Self::clamp_delegated_session_expires_at(
            issued_at,
            bootstrap_token.claims.expires_at,
            configured_max_ttl_secs,
            requested_ttl_secs,
        )
        .inspect_err(|_| record_session_bootstrap_rejected_ttl_invalid())?;

        let token_fingerprint =
            Self::delegated_session_bootstrap_token_fingerprint(&bootstrap_token)
                .inspect_err(|_| record_session_bootstrap_rejected_token_invalid())?;

        if Self::enforce_bootstrap_replay_policy(
            wallet_caller,
            delegated_subject,
            token_fingerprint,
            issued_at,
        )? {
            return Ok(());
        }

        let had_active_session =
            AuthStateOps::delegated_session(wallet_caller, issued_at).is_some();

        let upsert_result = AuthStateOps::upsert_delegated_session_with_bootstrap_binding(
            DelegatedSession {
                wallet_pid: wallet_caller,
                delegated_pid: delegated_subject,
                issued_at,
                expires_at,
                bootstrap_token_fingerprint: Some(token_fingerprint),
            },
            DelegatedSessionBootstrapBinding {
                wallet_pid: wallet_caller,
                delegated_pid: delegated_subject,
                token_fingerprint,
                bound_at: issued_at,
                expires_at: bootstrap_token.claims.expires_at,
            },
            issued_at,
        );
        if !matches!(upsert_result, DelegatedSessionUpsertResult::Upserted) {
            record_session_bootstrap_rejected_capacity();
            return Err(Error::exhausted(delegated_session_capacity_message(
                upsert_result,
            )));
        }

        if had_active_session {
            record_session_replaced();
        } else {
            record_session_created();
        }

        Ok(())
    }

    /// Remove the caller's delegated session subject.
    pub fn clear_delegated_session() {
        let wallet_caller = IcOps::msg_caller();
        let had_active_session =
            AuthStateOps::delegated_session(wallet_caller, IcOps::now_secs()).is_some();
        AuthStateOps::clear_delegated_session(wallet_caller);
        if had_active_session {
            record_session_cleared();
        }
    }

    /// Read the caller's active delegated session subject, if configured.
    #[must_use]
    pub fn delegated_session_subject() -> Option<Principal> {
        let wallet_caller = IcOps::msg_caller();
        AuthStateOps::delegated_session_subject(wallet_caller, IcOps::now_secs())
    }

    /// Prune all currently expired delegated sessions.
    #[must_use]
    pub fn prune_expired_delegated_sessions() -> usize {
        let now_secs = IcOps::now_secs();
        let removed = AuthStateOps::prune_expired_delegated_sessions(now_secs);
        let _ = AuthStateOps::prune_expired_delegated_session_bootstrap_bindings(now_secs);
        if removed > 0 {
            record_session_pruned(removed);
        }
        removed
    }

    // Fingerprint a bootstrap token for replay protection and idempotence checks.
    fn delegated_session_bootstrap_token_fingerprint(
        token: &DelegatedToken,
    ) -> Result<[u8; 32], Error> {
        let token_bytes = crate::cdk::candid::encode_one(token).map_err(|err| {
            Error::internal(format!("bootstrap token fingerprint encode failed: {err}"))
        })?;
        let mut hasher = Sha256::new();
        hasher.update(Self::SESSION_BOOTSTRAP_TOKEN_FINGERPRINT_DOMAIN);
        hasher.update(token_bytes);
        Ok(hasher.finalize().into())
    }

    // Enforce replay policy for delegated-session bootstrap by token fingerprint.
    fn enforce_bootstrap_replay_policy(
        wallet_caller: Principal,
        delegated_subject: Principal,
        token_fingerprint: [u8; 32],
        issued_at: u64,
    ) -> Result<bool, Error> {
        let Some(binding) =
            AuthStateOps::delegated_session_bootstrap_binding(token_fingerprint, issued_at)
        else {
            return Ok(false);
        };

        if binding.wallet_pid == wallet_caller && binding.delegated_pid == delegated_subject {
            let active_same_session = AuthStateOps::delegated_session(wallet_caller, issued_at)
                .is_some_and(|session| {
                    session.delegated_pid == delegated_subject
                        && session.bootstrap_token_fingerprint == Some(token_fingerprint)
                });

            if active_same_session {
                record_session_bootstrap_replay_idempotent();
                return Ok(true);
            }

            record_session_bootstrap_rejected_replay_reused();
            return Err(Error::forbidden(
                "delegated session bootstrap token replay rejected; use a fresh token",
            ));
        }

        record_session_bootstrap_rejected_replay_conflict();
        Err(Error::forbidden(format!(
            "delegated session bootstrap token already bound (wallet={} delegated_subject={})",
            binding.wallet_pid, binding.delegated_pid
        )))
    }

    // Clamp delegated-session lifetime against token expiry, config, and request TTL.
    pub(super) fn clamp_delegated_session_expires_at(
        now_secs: u64,
        token_expires_at: u64,
        configured_max_ttl_secs: u64,
        requested_ttl_secs: Option<u64>,
    ) -> Result<u64, Error> {
        match AuthOps::clamp_delegated_session_expires_at(
            now_secs,
            token_expires_at,
            configured_max_ttl_secs,
            requested_ttl_secs,
        ) {
            DelegatedSessionExpiryClamp::Accepted(expires_at) => Ok(expires_at),
            DelegatedSessionExpiryClamp::InvalidConfiguredMaxTtl => Err(Error::invariant(
                "delegated session configured max ttl_secs must be greater than zero",
            )),
            DelegatedSessionExpiryClamp::InvalidRequestedTtl => Err(Error::invalid(
                "delegated session requested ttl_secs must be greater than zero",
            )),
            DelegatedSessionExpiryClamp::ExpiredToken => Err(Error::forbidden(
                "delegated session bootstrap token is expired",
            )),
        }
    }
}

fn delegated_session_capacity_message(result: DelegatedSessionUpsertResult) -> String {
    match result {
        DelegatedSessionUpsertResult::Upserted => {
            "delegated session state was already updated".to_string()
        }
        DelegatedSessionUpsertResult::SessionCapacityReached { capacity } => {
            format!("delegated session capacity reached ({capacity})")
        }
        DelegatedSessionUpsertResult::SessionSubjectCapacityReached {
            delegated_pid,
            capacity,
        } => format!("delegated session subject capacity reached for {delegated_pid} ({capacity})"),
        DelegatedSessionUpsertResult::BootstrapBindingCapacityReached { capacity } => {
            format!("delegated session bootstrap binding capacity reached ({capacity})")
        }
        DelegatedSessionUpsertResult::BootstrapBindingSubjectCapacityReached {
            delegated_pid,
            capacity,
        } => format!(
            "delegated session bootstrap binding subject capacity reached for {delegated_pid} ({capacity})"
        ),
    }
}
