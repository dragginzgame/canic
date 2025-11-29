//! HRW (Highest Random Weight) shard selection.
//!
//! Deterministic, stateless, and pure.
//! Given a tenant and a set of shard principals, it selects
//! the shard with the highest weighted hash.
//!
//! Used by [`ShardAllocator`] or [`ShardingPolicyOps`] when assigning tenants.

use crate::utils::hash::hash_u64;
use candid::Principal;

/// HRW-based shard selector.
pub struct HrwSelector;

impl HrwSelector {
    /// Pick the shard with the highest HRW score for this tenant.
    #[must_use]
    pub fn select(tenant: &str, shards: &[Principal]) -> Option<Principal> {
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

    /// Pick the slot index with the highest HRW score for this tenant.
    #[must_use]
    pub fn select_slot(pool: &str, tenant: &str, slots: u32) -> Option<u32> {
        if slots == 0 {
            return None;
        }

        let mut best_score = 0u64;
        let mut best_slot = 0u32;

        for slot in 0..slots {
            let score = Self::hrw_score_slot(pool, tenant, slot);
            if score > best_score {
                best_score = score;
                best_slot = slot;
            }
        }

        Some(best_slot)
    }

    /// Deterministic HRW score = hash(tenant || shard).
    #[inline]
    fn hrw_score(tenant: &str, shard: &Principal) -> u64 {
        let mut bytes = Vec::with_capacity(tenant.len() + shard.as_slice().len());
        bytes.extend_from_slice(tenant.as_bytes());
        bytes.extend_from_slice(shard.as_slice());

        hash_u64(&bytes)
    }

    #[inline]
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

    #[test]
    fn select_slot_is_deterministic() {
        let tenant = "tenant-123";
        let pool = "primary";
        let slot = HrwSelector::select_slot(pool, tenant, 4).unwrap();
        let slot_again = HrwSelector::select_slot(pool, tenant, 4).unwrap();
        assert_eq!(slot, slot_again);
    }
}
