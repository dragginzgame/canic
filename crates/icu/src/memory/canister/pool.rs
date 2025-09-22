use crate::{
    Log,
    cdk::{
        futures::spawn,
        structures::{BTreeMap, DefaultMemoryImpl, Memory, memory::VirtualMemory},
        timers::{TimerId, clear_timer, set_timer, set_timer_interval},
    },
    config::Config,
    icu_register_memory, impl_storable_candid_unbounded, log,
    memory::CANISTER_POOL_MEMORY_ID,
    ops::pool::create_pool_canister,
    types::Cycles,
    utils::time::now_secs,
};
use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;

//
// CANISTER_POOL
//

thread_local! {
    static CANISTER_POOL: RefCell<CanisterPoolCore<VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(CanisterPoolCore::new(BTreeMap::init(
            icu_register_memory!(CANISTER_POOL_MEMORY_ID),
        )));

    static TIMER: RefCell<Option<TimerId>> = const { RefCell::new(None) };
}

const POOL_CHECK_TIMER: u64 = 30 * 60; // 30 mins

///
/// CanisterPoolEntry
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct CanisterPoolEntry {
    pub created_at: u64,
    pub cycles: Cycles,
}

impl_storable_candid_unbounded!(CanisterPoolEntry);

///
/// CanisterPool
///

pub type CanisterPoolView = Vec<(Principal, CanisterPoolEntry)>;

pub struct CanisterPool;

impl CanisterPool {
    /// Start recurring tracking every 5 minutes
    /// Safe to call multiple times: only one loop will run.
    pub fn start() {
        TIMER.with_borrow_mut(|slot| {
            if slot.is_some() {
                return;
            }

            // set a timer to track, and possibly top-up
            let id = set_timer(crate::CANISTER_INIT_DELAY, || {
                // do first track
                let _ = Self::check();

                // now start the recurring interval
                let interval_id =
                    set_timer_interval(std::time::Duration::from_secs(POOL_CHECK_TIMER), || {
                        let _ = Self::check();
                    });

                TIMER.with_borrow_mut(|slot| *slot = Some(interval_id));
            });

            *slot = Some(id);
        });
    }

    /// Stop recurring tracking.
    pub fn stop() {
        TIMER.with_borrow_mut(|slot| {
            if let Some(id) = slot.take() {
                clear_timer(id);
            }
        });
    }

    #[must_use]
    pub fn check() -> u64 {
        let pool_size = CANISTER_POOL.with_borrow(CanisterPoolCore::len);

        if let Ok(canister) = Config::try_get() {
            let min_size = u64::from(canister.pool.minimum_size);
            if pool_size < min_size {
                // Safety valve: never create more than 10 at once.
                // This avoids a "thundering herd" if the pool is empty and min_size is large.
                let missing = (min_size - pool_size).min(10);

                log!(
                    Log::Ok,
                    "💧 canister pool low: size {pool_size}, min {min_size}, creating {missing}",
                );

                spawn(async move {
                    for i in 0..missing {
                        match create_pool_canister().await {
                            Ok(_) => {
                                log!(Log::Ok, "✨ pool canister created ({}/{missing})", i + 1);
                            }
                            Err(e) => {
                                log!(Log::Warn, "⚠️ failed to create pool canister: {e:?}");
                            }
                        }
                    }
                });

                return missing;
            }
        }

        0
    }

    pub fn register(pid: Principal, cycles: Cycles) {
        let entry = CanisterPoolEntry {
            created_at: now_secs(),
            cycles,
        };

        CANISTER_POOL.with_borrow_mut(|core| core.insert(pid, entry));
    }

    #[must_use]
    pub fn pop_first() -> Option<(Principal, CanisterPoolEntry)> {
        CANISTER_POOL.with_borrow_mut(CanisterPoolCore::pop_first)
    }

    #[must_use]
    pub fn remove(pid: &Principal) -> Option<CanisterPoolEntry> {
        CANISTER_POOL.with_borrow_mut(|core| core.remove(pid))
    }

    #[must_use]
    pub fn export() -> CanisterPoolView {
        CANISTER_POOL.with_borrow(CanisterPoolCore::export)
    }
}

///
/// CanisterPoolCore
///

pub struct CanisterPoolCore<M: Memory> {
    map: BTreeMap<Principal, CanisterPoolEntry, M>,
}

impl<M: Memory> CanisterPoolCore<M> {
    pub const fn new(map: BTreeMap<Principal, CanisterPoolEntry, M>) -> Self {
        Self { map }
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    pub fn len(&self) -> u64 {
        self.map.len()
    }

    pub fn insert(&mut self, pid: Principal, entry: CanisterPoolEntry) {
        self.map.insert(pid, entry);
    }

    // gets the oldest canister in the pool
    pub fn pop_first(&mut self) -> Option<(Principal, CanisterPoolEntry)> {
        let min_pid = self
            .map
            .iter()
            .min_by_key(|entry| entry.value().created_at)
            .map(|entry| *entry.key())?;

        self.map.remove(&min_pid).map(|entry| (min_pid, entry))
    }

    pub fn remove(&mut self, pid: &Principal) -> Option<CanisterPoolEntry> {
        self.map.remove(pid)
    }

    pub fn export(&self) -> CanisterPoolView {
        self.map.to_vec()
    }
}
