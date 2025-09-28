use crate::{
    Log,
    cdk::{
        futures::spawn,
        structures::{BTreeMap, DefaultMemoryImpl, memory::VirtualMemory},
        timers::{TimerId, clear_timer, set_timer, set_timer_interval},
    },
    config::Config,
    icu_eager_static, icu_memory, impl_storable_unbounded, log,
    memory::id::root::CANISTER_RESERVE_ID,
    ops::reserve::create_reserve_canister,
    types::Cycles,
    utils::time::now_secs,
};
use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;

//
// CANISTER_RESERVE
//

icu_eager_static! {
    static CANISTER_RESERVE: RefCell<BTreeMap<Principal, CanisterReserveEntry, VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(BTreeMap::init(
            icu_memory!(CanisterReserve, CANISTER_RESERVE_ID),
        ));
}

thread_local! {
    static TIMER: RefCell<Option<TimerId>> = const { RefCell::new(None) };
}

const RESERVE_CHECK_TIMER: u64 = 30 * 60; // 30 mins

///
/// CanisterReserveEntry
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct CanisterReserveEntry {
    pub created_at: u64,
    pub cycles: Cycles,
}

impl_storable_unbounded!(CanisterReserveEntry);

///
/// CanisterReserve
///

pub struct CanisterReserve;

pub type CanisterReserveView = Vec<(Principal, CanisterReserveEntry)>;

impl CanisterReserve {
    /// Start recurring tracking every 30 minutes
    /// Safe to call multiple times: only one loop will run.
    pub fn start() {
        TIMER.with_borrow_mut(|slot| {
            if slot.is_some() {
                return;
            }

            let id = set_timer(crate::CANISTER_INIT_DELAY, || {
                let _ = Self::check();

                let interval_id =
                    set_timer_interval(std::time::Duration::from_secs(RESERVE_CHECK_TIMER), || {
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

    /// Check the reserve size and create new canisters if required.
    #[must_use]
    pub fn check() -> u64 {
        let reserve_size = CANISTER_RESERVE.with_borrow(|map| map.len());

        if let Ok(cfg) = Config::try_get() {
            let min_size = u64::from(cfg.reserve.minimum_size);
            if reserve_size < min_size {
                // Safety valve: never create more than 10 at once.
                let missing = (min_size - reserve_size).min(10);

                log!(
                    Log::Ok,
                    "💧 canister reserve low: size {reserve_size}, min {min_size}, creating {missing}"
                );

                spawn(async move {
                    for i in 0..missing {
                        match create_reserve_canister().await {
                            Ok(_) => {
                                log!(Log::Ok, "✨ reserve canister created ({}/{missing})", i + 1);
                            }
                            Err(e) => {
                                log!(Log::Warn, "⚠️  failed to create reserve canister: {e:?}");
                            }
                        }
                    }
                });

                return missing;
            }
        }

        0
    }

    /// Register a canister into the reserve.
    pub fn register(pid: Principal, cycles: Cycles) {
        let entry = CanisterReserveEntry {
            created_at: now_secs(),
            cycles,
        };

        CANISTER_RESERVE.with_borrow_mut(|map| {
            map.insert(pid, entry);
        });
    }

    /// Pop the oldest canister from the reserve.
    #[must_use]
    pub fn pop_first() -> Option<(Principal, CanisterReserveEntry)> {
        CANISTER_RESERVE.with_borrow_mut(|map| {
            let min_pid = map
                .iter()
                .min_by_key(|entry| entry.value().created_at)
                .map(|entry| *entry.key())?;
            map.remove(&min_pid).map(|entry| (min_pid, entry))
        })
    }

    /// Remove a specific canister from the reserve.
    #[must_use]
    pub fn remove(pid: &Principal) -> Option<CanisterReserveEntry> {
        CANISTER_RESERVE.with_borrow_mut(|map| map.remove(pid))
    }

    /// Export the reserve as a vector of (Principal, Entry).
    #[must_use]
    pub fn export() -> CanisterReserveView {
        CANISTER_RESERVE.with_borrow(BTreeMap::to_vec)
    }

    /// Clear the reserve (mainly for tests).
    pub fn clear() {
        CANISTER_RESERVE.with_borrow_mut(BTreeMap::clear);
    }

    /// Return the current reserve size.
    #[must_use]
    pub fn len() -> u64 {
        CANISTER_RESERVE.with_borrow(|map| map.len())
    }

    /// Return whether the reserve is empty.
    #[must_use]
    pub fn is_empty() -> bool {
        CANISTER_RESERVE.with_borrow(|map| map.is_empty())
    }
}

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::*;
    use candid::Principal;

    fn pid(n: u8) -> Principal {
        Principal::self_authenticating(vec![n])
    }

    #[test]
    fn register_and_export() {
        CanisterReserve::clear();

        let p1 = pid(1);
        let p2 = pid(2);

        CanisterReserve::register(p1, 100u128.into());
        CanisterReserve::register(p2, 200u128.into());

        let view = CanisterReserve::export();
        assert_eq!(view.len(), 2);

        let entry1 = view.iter().find(|(id, _)| *id == p1).unwrap();
        assert_eq!(entry1.1.cycles, 100u128.into());

        let entry2 = view.iter().find(|(id, _)| *id == p2).unwrap();
        assert_eq!(entry2.1.cycles, 200u128.into());
    }

    #[test]
    fn remove_specific_pid() {
        CanisterReserve::clear();

        let p1 = pid(1);
        let p2 = pid(2);

        CanisterReserve::register(p1, 123u128.into());
        CanisterReserve::register(p2, 456u128.into());

        let removed = CanisterReserve::remove(&p1).unwrap();
        assert_eq!(removed.cycles, 123u128.into());

        // only p2 should remain
        let view = CanisterReserve::export();
        assert_eq!(view.len(), 1);
        assert_eq!(view[0].0, p2);
    }

    #[test]
    fn clear_resets_reserve() {
        CanisterReserve::clear();

        CanisterReserve::register(pid(1), 10u128.into());
        assert!(!CanisterReserve::is_empty());

        CanisterReserve::clear();
        assert!(CanisterReserve::is_empty());
    }
}
