use super::DelegatedTokenOps;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DelegatedSessionExpiryClamp {
    Accepted(u64),
    InvalidConfiguredMaxTtl,
    InvalidRequestedTtl,
    ExpiredToken,
}

impl DelegatedTokenOps {
    // Clamp delegated-session expiry against token, config, and requested TTL bounds.
    pub(crate) fn clamp_delegated_session_expires_at(
        now_secs: u64,
        token_expires_at: u64,
        configured_max_ttl_secs: u64,
        requested_ttl_secs: Option<u64>,
    ) -> DelegatedSessionExpiryClamp {
        if configured_max_ttl_secs == 0 {
            return DelegatedSessionExpiryClamp::InvalidConfiguredMaxTtl;
        }

        if let Some(ttl_secs) = requested_ttl_secs
            && ttl_secs == 0
        {
            return DelegatedSessionExpiryClamp::InvalidRequestedTtl;
        }

        let mut expires_at = token_expires_at;
        expires_at = expires_at.min(now_secs.saturating_add(configured_max_ttl_secs));
        if let Some(ttl_secs) = requested_ttl_secs {
            expires_at = expires_at.min(now_secs.saturating_add(ttl_secs));
        }

        if expires_at <= now_secs {
            DelegatedSessionExpiryClamp::ExpiredToken
        } else {
            DelegatedSessionExpiryClamp::Accepted(expires_at)
        }
    }
}
