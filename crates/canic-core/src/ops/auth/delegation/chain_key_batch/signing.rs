//! Module: ops::auth::delegation::chain_key_batch::signing
//!
//! Responsibility: drive retry-safe chain-key batch signing state transitions.
//! Does not own: batch construction, issuer install planning, or timer orchestration.
//! Boundary: private helper for one persisted batch's management-canister signing step.

use super::{CHAIN_KEY_SIGNING_RETRY_BACKOFF_NS, SignNextChainKeyRootDelegationBatchResult};
use crate::{
    InternalError, InternalErrorOrigin,
    ops::{
        auth::delegated::chain_key_signing::{
            ChainKeySigner, ChainKeySigningPolicy, SignChainKeyBatchHeaderInput,
            sign_chain_key_batch_header,
        },
        storage::auth::{
            AuthStateOps, ChainKeyRootDelegationBatch, ChainKeyRootDelegationBatchStatus,
        },
    },
};

pub(in crate::ops::auth) async fn sign_next_chain_key_root_delegation_batch<S>(
    signing_policy: &ChainKeySigningPolicy,
    now_ns: u64,
    signer: &mut S,
) -> Result<SignNextChainKeyRootDelegationBatchResult, InternalError>
where
    S: ChainKeySigner,
{
    AuthStateOps::prune_chain_key_root_delegation_batches(now_ns);
    let Some(batch) = next_chain_key_batch_for_signing(now_ns) else {
        return Ok(SignNextChainKeyRootDelegationBatchResult {
            batch_id: None,
            signed: false,
            reused_signed: false,
            signing_in_flight: false,
        });
    };
    sign_chain_key_root_delegation_batch(signing_policy, batch.batch_id, now_ns, signer).await
}

#[expect(
    clippy::too_many_lines,
    reason = "the retry-safe signing transition keeps the state read, callback validation, and persistence update together"
)]
pub(in crate::ops::auth) async fn sign_chain_key_root_delegation_batch<S>(
    signing_policy: &ChainKeySigningPolicy,
    batch_id: [u8; 32],
    now_ns: u64,
    signer: &mut S,
) -> Result<SignNextChainKeyRootDelegationBatchResult, InternalError>
where
    S: ChainKeySigner,
{
    AuthStateOps::prune_chain_key_root_delegation_batches(now_ns);
    let Some(mut batch) = AuthStateOps::chain_key_root_delegation_batch(batch_id) else {
        return Ok(SignNextChainKeyRootDelegationBatchResult {
            batch_id: None,
            signed: false,
            reused_signed: false,
            signing_in_flight: false,
        });
    };
    if now_ns >= batch.header.expires_at_ns {
        return Ok(SignNextChainKeyRootDelegationBatchResult {
            batch_id: None,
            signed: false,
            reused_signed: false,
            signing_in_flight: false,
        });
    }
    match batch.status {
        ChainKeyRootDelegationBatchStatus::Signed
        | ChainKeyRootDelegationBatchStatus::Installing => {
            return Ok(SignNextChainKeyRootDelegationBatchResult {
                batch_id: Some(batch.batch_id),
                signed: false,
                reused_signed: true,
                signing_in_flight: false,
            });
        }
        ChainKeyRootDelegationBatchStatus::Signing => {
            return Ok(SignNextChainKeyRootDelegationBatchResult {
                batch_id: Some(batch.batch_id),
                signed: false,
                reused_signed: false,
                signing_in_flight: true,
            });
        }
        ChainKeyRootDelegationBatchStatus::Prepared => {}
        ChainKeyRootDelegationBatchStatus::FailedRetryable => {
            if batch
                .retry_after_ns
                .is_some_and(|retry_after_ns| now_ns < retry_after_ns)
            {
                return Ok(SignNextChainKeyRootDelegationBatchResult {
                    batch_id: None,
                    signed: false,
                    reused_signed: false,
                    signing_in_flight: false,
                });
            }
        }
        ChainKeyRootDelegationBatchStatus::Installed => {
            return Ok(SignNextChainKeyRootDelegationBatchResult {
                batch_id: None,
                signed: false,
                reused_signed: false,
                signing_in_flight: false,
            });
        }
    }

    batch.status = ChainKeyRootDelegationBatchStatus::Signing;
    batch.retry_after_ns = None;
    batch.failure = None;
    AuthStateOps::upsert_chain_key_root_delegation_batch(batch.clone());

    match sign_chain_key_batch_header(
        SignChainKeyBatchHeaderInput {
            header: &batch.header,
            policy: signing_policy,
        },
        signer,
    )
    .await
    {
        Ok(signature) => {
            let Some(current) = AuthStateOps::chain_key_root_delegation_batch(batch.batch_id)
            else {
                return Ok(SignNextChainKeyRootDelegationBatchResult {
                    batch_id: None,
                    signed: false,
                    reused_signed: false,
                    signing_in_flight: false,
                });
            };
            if current.status != ChainKeyRootDelegationBatchStatus::Signing
                || current.header_hash != batch.header_hash
            {
                return Ok(SignNextChainKeyRootDelegationBatchResult {
                    batch_id: Some(batch.batch_id),
                    signed: false,
                    reused_signed: false,
                    signing_in_flight: false,
                });
            }
            batch = current;
            batch.status = ChainKeyRootDelegationBatchStatus::Signed;
            batch.signature = Some(signature);
            batch.signed_at_ns = Some(now_ns);
            batch.retry_after_ns = None;
            batch.failure = None;
            AuthStateOps::upsert_chain_key_root_delegation_batch(batch.clone());
            Ok(SignNextChainKeyRootDelegationBatchResult {
                batch_id: Some(batch.batch_id),
                signed: true,
                reused_signed: false,
                signing_in_flight: false,
            })
        }
        Err(err) => {
            batch.status = ChainKeyRootDelegationBatchStatus::FailedRetryable;
            batch.retry_after_ns = Some(chain_key_signing_retry_after_ns(
                now_ns,
                batch.header.expires_at_ns,
            ));
            batch.failure = Some(err.to_string());
            AuthStateOps::upsert_chain_key_root_delegation_batch(batch);
            Err(InternalError::ops(
                InternalErrorOrigin::Ops,
                format!("chain-key root delegation batch signing failed: {err}"),
            ))
        }
    }
}

fn next_chain_key_batch_for_signing(now_ns: u64) -> Option<ChainKeyRootDelegationBatch> {
    let mut batches = AuthStateOps::chain_key_root_delegation_batches()
        .into_iter()
        .filter(|batch| now_ns < batch.header.expires_at_ns)
        .filter(|batch| match batch.status {
            ChainKeyRootDelegationBatchStatus::Prepared
            | ChainKeyRootDelegationBatchStatus::Signing
            | ChainKeyRootDelegationBatchStatus::Signed
            | ChainKeyRootDelegationBatchStatus::Installing => true,
            ChainKeyRootDelegationBatchStatus::FailedRetryable => batch
                .retry_after_ns
                .is_none_or(|retry_after_ns| now_ns >= retry_after_ns),
            ChainKeyRootDelegationBatchStatus::Installed => false,
        })
        .collect::<Vec<_>>();
    batches.sort_by(|left, right| {
        left.prepared_at_ns
            .cmp(&right.prepared_at_ns)
            .then_with(|| left.batch_id.cmp(&right.batch_id))
    });
    batches.into_iter().next()
}

fn chain_key_signing_retry_after_ns(now_ns: u64, expires_at_ns: u64) -> u64 {
    let backed_off = now_ns.saturating_add(CHAIN_KEY_SIGNING_RETRY_BACKOFF_NS);
    let last_retryable_ns = expires_at_ns.saturating_sub(1);
    backed_off.min(last_retryable_ns).max(now_ns)
}
