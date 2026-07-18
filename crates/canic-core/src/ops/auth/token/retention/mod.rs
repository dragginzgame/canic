//! Module: ops::auth::token::retention
//!
//! Responsibility: bound and retain issuer-local delegated-token preparations.
//! Does not own: replay response storage, proof construction, or endpoint admission.
//! Boundary: token ops prune and admit here before adding a canister-signature witness.

#[cfg(test)]
mod tests;

use crate::{
    InternalError,
    cdk::types::Principal,
    ops::{
        auth::delegated::prepare::PreparedDelegatedToken,
        replay::receipt::ReplayReceiptRetentionLimits,
    },
};
use std::{cell::RefCell, collections::BTreeMap};
use thiserror::Error as ThisError;

const MAX_RETAINED_DELEGATED_TOKENS_PER_CALLER: usize = 64;
const MAX_RETAINED_DELEGATED_TOKENS_GLOBAL: usize = 512;

pub(super) const DELEGATED_TOKEN_REPLAY_RETENTION_LIMITS: ReplayReceiptRetentionLimits =
    ReplayReceiptRetentionLimits {
        max_active_per_actor: MAX_RETAINED_DELEGATED_TOKENS_PER_CALLER,
        max_active_per_command_kind: MAX_RETAINED_DELEGATED_TOKENS_GLOBAL,
        purge_scan_limit: 64,
    };

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) struct RetainedDelegatedTokenKey {
    claims_hash: [u8; 32],
    prepared_by: Vec<u8>,
}

impl RetainedDelegatedTokenKey {
    pub(super) fn new(claims_hash: [u8; 32], prepared_by: Principal) -> Self {
        Self {
            claims_hash,
            prepared_by: prepared_by.as_slice().to_vec(),
        }
    }

    fn belongs_to(&self, caller: Principal) -> bool {
        self.prepared_by == caller.as_slice()
    }
}

#[derive(Clone, Debug)]
pub(super) struct RetainedDelegatedToken {
    pub prepared: PreparedDelegatedToken,
    pub retrieval_expires_at_ns: u64,
}

#[derive(Debug, Eq, PartialEq, ThisError)]
pub(super) enum RetainedDelegatedTokenLookupError {
    #[error("delegated token retrieval window expired at {expires_at_ns}")]
    Expired { expires_at_ns: u64 },
    #[error("delegated token was not prepared or has been pruned")]
    Missing,
}

thread_local! {
    static RETAINED_DELEGATED_TOKENS: RefCell<BTreeMap<RetainedDelegatedTokenKey, RetainedDelegatedToken>> =
        const { RefCell::new(BTreeMap::new()) };
}

pub(super) fn prune_and_admit(prepared_by: Principal, now_ns: u64) -> Result<(), InternalError> {
    RETAINED_DELEGATED_TOKENS.with_borrow_mut(|retained| {
        retained.retain(|_, token| now_ns < token.retrieval_expires_at_ns);

        let caller_count = retained
            .keys()
            .filter(|key| key.belongs_to(prepared_by))
            .count();
        if caller_count >= DELEGATED_TOKEN_REPLAY_RETENTION_LIMITS.max_active_per_actor {
            return Err(InternalError::resource_exhausted(format!(
                "delegated token preparation capacity exceeded for caller; max_retained={}",
                DELEGATED_TOKEN_REPLAY_RETENTION_LIMITS.max_active_per_actor
            )));
        }

        if retained.len() >= DELEGATED_TOKEN_REPLAY_RETENTION_LIMITS.max_active_per_command_kind {
            return Err(InternalError::resource_exhausted(format!(
                "delegated token preparation global capacity exceeded; max_retained={}",
                DELEGATED_TOKEN_REPLAY_RETENTION_LIMITS.max_active_per_command_kind
            )));
        }

        Ok(())
    })
}

pub(super) fn insert(key: RetainedDelegatedTokenKey, retained_token: RetainedDelegatedToken) {
    RETAINED_DELEGATED_TOKENS.with_borrow_mut(|retained| {
        retained.insert(key, retained_token);
    });
}

pub(super) fn get(
    key: &RetainedDelegatedTokenKey,
    now_ns: u64,
) -> Result<RetainedDelegatedToken, RetainedDelegatedTokenLookupError> {
    let retained = RETAINED_DELEGATED_TOKENS.with_borrow(|tokens| tokens.get(key).cloned());
    let retained = retained.ok_or(RetainedDelegatedTokenLookupError::Missing)?;
    if now_ns >= retained.retrieval_expires_at_ns {
        return Err(RetainedDelegatedTokenLookupError::Expired {
            expires_at_ns: retained.retrieval_expires_at_ns,
        });
    }
    Ok(retained)
}
