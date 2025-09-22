use crate::{
    Log,
    cdk::{
        futures::spawn,
        structures::{BTreeMap, DefaultMemoryImpl, Memory, memory::VirtualMemory},
        timers::{TimerId, clear_timer, set_timer, set_timer_interval},
    },
    config::Config,
    icu_register_memory,
    interface::ic::canister_cycle_balance,
    log,
    memory::{CYCLE_TRACKER_MEMORY_ID, CanisterState},
    types::Cycles,
    utils::time::now_secs,
};
use std::cell::RefCell;

//
// CYCLE_TRACKER
// Timestamp, number of cycles
//

thread_local! {
    static TRACKER: RefCell<CycleTrackerCore<VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(CycleTrackerCore::new(BTreeMap::init(icu_register_memory!(
            CYCLE_TRACKER_MEMORY_ID
        ))));

    static TIMER: RefCell<Option<TimerId>> = const { RefCell::new(None) };
}

const TIMEOUT_SECS: u64 = 60 * 10; // 10 minutes
const RETAIN_SECS: u64 = 60 * 60 * 24 * 10; // ~10 days
const PURGE_INSERT_INTERVAL: u64 = 1_000; // purge every 1000 inserts
const MIN_SPACING_SECS: u64 = 30; // anti-spam safety

///
/// CycleTracker
///

pub type CycleTrackerView = Vec<(u64, Cycles)>;

pub struct CycleTracker;

impl CycleTracker {
    /// Start recurring tracking every X seconds
    /// Safe to call multiple times: only one loop will run.
    pub fn start() {
        TIMER.with_borrow_mut(|slot| {
            if slot.is_some() {
                return;
            }

            // set a timer to track, and possibly top-up
            let id = set_timer(crate::CANISTER_INIT_DELAY, || {
                // do first track
                let _ = Self::track();

                // now start the recurring interval
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

        TRACKER.with_borrow_mut(|core| core.track(ts, cycles))
    }

    pub fn check_auto_topup() {
        use crate::ops::request::cycles_request;

        if let Some(entry) = CanisterState::get_view()
            && let Ok(canister) = Config::try_get_canister(&entry.ty)
            && let Some(topup) = canister.topup
        {
            let cycles = canister_cycle_balance();

            if cycles < topup.threshold {
                // fire and forget
                spawn(async move {
                    match cycles_request(topup.amount.to_u128()).await {
                        Ok(res) => log!(
                            Log::Ok,
                            "💫 requested {}, topped up by {}, now {}",
                            topup.amount,
                            res.cycles_transferred,
                            canister_cycle_balance()
                        ),
                        Err(e) => {
                            log!(Log::Error, "💫 failed to request cycles: {e}");
                        }
                    }
                });
            }
        }
    }

    #[must_use]
    pub fn purge_old() -> usize {
        let ts = now_secs();
        TRACKER.with_borrow_mut(|core| core.purge_old(ts))
    }

    pub fn clear() {
        TRACKER.with_borrow_mut(CycleTrackerCore::clear);
    }

    #[must_use]
    pub fn export() -> CycleTrackerView {
        TRACKER.with_borrow(CycleTrackerCore::export)
    }
}

///
/// CycleTrackerCore
///

pub struct CycleTrackerCore<M: Memory> {
    map: BTreeMap<u64, u128, M>,
    first_ts: Option<u64>,
    last_ts: Option<u64>,
    insert_count: u64,
}

impl<M: Memory> CycleTrackerCore<M> {
    pub const fn new(map: BTreeMap<u64, u128, M>) -> Self {
        Self {
            map,
            first_ts: None,
            last_ts: None,
            insert_count: 0,
        }
    }

    // for testing
    #[cfg(test)]
    const fn map(&self) -> &BTreeMap<u64, u128, M> {
        &self.map
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    pub fn clear(&mut self) {
        self.map.clear();
        self.first_ts = None;
        self.last_ts = None;
        self.insert_count = 0;
    }

    pub fn track(&mut self, now: u64, cycles: u128) -> bool {
        // first_ts and last_ts won't persist after a canister upgrade
        // so let's just reset them here
        if self.first_ts.is_none() {
            self.first_ts = self.map.first_key_value().map(|(ts, _)| ts);
        }
        if self.last_ts.is_none() {
            self.last_ts = self.map.last_key_value().map(|(ts, _)| ts);
        }

        // check timeout
        let can_insert = match self.last_ts {
            Some(last_ts) => now.saturating_sub(last_ts) >= MIN_SPACING_SECS,
            None => true,
        };

        if can_insert {
            self.map.insert(now, cycles);
            if self.first_ts.is_none() {
                self.first_ts = Some(now);
            }
            self.last_ts = Some(now);
            self.insert_count += 1;

            // purge every Nth insert
            if self.insert_count.is_multiple_of(PURGE_INSERT_INTERVAL) {
                self.purge_old(now);
            }

            true
        } else {
            false
        }
    }

    // purge_old
    pub fn purge_old(&mut self, now: u64) -> usize {
        let cutoff = now.saturating_sub(RETAIN_SECS);
        let mut purged = 0;

        while let Some(first_ts) = self.first_ts {
            if first_ts < cutoff {
                self.map.remove(&first_ts);
                purged += 1;
                self.first_ts = self.map.first_key_value().map(|(ts, _)| ts);
            } else {
                break;
            }
        }

        log!(Log::Info, "cycle_tracker: purged {purged} old entries");

        purged
    }

    // export
    // export to an ordered vec view
    pub fn export(&self) -> CycleTrackerView {
        self.map.view().map(|(t, c)| (t, c.into())).collect()
    }
}

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cdk::structures::DefaultMemoryImpl;

    fn make_core() -> CycleTrackerCore<DefaultMemoryImpl> {
        let tree = BTreeMap::init(DefaultMemoryImpl::default());
        CycleTrackerCore::new(tree)
    }

    #[test]
    fn test_track_and_purge() {
        let mut tracker = make_core();

        // First insert always accepted
        assert!(tracker.track(1000, 111));
        // Too soon (< MIN_SPACING_SECS)
        assert!(!tracker.track(1000 + (MIN_SPACING_SECS - 1), 222));
        // Exactly on spacing boundary is ok
        assert!(tracker.track(1000 + MIN_SPACING_SECS, 333));

        assert_eq!(tracker.first_ts, Some(1000));
        assert_eq!(tracker.last_ts, Some(1000 + MIN_SPACING_SECS));

        // Purge old entries when well past retention window
        let purged = tracker.purge_old(1000 + RETAIN_SECS + 1);
        assert!(purged >= 1);
    }

    #[test]
    fn test_is_empty_and_clear() {
        let mut tracker = make_core();
        assert!(tracker.is_empty());

        tracker.track(10, 1000);
        assert!(!tracker.is_empty());

        tracker.clear();
        assert!(tracker.is_empty());
        assert_eq!(tracker.first_ts, None);
        assert_eq!(tracker.last_ts, None);
    }

    #[test]
    fn test_track_spacing() {
        let mut tracker = make_core();

        // First always goes in
        assert!(tracker.track(0, 1));
        // Too soon (< MIN_SPACING_SECS)
        assert!(!tracker.track(MIN_SPACING_SECS - 1, 2));
        // Exactly on spacing boundary
        assert!(tracker.track(MIN_SPACING_SECS, 3));
        // Well beyond spacing
        assert!(tracker.track(2 * MIN_SPACING_SECS + 5, 4));

        let data = tracker.map();
        assert_eq!(data.len(), 3); // first, third, fourth inserted
        assert!(data.contains_key(&0));
        assert!(data.contains_key(&MIN_SPACING_SECS));
        assert!(data.contains_key(&(2 * MIN_SPACING_SECS + 5)));
    }

    #[test]
    fn test_purge_keeps_recent_entries() {
        let mut tracker = make_core();
        tracker.track(100, 1);
        tracker.track(200, 2);
        tracker.track(300, 3);

        // cutoff is now - RETAIN_SECS; here, all timestamps are "recent"
        let purged = tracker.purge_old(400);
        assert_eq!(purged, 0);

        // far future: everything should be purged
        let purged = tracker.purge_old(400 + RETAIN_SECS + 1);
        assert!(purged >= 1);
        assert!(tracker.is_empty());
    }

    #[test]
    fn test_export_matches_inserted() {
        let mut tracker = make_core();
        tracker.track(10, 100);
        tracker.track(20 + TIMEOUT_SECS, 200);

        let map = tracker.map();
        assert_eq!(map.get(&10), Some(100));
        assert_eq!(map.get(&(20 + TIMEOUT_SECS)), Some(200));
    }

    #[test]
    fn test_purge_removes_only_old_entries() {
        let mut tracker = make_core();

        // Insert entries spaced far apart
        tracker.track(0, 1);
        tracker.track(RETAIN_SECS / 2, 2);
        tracker.track(RETAIN_SECS, 3);

        // Purge just after RETAIN_SECS + 1 → first entry should go
        let purged = tracker.purge_old(RETAIN_SECS + 1);
        assert_eq!(purged, 1);
        assert!(tracker.map().contains_key(&(RETAIN_SECS / 2)));
        assert!(tracker.map().contains_key(&RETAIN_SECS));
    }

    #[test]
    fn test_first_and_last_recovery_after_clear() {
        let mut tracker = make_core();

        tracker.track(10, 111);
        tracker.track(10 + TIMEOUT_SECS, 222);

        assert_eq!(tracker.first_ts, Some(10));
        assert_eq!(tracker.last_ts, Some(10 + TIMEOUT_SECS));

        tracker.clear();
        assert_eq!(tracker.first_ts, None);
        assert_eq!(tracker.last_ts, None);

        // Tracking again should recover first/last correctly
        tracker.track(500, 999);
        assert_eq!(tracker.first_ts, Some(500));
        assert_eq!(tracker.last_ts, Some(500));
    }

    #[test]
    fn test_track_skips_if_too_soon() {
        let mut tracker = make_core();

        assert!(tracker.track(1000, 1));
        // Too soon (< MIN_SPACING_SECS)
        assert!(!tracker.track(1000 + MIN_SPACING_SECS - 1, 2));
        // Exactly at spacing boundary
        assert!(tracker.track(1000 + MIN_SPACING_SECS, 3));

        let map = tracker.map();
        assert_eq!(map.len(), 2);
        assert!(map.contains_key(&1000));
        assert!(map.contains_key(&(1000 + MIN_SPACING_SECS)));
    }

    #[test]
    fn test_track_multiple_calls_within_spacing() {
        let mut tracker = make_core();

        assert!(tracker.track(500, 10));
        // spam calls within 1 second each
        for i in 0..10 {
            assert!(!tracker.track(500 + i, 20));
        }

        // only first entry should be present
        let map = tracker.map();
        assert_eq!(map.len(), 1);
        assert!(map.contains_key(&500));
    }

    #[test]
    fn test_track_allows_after_spacing() {
        let mut tracker = make_core();

        assert!(tracker.track(1000, 1));
        assert!(!tracker.track(1000 + 5, 2)); // too soon
        assert!(tracker.track(1000 + MIN_SPACING_SECS, 3)); // ok after spacing
        assert!(tracker.track(1000 + 2 * MIN_SPACING_SECS, 4)); // also ok

        let map = tracker.map();
        assert_eq!(map.len(), 3);
        assert!(map.contains_key(&1000));
        assert!(map.contains_key(&(1000 + MIN_SPACING_SECS)));
        assert!(map.contains_key(&(1000 + 2 * MIN_SPACING_SECS)));
    }

    #[test]
    fn test_purge_not_triggered_before_interval() {
        let mut tracker = make_core();

        // Insert fewer than PURGE_INSERT_INTERVAL entries
        for i in 0..(PURGE_INSERT_INTERVAL - 1) {
            assert!(tracker.track(i * MIN_SPACING_SECS, u128::from(i)));
        }

        // Purge should not have run yet (still contains all entries)
        assert_eq!(tracker.map().len() as u64, PURGE_INSERT_INTERVAL - 1);
    }

    #[test]
    fn test_purge_triggered_on_interval() {
        let mut tracker = make_core();

        // Insert exactly PURGE_INSERT_INTERVAL entries
        for i in 0..PURGE_INSERT_INTERVAL {
            assert!(tracker.track(i * MIN_SPACING_SECS, u128::from(i)));
        }

        // At this point, purge should have been called once
        // → Entries older than RETAIN_SECS may be gone, but at least 1 purge happened
        assert!(tracker.insert_count.is_multiple_of(PURGE_INSERT_INTERVAL));
        assert!(tracker.map().len() as u64 <= PURGE_INSERT_INTERVAL);
    }

    #[test]
    fn test_multiple_purge_cycles() {
        let mut tracker = make_core();

        let total_inserts = PURGE_INSERT_INTERVAL * 3;
        for i in 0..total_inserts {
            assert!(tracker.track(i * MIN_SPACING_SECS, u128::from(i)));
        }

        // Purge should have run 3 times
        assert_eq!(tracker.insert_count, total_inserts);

        // Old entries beyond RETAIN_SECS should be gone
        let last_ts = (total_inserts - 1) * MIN_SPACING_SECS;
        let cutoff = last_ts.saturating_sub(RETAIN_SECS);

        for (ts, _) in tracker.export() {
            assert!(ts >= cutoff, "found entry {ts} older than cutoff {cutoff}");
        }
    }

    #[test]
    fn test_purge_on_empty_tracker() {
        let mut tracker = make_core();
        let purged = tracker.purge_old(10_000);
        assert_eq!(purged, 0);
        assert!(tracker.is_empty());
    }

    #[test]
    fn test_export_ordering() {
        let mut tracker = make_core();

        let base = 1000;
        assert!(tracker.track(base, 1));
        assert!(tracker.track(base + MIN_SPACING_SECS, 2));
        assert!(tracker.track(base + 2 * MIN_SPACING_SECS, 3));

        let exported = tracker.export();
        let ts: Vec<u64> = exported.iter().map(|(t, _)| *t).collect();

        assert_eq!(
            ts,
            vec![base, base + MIN_SPACING_SECS, base + 2 * MIN_SPACING_SECS]
        );
    }

    #[test]
    #[allow(clippy::cast_sign_loss)]
    fn test_track_exact_spacing_repeated() {
        let mut tracker = make_core();

        let mut now = 0;
        assert!(tracker.track(now, 1));

        for i in 1..5 {
            now += MIN_SPACING_SECS;
            assert!(tracker.track(now, i as u128));
        }

        assert_eq!(tracker.map().len(), 5);
    }

    #[test]
    fn test_large_timestamp_jump_triggers_purge() {
        let mut tracker = make_core();
        tracker.track(0, 1);
        tracker.track(RETAIN_SECS * 2, 2);

        // After inserting at RETAIN_SECS * 2, old entry should be purged
        tracker.purge_old(RETAIN_SECS * 2);
        let exported = tracker.export();

        assert_eq!(exported.len(), 1);
        assert_eq!(exported[0].0, RETAIN_SECS * 2);
    }

    #[test]
    fn test_track_after_full_purge() {
        let mut tracker = make_core();
        tracker.track(0, 1);

        // force purge far in future
        tracker.purge_old(RETAIN_SECS * 10);
        assert!(tracker.is_empty());

        tracker.track(9999, 42);
        assert_eq!(tracker.first_ts, Some(9999));
        assert_eq!(tracker.last_ts, Some(9999));
        assert_eq!(tracker.map().len(), 1);
    }
}
