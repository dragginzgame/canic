use super::DelegatedTokenUseRecord;

///
/// DelegatedTokenUseConsumeResult
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DelegatedTokenUseConsumeResult {
    Consumed,
    Replayed,
    CapacityReached,
}

// Atomically record one delegated token use after pruning expired entries.
pub(super) fn consume_delegated_token_use(
    uses: &mut Vec<DelegatedTokenUseRecord>,
    token_use: DelegatedTokenUseRecord,
    now_secs: u64,
    capacity: usize,
) -> DelegatedTokenUseConsumeResult {
    prune_expired_delegated_token_uses(uses, now_secs);

    if uses
        .iter()
        .any(|entry| same_token_use_key(entry, &token_use))
    {
        return DelegatedTokenUseConsumeResult::Replayed;
    }

    if uses.len() >= capacity {
        return DelegatedTokenUseConsumeResult::CapacityReached;
    }

    uses.push(token_use);
    DelegatedTokenUseConsumeResult::Consumed
}

// Prune expired delegated token uses and return the removal count.
pub(super) fn prune_expired_delegated_token_uses(
    uses: &mut Vec<DelegatedTokenUseRecord>,
    now_secs: u64,
) -> usize {
    let before = uses.len();
    uses.retain(|entry| !delegated_token_use_expired(entry.expires_at, now_secs));
    before.saturating_sub(uses.len())
}

// Treat consumed-token state as expired at the same exclusive boundary as tokens.
const fn delegated_token_use_expired(expires_at: u64, now_secs: u64) -> bool {
    now_secs >= expires_at
}

fn same_token_use_key(left: &DelegatedTokenUseRecord, right: &DelegatedTokenUseRecord) -> bool {
    left.issuer_shard_pid == right.issuer_shard_pid
        && left.subject == right.subject
        && left.cert_hash == right.cert_hash
        && left.nonce == right.nonce
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cdk::types::Principal;

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn token_use(nonce: u8, expires_at: u64) -> DelegatedTokenUseRecord {
        DelegatedTokenUseRecord {
            issuer_shard_pid: p(1),
            subject: p(2),
            cert_hash: [3; 32],
            nonce: [nonce; 16],
            used_at: 10,
            expires_at,
        }
    }

    #[test]
    fn consume_rejects_active_replay() {
        let mut uses = Vec::new();

        assert_eq!(
            consume_delegated_token_use(&mut uses, token_use(7, 20), 10, 4),
            DelegatedTokenUseConsumeResult::Consumed
        );
        assert_eq!(
            consume_delegated_token_use(&mut uses, token_use(7, 20), 11, 4),
            DelegatedTokenUseConsumeResult::Replayed
        );
    }

    #[test]
    fn consume_allows_nonce_after_expiry_prune() {
        let mut uses = vec![token_use(7, 20)];

        assert_eq!(
            consume_delegated_token_use(&mut uses, token_use(7, 30), 20, 4),
            DelegatedTokenUseConsumeResult::Consumed
        );
        assert_eq!(uses.len(), 1);
        assert_eq!(uses[0].expires_at, 30);
    }

    #[test]
    fn consume_fails_closed_at_capacity() {
        let mut uses = vec![token_use(1, 20)];

        assert_eq!(
            consume_delegated_token_use(&mut uses, token_use(2, 20), 10, 1),
            DelegatedTokenUseConsumeResult::CapacityReached
        );
        assert_eq!(uses.len(), 1);
    }
}
