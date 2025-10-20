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
    pub fn select(tenant: &Principal, shards: &[Principal]) -> Option<Principal> {
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

    /// Deterministic HRW score = hash(tenant || shard).
    #[inline]
    fn hrw_score(tenant: &Principal, shard: &Principal) -> u64 {
        let mut bytes = Vec::with_capacity(tenant.as_slice().len() + shard.as_slice().len());
        bytes.extend_from_slice(tenant.as_slice());
        bytes.extend_from_slice(shard.as_slice());

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
        let tenant = Principal::anonymous();
        let shards = vec![
            Principal::from_text("aaaaa-aa").unwrap(),
            Principal::from_text("ryjl3-tyaaa-aaaaa-aaaba-cai").unwrap(),
        ];
        let s1 = HrwSelector::select(&tenant, &shards).unwrap();
        let s2 = HrwSelector::select(&tenant, &shards).unwrap();
        assert_eq!(s1, s2);
    }
}
