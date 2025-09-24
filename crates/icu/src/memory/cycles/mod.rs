use crate::{
    Log,
    cdk::{
        futures::spawn,
        structures::{BTreeMap, DefaultMemoryImpl, memory::VirtualMemory},
        timers::{TimerId, clear_timer, set_timer, set_timer_interval},
    },
    config::Config,
    icu_memory,
    interface::ic::canister_cycle_balance,
    log,
    memory::{CYCLE_TRACKER_MEMORY_ID, CanisterState},
    thread_local_register,
    types::Cycles,
    utils::time::now_secs,
};
use std::cell::RefCell;

//
// CYCLE_TRACKER
//

thread_local_register! {
    static CYCLE_TRACKER: RefCell<CycleTracker> =
        RefCell::new(CycleTracker::new(BTreeMap::init(
            icu_memory!(CycleTracker, CYCLE_TRACKER_MEMORY_ID),
        )));
}

thread_local! {
    static TIMER: RefCell<Option<TimerId>> = const { RefCell::new(None) };
}

/// constants
const TIMEOUT_SECS: u64 = 60 * 10; // 10 minutes
const RETAIN_SECS: u64 = 60 * 60 * 24 * 7; // ~7 days
const PURGE_INSERT_INTERVAL: u64 = 1_000; // purge every 1000 inserts

///
/// CycleTracker
///
/// NOTE : Can't really do tests for this here, it really needs e2e because I can't
/// declare M: Memory as a generic right now, it breaks ic-stable-structures/other ic packages
///

pub type CycleTrackerView = Vec<(u64, Cycles)>;

pub struct CycleTracker {
    map: BTreeMap<u64, u128, VirtualMemory<DefaultMemoryImpl>>,
    insert_count: u64,
}

impl CycleTracker {
    pub const fn new(map: BTreeMap<u64, u128, VirtualMemory<DefaultMemoryImpl>>) -> Self {
        Self {
            map,
            insert_count: 0,
        }
    }

    #[must_use]
    pub fn len() -> u64 {
        CYCLE_TRACKER.with_borrow(|t| t.map.len())
    }

    /// Start recurring tracking every X seconds
    /// Safe to call multiple times: only one loop will run.
    pub fn start() {
        TIMER.with_borrow_mut(|slot| {
            if slot.is_some() {
                return;
            }

            let id = set_timer(crate::CANISTER_INIT_DELAY, || {
                let _ = Self::track();

                let interval_id =
                    set_timer_interval(std::time::Duration::from_secs(TIMEOUT_SECS), || {
                        let _ = Self::track();
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
    pub fn track() -> bool {
        let ts = now_secs();
        let cycles = canister_cycle_balance().to_u128();

        Self::check_auto_topup();

        CYCLE_TRACKER.with_borrow_mut(|t| t.insert(ts, cycles))
    }

    pub fn check_auto_topup() {
        use crate::ops::request::cycles_request;

        if let Some(entry) = CanisterState::get_view()
            && let Ok(canister) = Config::try_get_canister(&entry.ty)
            && let Some(topup) = canister.topup
        {
            let cycles = canister_cycle_balance();

            if cycles < topup.threshold {
                spawn(async move {
                    match cycles_request(topup.amount.to_u128()).await {
                        Ok(res) => log!(
                            Log::Ok,
                            "💫 requested {}, topped up by {}, now {}",
                            topup.amount,
                            res.cycles_transferred,
                            canister_cycle_balance()
                        ),
                        Err(e) => log!(Log::Error, "💫 failed to request cycles: {e}"),
                    }
                });
            }
        }
    }

    #[must_use]
    pub fn purge_old() -> usize {
        let ts = now_secs();
        CYCLE_TRACKER.with_borrow_mut(|t| t.purge(ts))
    }

    pub fn clear() {
        CYCLE_TRACKER.with_borrow_mut(|t| {
            t.map.clear();
            t.insert_count = 0;
        });
    }

    #[must_use]
    pub fn export() -> CycleTrackerView {
        CYCLE_TRACKER.with_borrow(Self::view)
    }

    // --- internal state methods ---

    fn insert(&mut self, now: u64, cycles: u128) -> bool {
        self.map.insert(now, cycles);
        self.insert_count += 1;

        if self.insert_count.is_multiple_of(PURGE_INSERT_INTERVAL) {
            self.purge(now);
        }

        true
    }

    fn purge(&mut self, now: u64) -> usize {
        let cutoff = now.saturating_sub(RETAIN_SECS);
        let mut purged = 0;

        while let Some((first_ts, _)) = self.map.first_key_value() {
            if first_ts < cutoff {
                self.map.remove(&first_ts);
                purged += 1;
            } else {
                break;
            }
        }

        log!(Log::Info, "cycle_tracker: purged {purged} old entries");
        purged
    }

    fn view(&self) -> CycleTrackerView {
        self.map.view().map(|(t, c)| (t, c.into())).collect()
    }
}
