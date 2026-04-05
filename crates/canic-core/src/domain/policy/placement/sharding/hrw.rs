//! HRW (Highest Random Weight) shard selection.

use crate::cdk::candid::Principal;
use sha2::{Digest, Sha256};

///
/// HrwSelector
/// HRW-based shard selector
///

pub struct HrwSelector;

impl HrwSelector {
    #[must_use]
    pub(crate) fn select(partition_key: &str, shards: &[Principal]) -> Option<Principal> {
        if shards.is_empty() {
            return None;
        }

        let mut best_score = 0u64;
        let mut best_shard = shards[0];

        for &shard in shards {
            let score = Self::hrw_score(partition_key, &shard);
            if score > best_score {
                best_score = score;
                best_shard = shard;
            }
        }

        Some(best_shard)
    }

    #[must_use]
    pub(crate) fn select_from_slots(pool: &str, partition_key: &str, slots: &[u32]) -> Option<u32> {
        if slots.is_empty() {
            return None;
        }

        let mut best_score = 0u64;
        let mut best_slot = slots[0];

        for &slot in slots {
            let score = Self::hrw_score_slot(pool, partition_key, slot);
            if score > best_score {
                best_score = score;
                best_slot = slot;
            }
        }

        Some(best_slot)
    }

    fn hrw_score(partition_key: &str, shard: &Principal) -> u64 {
        let mut hasher = Sha256::new();
        hasher.update(partition_key.as_bytes());
        hasher.update(shard.as_slice());
        score_from_digest(hasher.finalize())
    }

    fn hrw_score_slot(pool: &str, partition_key: &str, slot: u32) -> u64 {
        let mut hasher = Sha256::new();
        hasher.update(pool.as_bytes());
        hasher.update([0xFF]);
        hasher.update(partition_key.as_bytes());
        hasher.update(slot.to_le_bytes());
        score_from_digest(hasher.finalize())
    }
}

fn score_from_digest(digest: impl AsRef<[u8]>) -> u64 {
    let bytes: [u8; 8] = digest.as_ref()[..8]
        .try_into()
        .expect("sha256 digest prefix must fit into u64 bytes");
    u64::from_be_bytes(bytes)
}
