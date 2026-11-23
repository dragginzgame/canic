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
    method: ic_wire::method_id("canic.hub_routes.v2"),
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
    draining: bool,
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
// Values retain (current ordinal, previous ordinal). Equal ordinals mean initial assignment.
type Owners = StableBTreeMap<(u8, [u8; 30]), (u8, u8)>;

struct Index {
    roster: Roster,
    cell: StableCell<Vec<u8>>,
    owners: Owners,
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
        let (slot, _) = i.owners.get(&(
            u8::try_from(n).expect("pool namespace"),
            subject_key(subject),
        ))?;
        Some(i.roster.pools[n].shards[usize::from(slot)].target.clone())
    })
}

/// A failed authorization belongs to the application. Placement errors belong to Canic.
#[derive(Debug, PartialEq, Eq)]
pub enum ReservationError<E> {
    /// The application's check refused the request.
    Authorization(E),
    /// The route state or capacity reply could not establish an assignment.
    Placement(&'static str),
}

/// Reserve the subject's original shard, growing through Root only on exhaustion.
/// The application supplies its fallible authorization check.
/// Canic repeats it after each await before committing an assignment.
/// # Errors
/// Returns the original authorization refusal. Placement failures leave the growth ordinal retryable.
/// # Panics
/// Requires an open index. Conflicting replies or storage failures trap the message.
pub async fn assign_shard<E>(
    pool: &str,
    subject: Principal,
    authorize: impl Fn() -> Result<(), E>,
) -> Result<Placement, ReservationError<E>> {
    reserve(pool, subject, None, authorize).await
}

/// Reserve a destination while preserving the account's active source route.
/// The application fences source writes before copying and verifies destination readiness.
/// # Errors
/// Preserves authorization refusals separately from changed or non-draining source routes.
pub async fn prepare_move<E>(
    pool: &str,
    subject: Principal,
    source: &ShardTarget,
    authorize: impl Fn() -> Result<(), E>,
) -> Result<Placement, ReservationError<E>> {
    reserve(pool, subject, Some(source), authorize).await
}

fn reservation_key(subject: Principal, moving: bool) -> [u8; 30] {
    let mut key = subject_key(subject);
    // Principal lengths occupy 0..=29. The high bit identifies a pending destination.
    if moving {
        key[0] |= 128;
    }
    key
}

enum Reservation {
    Assigned(ShardTarget),
    Grow(Principal, u8),
}

async fn reserve<E>(
    pool: &str,
    subject: Principal,
    source: Option<&ShardTarget>,
    authorize: impl Fn() -> Result<(), E>,
) -> Result<Placement, ReservationError<E>> {
    loop {
        authorize().map_err(ReservationError::Authorization)?;
        let reservation = INDEX.with_borrow_mut(|i| {
            let i = i.as_mut().expect("open routes");
            let root = i.roster.root;
            let Some(n) = i.roster.pool(pool) else {
                if source.is_some() {
                    return Err("unknown source pool");
                }
                return Ok(Reservation::Grow(root, 0));
            };
            let namespace = u8::try_from(n).expect("pool namespace");
            if let Some(source) = source {
                let (slot, previous) = i
                    .owners
                    .get(&(namespace, subject_key(subject)))
                    .ok_or("unknown source route")?;
                let shard = &i.roster.pools[n].shards[usize::from(slot)];
                if slot != previous
                    && &i.roster.pools[n].shards[usize::from(previous)].target == source
                {
                    return Ok(Reservation::Assigned(shard.target.clone()));
                }
                if &shard.target != source || !shard.draining {
                    return Err("source route is not draining");
                }
            }
            let key = (namespace, reservation_key(subject, source.is_some()));
            if let Some((slot, _)) = i.owners.get(&key) {
                return Ok(Reservation::Assigned(
                    i.roster.pools[n].shards[usize::from(slot)].target.clone(),
                ));
            }
            let shards = &mut i.roster.pools[n].shards;
            if let Some((slot, s)) = shards
                .iter_mut()
                .enumerate()
                .find(|(_, s)| !s.draining && s.occupied < s.limit)
            {
                let ordinal = u8::try_from(slot).unwrap();
                let old = i.owners.insert(key, (ordinal, ordinal));
                assert!(old.is_none(), "route changed");
                s.occupied += 1;
                let target = s.target.clone();
                i.cell.set(i.roster.encode());
                Ok(Reservation::Assigned(target))
            } else {
                Ok(Reservation::Grow(
                    root,
                    u8::try_from(shards.len()).expect("shard ordinal"),
                ))
            }
        });
        let (root, ordinal) = match reservation.map_err(ReservationError::Placement)? {
            Reservation::Assigned(target) => {
                return Ok(Placement::Assigned(
                    target.allocation,
                    target.shard.to_text(),
                ));
            }
            Reservation::Grow(root, ordinal) => (root, ordinal),
        };
        let reply = grow_shard(root, pool, ordinal).await;
        authorize().map_err(ReservationError::Authorization)?;
        let (result, limit) =
            reply.map_err(|_| ReservationError::Placement("shard capacity reply unavailable"))?;
        let Some((allocation, shard)) = result
            .assigned_target()
            .map_err(ReservationError::Placement)?
        else {
            // A concurrent registration may have committed this subject while suspended.
            if INDEX.with_borrow(|index| {
                let i = index.as_ref().expect("open routes");
                i.roster.pool(pool).is_some_and(|n| {
                    i.owners.contains_key(&(
                        u8::try_from(n).unwrap(),
                        reservation_key(subject, source.is_some()),
                    ))
                })
            }) {
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
                draining: false,
            });
            i.cell.set(i.roster.encode());
        }
    });
}

/// Commit the exact prepared move after the app has verified the destination.
/// Replaying a completed move acknowledges the same binding.
/// # Errors
/// Refuses a different active source, unreserved destination or changed allocation.
/// # Panics
/// Requires an opened route index with consistent retained occupancy.
pub fn confirm_move(
    pool: &str,
    subject: Principal,
    source: &ShardTarget,
    destination: &ShardTarget,
) -> Result<(), &'static str> {
    INDEX.with_borrow_mut(|index| {
        let i = index.as_mut().expect("open routes");
        let n = i.roster.pool(pool).ok_or("unknown pool")?;
        let shards = &mut i.roster.pools[n].shards;
        let from = shards
            .iter()
            .position(|s| &s.target == source && s.draining)
            .ok_or("invalid draining source")?;
        let to = shards
            .iter()
            .position(|s| &s.target == destination)
            .ok_or("unknown destination")?;
        if from == to {
            return Err("source equals destination");
        }
        let namespace = u8::try_from(n).expect("pool namespace");
        let key = (namespace, subject_key(subject));
        let pending = (namespace, reservation_key(subject, true));
        let (current, previous) = i.owners.get(&key).ok_or("unknown subject")?;
        let current = usize::from(current);
        if current == to && usize::from(previous) == from {
            return Ok(());
        }
        if current != from || i.owners.get(&pending).map(|(slot, _)| usize::from(slot)) != Some(to)
        {
            return Err("route reservation changed");
        }
        i.owners.remove(&pending);
        i.owners.insert(
            key,
            (u8::try_from(to).unwrap(), u8::try_from(from).unwrap()),
        );
        shards[from].occupied -= 1;
        i.cell.set(i.roster.encode());
        Ok(())
    })
}

/// Observe or permanently fence an exact allocation on request from its retained Root.
/// # Errors
/// Refuses foreign callers and unknown pool or allocation bindings.
/// # Panics
/// Requires an open hub route index.
pub fn shard_status(
    pool: &str,
    target: &ShardTarget,
    drain: bool,
) -> Result<super::ShardStatus, String> {
    status(ic_platform_runtime::system::caller(), pool, target, drain)
}

fn status(
    caller: Principal,
    pool: &str,
    target: &ShardTarget,
    drain: bool,
) -> Result<super::ShardStatus, String> {
    INDEX.with_borrow_mut(|index| {
        let i = index.as_mut().expect("open routes");
        if i.roster.root != caller {
            return Err("routing Root required".into());
        }
        let n = i.roster.pool(pool).ok_or("unknown shard pool")?;
        let shard = i.roster.pools[n]
            .shards
            .iter_mut()
            .find(|s| &s.target == target)
            .ok_or("unknown shard generation")?;
        let changed = drain && !shard.draining;
        shard.draining |= drain;
        let status = super::ShardStatus {
            target: target.clone(),
            draining: shard.draining,
            live_placements: shard.occupied,
        };
        if changed {
            i.cell.set(i.roster.encode());
        }
        Ok(status)
    })
}

/// Mount Root's route occupancy and drain operation in a hub that opens the index.
#[macro_export]
macro_rules! mount_hub_routes {
    () => {
        #[$crate::update(contract = $crate::sharding::HubShardStatus)]
        fn hub_shard_status(
            (pool, target, drain): (String, $crate::sharding::ShardTarget, bool),
        ) -> Result<$crate::sharding::ShardStatus, String> {
            $crate::sharding::routes::shard_status(&pool, &target, drain)
        }
    };
}
