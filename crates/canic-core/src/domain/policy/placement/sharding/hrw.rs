//! HRW (Highest Random Weight) shard selection.
//!
//! Deterministic, stateless, and pure.
//! Given a tenant and a set of shard principals, it selects
//! the shard with the highest weighted hash.
//!
//! Used by [`ShardAllocator`] or [`ShardingPolicy`] when assigning tenants.

use crate::utils::hash::hash_u64;
use candid::Principal;

///
/// HrwSelector
/// HRW-based shard selector
///

pub struct HrwSelector;

impl HrwSelector {
    /// Pick the shard with the highest HRW score for this tenant.
    #[must_use]
    pub(crate) fn select(tenant: &str, shards: &[Principal]) -> Option<Principal> {
        if shards.is_empty() {
            return None;
        }

        let mut best_score = 0u64;
        let mut best_shard = shards[0];

        for &shard in shards {
            let score = Self::hrw_score(tenant, &shard);
            if score > best_score {
                best_score = score;
                best_shard = shard;
            }
        }

        Some(best_shard)
    }

    /// Pick the highest-scoring slot from a provided list.
    #[must_use]
    pub(crate) fn select_from_slots(pool: &str, tenant: &str, slots: &[u32]) -> Option<u32> {
        if slots.is_empty() {
            return None;
        }

        let mut best_score = 0u64;
        let mut best_slot = slots[0];

        for &slot in slots {
            let score = Self::hrw_score_slot(pool, tenant, slot);
            if score > best_score {
                best_score = score;
                best_slot = slot;
            }
        }

        Some(best_slot)
    }

    /// Deterministic HRW score = hash(tenant || shard).
    fn hrw_score(tenant: &str, shard: &Principal) -> u64 {
        let mut bytes = Vec::with_capacity(tenant.len() + shard.as_slice().len());
        bytes.extend_from_slice(tenant.as_bytes());
        bytes.extend_from_slice(shard.as_slice());

        hash_u64(&bytes)
    }

    fn hrw_score_slot(pool: &str, tenant: &str, slot: u32) -> u64 {
        let mut bytes =
            Vec::with_capacity(pool.len() + tenant.len() + std::mem::size_of::<u32>() + 1);
        bytes.extend_from_slice(pool.as_bytes());
        bytes.push(0xFF); // delimiter to avoid accidental overlaps
        bytes.extend_from_slice(tenant.as_bytes());
        bytes.extend_from_slice(&slot.to_le_bytes());

        hash_u64(&bytes)
    }
}

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::*;
    use candid::Principal;

    #[test]
    fn selects_consistently() {
        let tenant = "hello";
        let shards = vec![
            Principal::from_text("aaaaa-aa").unwrap(),
            Principal::from_text("ryjl3-tyaaa-aaaaa-aaaba-cai").unwrap(),
        ];
        let s1 = HrwSelector::select(tenant, &shards).unwrap();
        let s2 = HrwSelector::select(tenant, &shards).unwrap();
        assert_eq!(s1, s2);
    }
}
