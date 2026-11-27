//! Hub-local durable placement for named shard pools.
#[cfg(test)]
mod tests;

use super::{Placement, ShardTarget, grow_shard};
pub use ic_platform_types::Principal;
use ic_storage::{
    DefaultMemoryImpl, MemoryRuntime, StableKey,
    adapter::{StableBTreeMap, StableCell},
};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;

const FRAME: ic_wire::Encoding = ic_wire::Encoding::Frame {
    method: ic_wire::method_id("canic.hub.route_roster.v1"),
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
    limit: u32,
    shards: Vec<Shard>,
}
#[derive(Serialize, Deserialize)]
struct Shard {
    target: ShardTarget,
    occupied: u32,
    draining: bool,
}
#[derive(Serialize, Deserialize)]
struct Roster {
    operator: Option<Principal>,
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
    install_root: Option<(Principal, Option<Principal>)>,
) {
    let cell = crate::storage::cell(
        runtime,
        roster_key,
        LIMITS.bytes,
        install_root.map(|(root, operator)| {
            assert!(
                operator.is_none_or(
                    |p| p != Principal::anonymous() && p != Principal::management_canister()
                ),
                "invalid hub operator"
            );
            Roster {
                operator,
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
        .flat_map(|p| p.shards.iter().map(move |s| (p.limit, s)))
        .map(|(limit, s)| {
            assert!(limit > 0 && s.occupied <= limit);
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

/// Root retained when this hub was installed.
/// # Panics
/// Requires an opened route index.
#[must_use]
pub fn root() -> Principal {
    INDEX.with_borrow(|index| index.as_ref().expect("open routes").roster.root)
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
                return Err("shard routing policy missing");
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
            let limit = i.roster.pools[n].limit;
            let shards = &mut i.roster.pools[n].shards;
            if let Some((slot, s)) = shards
                .iter_mut()
                .enumerate()
                .find(|(_, s)| !s.draining && s.occupied < limit)
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
        let result =
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
        record_shard(pool, ordinal, target);
    }
}

/// Approve one pool's immutable routing limit on this hub.
#[doc(hidden)]
pub fn configure_shard_routes(
    caller: Principal,
    pool: &str,
    limit: u32,
) -> Result<(), super::ShardError> {
    use super::ShardError;
    INDEX.with_borrow_mut(|index| {
        let index = index.as_mut().expect("open routes");
        if index.roster.operator != Some(caller) {
            return Err(ShardError::Unauthorized);
        }
        if pool.is_empty() || pool.len() > 64 || !(1..=super::MAX_KEYS_PER_SHARD).contains(&limit) {
            return Err(ShardError::InvalidPolicy);
        }
        if let Some(n) = index.roster.pool(pool) {
            return (index.roster.pools[n].limit == limit)
                .then_some(())
                .ok_or(ShardError::InvalidPolicy);
        }
        if index.roster.pools.len() >= 32 {
            return Err(ShardError::InvalidPolicy);
        }
        index.roster.pools.push(Pool {
            name: pool.to_owned(),
            limit,
            shards: vec![],
        });
        index.cell.set(index.roster.encode());
        Ok(())
    })
}

fn record_shard(pool: &str, ordinal: u8, target: ShardTarget) {
    INDEX.with_borrow_mut(|i| {
        let i = i.as_mut().expect("open routes");
        let n = i.roster.pool(pool).expect("configured pool");
        let shards = &mut i.roster.pools[n].shards;
        if let Some(old) = shards.get(usize::from(ordinal)) {
            assert_eq!(old.target, target, "capacity binding changed");
        } else {
            assert_eq!(usize::from(ordinal), shards.len());
            assert!(!shards.iter().any(|s| s.target.shard == target.shard));
            shards.push(Shard {
                target,
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

/// Release an account after the application has removed its data from a draining shard.
/// # Errors
/// Refuses a changed generation, active shard, different route or unfinished move.
/// # Panics
/// Requires an opened route index.
pub fn release(pool: &str, subject: Principal, target: &ShardTarget) -> Result<(), &'static str> {
    INDEX.with_borrow_mut(|index| {
        let i = index.as_mut().expect("open routes");
        let n = i.roster.pool(pool).ok_or("unknown pool")?;
        let shards = &mut i.roster.pools[n].shards;
        let slot = shards
            .iter()
            .position(|s| &s.target == target)
            .ok_or("assignment changed")?;
        if !shards[slot].draining {
            return Err("shard is not draining");
        }
        let namespace = u8::try_from(n).expect("pool namespace");
        if i.owners
            .contains_key(&(namespace, reservation_key(subject, true)))
        {
            return Err("move pending");
        }
        let key = (namespace, subject_key(subject));
        if let Some((current, _)) = i.owners.get(&key) {
            if usize::from(current) != slot {
                return Err("assignment changed");
            }
            i.owners.remove(&key);
            shards[slot].occupied -= 1;
            i.cell.set(i.roster.encode());
        }
        Ok(())
    })
}

/// Read local occupancy as the bound hub operator.
pub fn observe_shard(principal: Principal) -> Result<super::ShardStatus, super::ShardError> {
    observe(ic_platform_runtime::system::caller(), principal)
}

fn observe(
    caller: Principal,
    principal: Principal,
) -> Result<super::ShardStatus, super::ShardError> {
    INDEX.with_borrow(|index| {
        let index = index.as_ref().expect("open routes");
        if index.roster.operator != Some(caller) {
            return Err(super::ShardError::Unauthorized);
        }
        let mut shards = index
            .roster
            .pools
            .iter()
            .flat_map(|pool| &pool.shards)
            .filter(|shard| shard.target.shard == principal);
        let shard = shards.next().ok_or(super::ShardError::UnknownShard)?;
        if shards.next().is_some() {
            return Err(super::ShardError::GenerationChanged);
        }
        Ok(shard.status())
    })
}

impl Shard {
    fn status(&self) -> super::ShardStatus {
        super::ShardStatus {
            target: self.target.clone(),
            draining: self.draining,
            live_placements: self.occupied,
        }
    }
}

fn fence(caller: Principal, target: &ShardTarget) -> Result<(Principal, u32), super::ShardError> {
    INDEX.with_borrow_mut(|index| {
        let index = index.as_mut().expect("open routes");
        if index.roster.operator != Some(caller) {
            return Err(super::ShardError::Unauthorized);
        }
        let shard = index
            .roster
            .pools
            .iter_mut()
            .flat_map(|pool| &mut pool.shards)
            .find(|shard| shard.target == *target)
            .ok_or(super::ShardError::GenerationChanged)?;
        let occupied = shard.occupied;
        if !shard.draining {
            shard.draining = true;
            index.cell.set(index.roster.encode());
        }
        Ok((index.roster.root, occupied))
    })
}

/// Fence the exact generation locally. Release it to its creator once evacuated.
/// # Errors
/// A lost release acknowledgement is recovered by repeating the same drain.
/// # Panics
/// Requires an open hub.
pub async fn begin_shard_drain(target: ShardTarget) -> Result<u32, super::ShardError> {
    let (root, occupied) = fence(ic_platform_runtime::system::caller(), &target)?;
    if occupied == 0 {
        crate::method::call::<super::ReleaseChildAllocation>(
            root,
            10,
            &(target.shard, target.allocation),
        )
        .await
        .map_err(|_| super::ShardError::ReleaseUnconfirmed)?
        .map_err(|reason| match reason {
            super::ReleaseError::Unauthorized => super::ShardError::Unauthorized,
            super::ReleaseError::UnknownAllocation => super::ShardError::GenerationChanged,
        })?;
    }
    Ok(occupied)
}

/// Mount direct operator inspection and drain on the owning hub.
#[macro_export]
macro_rules! mount_hub_routes {
    () => {
        #[$crate::__canic_update(contract = $crate::sharding::ConfigureShardRoutes)]
        fn configure_shard_routes(
            input: (String, u32),
        ) -> Result<(), $crate::sharding::ShardError> {
            $crate::sharding::routes::configure_shard_routes(
                $crate::__private::runtime::system::caller(),
                &input.0,
                input.1,
            )
        }
        #[$crate::__canic_update(contract = $crate::sharding::ObserveShard)]
        fn observe_shard(
            shard: $crate::sharding::routes::Principal,
        ) -> Result<$crate::sharding::ShardStatus, $crate::sharding::ShardError> {
            $crate::sharding::routes::observe_shard(shard)
        }
        #[$crate::__canic_update(contract = $crate::sharding::BeginShardDrain)]
        async fn begin_shard_drain(
            target: $crate::sharding::ShardTarget,
        ) -> Result<u32, $crate::sharding::ShardError> {
            $crate::sharding::routes::begin_shard_drain(target).await
        }
    };
}
