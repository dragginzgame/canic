//! HRW (Highest Random Weight) shard selection.

use canic_core::cdk::candid::Principal;
use xxhash_rust::xxh3::xxh3_64;

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
        let mut bytes = Vec::with_capacity(partition_key.len() + shard.as_slice().len());
        bytes.extend_from_slice(partition_key.as_bytes());
        bytes.extend_from_slice(shard.as_slice());

        hash_u64(&bytes)
    }

    fn hrw_score_slot(pool: &str, partition_key: &str, slot: u32) -> u64 {
        let mut bytes =
            Vec::with_capacity(pool.len() + partition_key.len() + std::mem::size_of::<u32>() + 1);
        bytes.extend_from_slice(pool.as_bytes());
        bytes.push(0xFF);
        bytes.extend_from_slice(partition_key.as_bytes());
        bytes.extend_from_slice(&slot.to_le_bytes());

        hash_u64(&bytes)
    }
}

fn hash_u64(bytes: &[u8]) -> u64 {
    xxh3_64(bytes)
}
