use super::DelegationApi;
use crate::{
    access::auth::validate_delegated_session_subject,
    cdk::types::Principal,
    dto::{auth::DelegatedToken, error::Error},
    ops::{
        auth::{DelegatedSessionExpiryClamp, DelegatedTokenOps, VerifyDelegatedTokenRuntimeInput},
        config::ConfigOps,
        ic::IcOps,
        runtime::metrics::auth::{
            record_session_bootstrap_rejected_disabled,
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
        storage::auth::{DelegatedSession, DelegatedSessionBootstrapBinding, DelegationStateOps},
    },
};
use sha2::{Digest, Sha256};

impl DelegationApi {
    /// Persist a temporary delegated session subject for the caller wallet.
    pub async fn set_delegated_session_subject(
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
        DelegatedTokenOps::ensure_root_public_key_cached(&bootstrap_token)
            .await
            .map_err(|err| {
                record_session_bootstrap_rejected_token_invalid();
                Self::map_delegation_error(err)
            })?;
        let max_ttl_secs = Self::delegated_auth_max_ttl_secs()?;
        let verified = DelegatedTokenOps::verify_token(VerifyDelegatedTokenRuntimeInput {
            token: &bootstrap_token,
            max_cert_ttl_secs: max_ttl_secs,
            max_token_ttl_secs: max_ttl_secs,
            required_scopes: &[],
            now_secs: issued_at,
        })
        .map_err(|err| {
            record_session_bootstrap_rejected_token_invalid();
            Self::map_delegation_error(err)
        })?;

        if verified.subject != delegated_subject {
            record_session_bootstrap_rejected_subject_mismatch();
            return Err(Error::forbidden(format!(
                "delegated session subject mismatch: requested={} token_subject={}",
                delegated_subject, verified.subject
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
            DelegationStateOps::delegated_session(wallet_caller, issued_at).is_some();

        DelegationStateOps::upsert_delegated_session(
            DelegatedSession {
                wallet_pid: wallet_caller,
                delegated_pid: delegated_subject,
                issued_at,
                expires_at,
                bootstrap_token_fingerprint: Some(token_fingerprint),
            },
            issued_at,
        );
        DelegationStateOps::upsert_delegated_session_bootstrap_binding(
            DelegatedSessionBootstrapBinding {
                wallet_pid: wallet_caller,
                delegated_pid: delegated_subject,
                token_fingerprint,
                bound_at: issued_at,
                expires_at: bootstrap_token.claims.expires_at,
            },
            issued_at,
        );

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
            DelegationStateOps::delegated_session(wallet_caller, IcOps::now_secs()).is_some();
        DelegationStateOps::clear_delegated_session(wallet_caller);
        if had_active_session {
            record_session_cleared();
        }
    }

    /// Read the caller's active delegated session subject, if configured.
    #[must_use]
    pub fn delegated_session_subject() -> Option<Principal> {
        let wallet_caller = IcOps::msg_caller();
        DelegationStateOps::delegated_session_subject(wallet_caller, IcOps::now_secs())
    }

    /// Prune all currently expired delegated sessions.
    #[must_use]
    pub fn prune_expired_delegated_sessions() -> usize {
        let now_secs = IcOps::now_secs();
        let removed = DelegationStateOps::prune_expired_delegated_sessions(now_secs);
        let _ = DelegationStateOps::prune_expired_delegated_session_bootstrap_bindings(now_secs);
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
            DelegationStateOps::delegated_session_bootstrap_binding(token_fingerprint, issued_at)
        else {
            return Ok(false);
        };

        if binding.wallet_pid == wallet_caller && binding.delegated_pid == delegated_subject {
            let active_same_session =
                DelegationStateOps::delegated_session(wallet_caller, issued_at).is_some_and(
                    |session| {
                        session.delegated_pid == delegated_subject
                            && session.bootstrap_token_fingerprint == Some(token_fingerprint)
                    },
                );

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
        match DelegatedTokenOps::clamp_delegated_session_expires_at(
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
