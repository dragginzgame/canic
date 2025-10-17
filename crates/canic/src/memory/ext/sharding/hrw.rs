use candid::{CandidType, Principal};
use canic::utils::hash::xxhash_256::*;
use serde::{Deserialize, Serialize};

/// A physical shard canister and how many virtual nodes it owns.
#[derive(Clone, Debug, CandidType, Deserialize, Serialize)]
pub struct ShardSpec {
    pub canister: Principal,
    pub vnodes: u32, // e.g., 64 or 128 per shard; tune for smoothness
}

/// Routing table the hub consults. Keep it small and versioned.
#[derive(CandidType, Clone, Debug, Default, Deserialize, Serialize)]
pub struct RoutingTable {
    pub version: u32,
    pub salt: [u8; 32], // changes force a full rehash (rare)
    pub shards: Vec<ShardSpec>,
}

/// Hub state (keep in stable memory via your usual pattern)
#[derive(CandidType, Clone, Debug, Deserialize, Serialize, Default)]
pub struct DiscoveryState {
    pub routing: RoutingTable,
}

fn hrw_score(salt: &[u8; 32], shard_canister: &Principal, vnode_idx: u32, key: &[u8]) -> u64 {
    // Hash(salt || shard || vnode || key) → take high 8 bytes as u64
    let mut buf = Vec::with_capacity(32 + shard_canister.as_slice().len() + 4 + key.len());
    buf.extend_from_slice(salt);
    buf.extend_from_slice(shard_canister.as_slice());
    buf.extend_from_slice(&vnode_idx.to_be_bytes());
    buf.extend_from_slice(key);
    let h = blake2b_256(&buf);

    u64::from_be_bytes([h[0], h[1], h[2], h[3], h[4], h[5], h[6], h[7]])
}

/// Pick the single best shard for a principal.
fn pick_shard(rt: &RoutingTable, principal: Principal) -> Principal {
    let key = principal.as_slice();
    let mut best = (0u64, Principal::anonymous());
    for shard in &rt.shards {
        for v in 0..shard.vnodes {
            let s = hrw_score(&rt.salt, &shard.canister, v, key);
            if s > best.0 {
                best = (s, shard.canister);
            }
        }
    }
    best.1
}

/// Optionally: top-R (e.g., for replication / dual-write during migrations).
fn pick_top_r(rt: &RoutingTable, principal: Principal, r: usize) -> Vec<Principal> {
    let key = principal.as_slice();
    let mut scores: Vec<(u64, Principal)> = Vec::with_capacity(rt.shards.len() * 2);
    for shard in &rt.shards {
        // One score per shard (max over its vnodes) is often enough; or keep per-vnode.
        let mut best = 0u64;
        for v in 0..shard.vnodes {
            let s = hrw_score(&rt.salt, &shard.canister, v, key);
            if s > best {
                best = s;
            }
        }
        scores.push((best, shard.canister));
    }
    scores.sort_unstable_by(|a, b| b.0.cmp(&a.0));
    scores.into_iter().take(r).map(|(_, p)| p).collect()
}
