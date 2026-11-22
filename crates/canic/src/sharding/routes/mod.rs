//! Hub-local durable placement for named shard pools.
#[cfg(test)]
mod tests;

use super::{Placement, ShardTarget, grow_shard};
use ic_platform_types::Principal;
use ic_storage::{
    DefaultMemoryImpl, MemoryRuntime, StableKey,
    adapter::{StableBTreeMap, StableCell},
};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;

const FRAME: ic_wire::Encoding = ic_wire::Encoding::Frame {
    method: ic_wire::method_id("canic.hub_routes.v1"),
    direction: ic_wire::Direction::Response,
};
const LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 16384,
    allocation: 65536,
    nodes: 4096,
    depth: 16,
};

#[derive(Serialize, Deserialize)]
struct Pool {
    name: String,
    shards: Vec<Shard>,
}
#[derive(Serialize, Deserialize)]
struct Shard {
    target: ShardTarget,
    limit: u32,
    occupied: u32,
}
#[derive(Serialize, Deserialize)]
struct Roster {
    root: Principal,
    pools: Vec<Pool>,
}
impl Roster {
    fn encode(&self) -> Vec<u8> {
        ic_wire::encode(self, FRAME, LIMITS).expect("bounded shard roster")
    }
    fn pool(&self, name: &str) -> Option<usize> {
        self.pools.iter().position(|p| p.name == name)
    }
}
struct Index {
    roster: Roster,
    cell: StableCell<Vec<u8>>,
    owners: StableBTreeMap<(u8, [u8; 30]), u8>,
}
thread_local! {
    static INDEX: RefCell<Option<Index>> = const { RefCell::new(None) };
}

// Include the length so two principals differing by trailing zeroes stay distinct.
fn subject_key(subject: Principal) -> [u8; 30] {
    let bytes = subject.as_slice();
    let mut key = [0; 30];
    key[0] = u8::try_from(bytes.len()).expect("principal length");
    key[1..=bytes.len()].copy_from_slice(bytes);
    key
}

/// Open the Canic route index on two distinct leases from the hub's Storage bootstrap.
/// On install supply its routing Root. On upgrade pass `None` to reopen the retained binding.
/// # Panics
/// Refuses missing, foreign or inconsistent retained storage.
pub fn open(
    runtime: &MemoryRuntime<DefaultMemoryImpl>,
    roster_key: &str,
    owners_key: &str,
    install_root: Option<Principal>,
) {
    let cell = crate::storage::cell(
        runtime,
        roster_key,
        LIMITS.bytes,
        install_root.map(|root| {
            Roster {
                root,
                pools: vec![],
            }
            .encode()
        }),
    );
    let roster: Roster = ic_wire::decode(cell.get(), FRAME, LIMITS).expect("retained shard roster");
    let lease = runtime
        .open_memory(&StableKey::parse(owners_key).expect("route index key"))
        .expect("route index lease");
    let owners = if install_root.is_some() {
        StableBTreeMap::create(lease)
    } else {
        StableBTreeMap::load(lease)
    };
    let occupied: u64 = roster
        .pools
        .iter()
        .flat_map(|p| &p.shards)
        .map(|s| {
            assert!(s.limit > 0 && s.occupied <= s.limit);
            u64::from(s.occupied)
        })
        .sum();
    assert_eq!(owners.len(), occupied, "retained route count");
    INDEX.set(Some(Index {
        roster,
        cell,
        owners,
    }));
}

/// Read an existing assignment locally, including a reservation whose application write is pending.
/// # Panics
/// Requires an opened route index.
#[must_use]
pub fn get_shard(pool: &str, subject: Principal) -> Option<ShardTarget> {
    INDEX.with_borrow(|i| {
        let i = i.as_ref().expect("open routes");
        let n = i.roster.pool(pool)?;
        let slot = i.owners.get(&(
            u8::try_from(n).expect("pool namespace"),
            subject_key(subject),
        ))?;
        Some(i.roster.pools[n].shards[usize::from(slot)].target.clone())
    })
}

/// Reserve the subject's original shard, growing through Root only on exhaustion.
/// The application supplies its authorization check, which must trap on refusal.
/// Canic repeats it after each await before committing an assignment.
/// # Errors
/// Transport or invalid replies leave the original growth ordinal retryable.
/// # Panics
/// Requires an open index. Conflicting replies or storage failures trap the message.
pub async fn assign_shard(
    pool: &str,
    subject: Principal,
    authorize: impl Fn(),
) -> Result<Placement, &'static str> {
    loop {
        authorize();
        if let Some(target) = get_shard(pool, subject) {
            return Ok(Placement::Assigned(
                target.allocation,
                target.shard.to_text(),
            ));
        }
        let growth = INDEX.with_borrow_mut(|i| {
            let i = i.as_mut().expect("open routes");
            let root = i.roster.root;
            let Some(n) = i.roster.pool(pool) else {
                return Some((root, 0));
            };
            let shards = &mut i.roster.pools[n].shards;
            if let Some((slot, s)) = shards
                .iter_mut()
                .enumerate()
                .find(|(_, s)| s.occupied < s.limit)
            {
                let old = i.owners.insert(
                    (u8::try_from(n).unwrap(), subject_key(subject)),
                    u8::try_from(slot).unwrap(),
                );
                assert!(old.is_none(), "route changed");
                s.occupied += 1;
                i.cell.set(i.roster.encode());
                None
            } else {
                Some((root, u8::try_from(shards.len()).expect("shard ordinal")))
            }
        });
        let Some((root, ordinal)) = growth else {
            continue;
        };
        let reply = grow_shard(root, pool, ordinal).await;
        authorize();
        let (result, limit) = reply.map_err(|_| "shard capacity reply unavailable")?;
        let Some((allocation, shard)) = result.assigned_target()? else {
            // A concurrent registration may have committed this subject while suspended.
            if get_shard(pool, subject).is_some() {
                continue;
            }
            return Ok(result);
        };
        let target = ShardTarget {
            allocation: allocation.clone(),
            shard,
        };
        record_shard(pool, ordinal, target, limit);
    }
}

fn record_shard(pool: &str, ordinal: u8, target: ShardTarget, limit: u32) {
    INDEX.with_borrow_mut(|i| {
        let i = i.as_mut().expect("open routes");
        let n = i.roster.pool(pool).unwrap_or_else(|| {
            i.roster.pools.push(Pool {
                name: pool.to_owned(),
                shards: vec![],
            });
            i.roster.pools.len() - 1
        });
        let shards = &mut i.roster.pools[n].shards;
        if let Some(old) = shards.get(usize::from(ordinal)) {
            assert_eq!(
                (&old.target, old.limit),
                (&target, limit),
                "capacity binding changed"
            );
        } else {
            assert_eq!(usize::from(ordinal), shards.len());
            assert!(limit > 0 && !shards.iter().any(|s| s.target.shard == target.shard));
            shards.push(Shard {
                target,
                limit,
                occupied: 0,
            });
            i.cell.set(i.roster.encode());
        }
    });
}
