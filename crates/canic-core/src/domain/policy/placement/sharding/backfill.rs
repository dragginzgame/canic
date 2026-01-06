//! Slot backfill logic for sharding policy.
//!
//! This module implements *planning-only* slot assignment used during
//! shard placement decisions. It must never be persisted or exposed
//! outside the policy layer.

use crate::{cdk::candid::Principal, ops::storage::placement::sharding::ShardEntry};
use std::collections::{BTreeMap, BTreeSet};

///
/// SlotBackfillPlan
/// (pure planning)
///

pub(super) struct SlotBackfillPlan {
    /// Effective slot mapping for shards in the pool (explicit or simulated).
    pub slots: BTreeMap<Principal, u32>,

    /// Slots considered occupied after deterministic backfill simulation.
    pub occupied: BTreeSet<u32>,
}

pub(super) fn plan_slot_backfill(
    pool: &str,
    view: &[(Principal, ShardEntry)],
    max_slots: u32,
) -> SlotBackfillPlan {
    let mut entries: Vec<(Principal, ShardEntry)> = view
        .iter()
        .filter(|(_, entry)| entry.pool == pool)
        .map(|(pid, entry)| (*pid, entry.clone()))
        .collect();

    entries.sort_by_key(|(pid, _)| *pid);

    let mut slots = BTreeMap::<Principal, u32>::new();
    let mut occupied = BTreeSet::<u32>::new();

    for (pid, entry) in &entries {
        if entry_has_assigned_slot(entry) {
            slots.insert(*pid, entry.slot);
            occupied.insert(entry.slot);
        }
    }

    if max_slots == 0 {
        return SlotBackfillPlan { slots, occupied };
    }

    let available: Vec<u32> = (0..max_slots)
        .filter(|slot| !occupied.contains(slot))
        .collect();

    if available.is_empty() {
        return SlotBackfillPlan { slots, occupied };
    }

    let mut idx = 0usize;
    for (pid, entry) in &entries {
        if entry_has_assigned_slot(entry) {
            continue;
        }

        // NOTE: Backfill simulates slot assignment for existing shards only.
        // Policy enforcement happens later; this function is purely positional.
        if idx >= available.len() {
            break;
        }

        let slot = available[idx];
        idx += 1;
        slots.insert(*pid, slot);
        occupied.insert(slot);
    }

    SlotBackfillPlan { slots, occupied }
}

const fn entry_has_assigned_slot(entry: &ShardEntry) -> bool {
    entry.slot != ShardEntry::UNASSIGNED_SLOT
}
