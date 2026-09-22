//! Module: model::public_metrics::history
//!
//! Responsibility: retain bounded public history with adjacent writes for grouped series.
//! Does not own: producers, schedules, DTO conversion, or query authorization.
//! Boundary: only validated observations enter history; reads never refresh it.

#[cfg(test)]
mod tests;

use crate::{
    cdk::types::Principal,
    domain::public_metrics::{PublicMetricFamily, PublicMetricKind},
    model::public_metrics::{PUBLIC_METRICS_CADENCE_NS, PublicMetricSample},
    view::public_metrics::PublicHistorySeries,
};
use std::{
    cell::RefCell,
    collections::{HashMap, hash_map::Entry},
    mem::size_of,
};

/// Fixed slots per series: twenty-four hours at a five-minute cadence.
pub const PUBLIC_HISTORY_SLOTS: usize = 288;
/// Total series across all selected families in one canister.
pub const MAX_HISTORY_SERIES: usize = 256;
/// Maximum conservatively accounted history storage, including unused group positions.
pub const MAX_HISTORY_BYTES: usize = 8 * 1024 * 1024;

const GROUP_WIDTH: usize = 8;
const MAX_GROUPS: usize = MAX_HISTORY_SERIES / GROUP_WIDTH;
const VALID_WORDS: usize = PUBLIC_HISTORY_SLOTS.div_ceil(64);
const EMPTY_POINT: PublicHistorySample = PublicHistorySample {
    slot: 0,
    observed_at_ns: 0,
    value: 0,
    kind: PublicMetricKind::Gauge,
};

/// An actual observation in a sampling slot.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PublicHistorySample {
    pub slot: u64,
    pub observed_at_ns: u64,
    pub value: u128,
    pub kind: PublicMetricKind,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct SeriesKey {
    family: PublicMetricFamily,
    name: String,
    canister_id: Option<Principal>,
}

/// Stable position and validity belong to the series, independently of its neighbours.
struct StoredSeries {
    unit: String,
    latest_observed_at_ns: u64,
    group: usize,
    position: usize,
    valid: [u64; VALID_WORDS],
}

/// One allocation, ordered by source slot then position; rows initialise lazily.
struct Group {
    origin: u64,
    occupied: u16,
    rows: Vec<[PublicHistorySample; GROUP_WIDTH]>,
}

impl Group {
    fn new(origin: u64) -> Self {
        Self {
            origin,
            occupied: 0,
            rows: Vec::with_capacity(PUBLIC_HISTORY_SLOTS),
        }
    }

    fn vacancy(&self) -> Option<usize> {
        (0..GROUP_WIDTH).find(|position| self.occupied & (1 << position) == 0)
    }

    fn write(&mut self, series: &mut StoredSeries, point: PublicHistorySample) {
        let window = PUBLIC_HISTORY_SLOTS as u64;
        let physical =
            usize::try_from((point.slot % window + window - self.origin % window) % window)
                .expect("history slot is below 288");
        self.rows.resize(
            self.rows.len().max(physical + 1),
            [EMPTY_POINT; GROUP_WIDTH],
        );
        self.rows[physical][series.position] = point;
        series.valid[physical / 64] |= 1 << (physical % 64);
        series.latest_observed_at_ns = point.observed_at_ns;
    }

    fn project(&self, series: &StoredSeries) -> PublicHistorySeries {
        let latest_slot = series.latest_observed_at_ns / PUBLIC_METRICS_CADENCE_NS;
        let mut slots = Vec::with_capacity(
            series
                .valid
                .iter()
                .map(|word| word.count_ones() as usize)
                .sum(),
        );
        // Start just after the newest modulo position: the retained window is
        // two ordered physical ranges, so no point sorting or rotation is needed.
        let window = PUBLIC_HISTORY_SLOTS as u64;
        let split =
            usize::try_from((latest_slot % window + 1 + window - self.origin % window) % window)
                .expect("history slot is below 288");
        self.append_range(series, split..PUBLIC_HISTORY_SLOTS, latest_slot, &mut slots);
        self.append_range(series, 0..split, latest_slot, &mut slots);
        PublicHistorySeries {
            unit: series.unit.clone(),
            slots,
        }
    }

    fn append_range(
        &self,
        series: &StoredSeries,
        range: std::ops::Range<usize>,
        latest_slot: u64,
        slots: &mut Vec<PublicHistorySample>,
    ) {
        if range.is_empty() {
            return;
        }
        let first_word = range.start / 64;
        let last_word = (range.end - 1) / 64;
        for word_index in first_word..=last_word {
            let mut remaining = series.valid[word_index];
            if word_index == first_word {
                remaining &= u64::MAX << (range.start % 64);
            }
            if word_index == last_word {
                remaining &= u64::MAX >> (63 - (range.end - 1) % 64);
            }
            while remaining != 0 {
                let physical = word_index * 64 + remaining.trailing_zeros() as usize;
                remaining &= remaining - 1;
                let point = self.rows[physical][series.position];
                if latest_slot.saturating_sub(point.slot) < PUBLIC_HISTORY_SLOTS as u64 {
                    slots.push(point);
                }
            }
        }
    }
}

struct History {
    // Admission follows validated input order; index iteration never decides publication.
    series: HashMap<SeriesKey, StoredSeries>,
    groups: [Option<Group>; MAX_GROUPS],
    heap_started_at_ns: Option<u64>,
    reserved_bytes: usize,
    truncated: bool,
    last_expired_slot: Option<u64>,
}

impl Default for History {
    fn default() -> Self {
        Self {
            series: HashMap::new(),
            groups: [const { None }; MAX_GROUPS],
            heap_started_at_ns: None,
            reserved_bytes: 0,
            truncated: false,
            last_expired_slot: None,
        }
    }
}

impl History {
    fn recount(&mut self) {
        if self.series.is_empty() {
            self.groups = [const { None }; MAX_GROUPS];
            self.reserved_bytes = 0;
            return;
        }
        self.reserved_bytes = group_index_reservation()
            + self.groups.iter().flatten().count() * group_reservation()
            + self
                .series
                .iter()
                .map(|(key, series)| reservation(key, &series.unit))
                .sum::<usize>();
    }

    fn record(&mut self, family: PublicMetricFamily, metric: &PublicMetricSample) {
        let slot = metric.observed_at_ns / PUBLIC_METRICS_CADENCE_NS;
        let key = SeriesKey {
            family,
            name: metric.name.clone(),
            canister_id: metric.canister_id,
        };
        let series_count = self.series.len();
        let series = match self.series.entry(key) {
            Entry::Vacant(entry) => {
                if series_count >= MAX_HISTORY_SERIES {
                    self.truncated = true;
                    return;
                }
                let vacancy = self.groups.iter().enumerate().find_map(|(index, group)| {
                    group.as_ref()?.vacancy().map(|position| (index, position))
                });
                let bytes = reservation(entry.key(), &metric.unit)
                    + if vacancy.is_none() {
                        group_reservation()
                    } else {
                        0
                    }
                    + if series_count == 0 {
                        group_index_reservation()
                    } else {
                        0
                    };
                if self.reserved_bytes + bytes > MAX_HISTORY_BYTES {
                    self.truncated = true;
                    return;
                }
                let (group, position) = occupy(&mut self.groups, vacancy, slot);
                self.reserved_bytes += bytes;
                entry.insert(StoredSeries {
                    unit: metric.unit.clone(),
                    latest_observed_at_ns: metric.observed_at_ns,
                    group,
                    position,
                    valid: [0; VALID_WORDS],
                })
            }
            Entry::Occupied(mut entry) => {
                if metric.observed_at_ns < entry.get().latest_observed_at_ns {
                    return;
                }
                if entry.get().unit != metric.unit {
                    let prior_bytes = reservation(entry.key(), &entry.get().unit);
                    let next_bytes = reservation(entry.key(), &metric.unit);
                    let next_total = self.reserved_bytes - prior_bytes + next_bytes;
                    if next_total > MAX_HISTORY_BYTES {
                        let removed = entry.remove();
                        release(&mut self.groups, &removed);
                        self.truncated = true;
                        self.recount();
                        return;
                    }
                    self.reserved_bytes = next_total;
                    let series = entry.get_mut();
                    series.unit = metric.unit.as_str().into();
                    series.valid = [0; VALID_WORDS];
                }
                entry.into_mut()
            }
        };
        self.groups[series.group].as_mut().unwrap().write(
            series,
            PublicHistorySample {
                slot,
                observed_at_ns: metric.observed_at_ns,
                value: metric.value,
                kind: metric.kind,
            },
        );
    }
}

fn occupy(
    groups: &mut [Option<Group>],
    vacancy: Option<(usize, usize)>,
    slot: u64,
) -> (usize, usize) {
    let (group, position) = vacancy.unwrap_or_else(|| {
        let index = groups
            .iter()
            .position(Option::is_none)
            .expect("series admission leaves a free group position");
        groups[index] = Some(Group::new(slot));
        (index, 0)
    });
    groups[group].as_mut().unwrap().occupied |= 1 << position;
    (group, position)
}

fn release(groups: &mut [Option<Group>], series: &StoredSeries) {
    let group = groups[series.group].as_mut().unwrap();
    group.occupied &= !(1 << series.position);
    if group.occupied == 0 {
        groups[series.group] = None;
    }
}

thread_local! {
    static HISTORY: RefCell<History> = RefCell::default();
}

/// Heap-only retention owner; restart naturally creates an empty history epoch.
pub struct PublicHistoryCache;

impl PublicHistoryCache {
    /// Evict whole expired series, bounded by the total admitted series cap.
    pub fn expire(now_ns: u64) {
        HISTORY.with_borrow_mut(|history| {
            history.heap_started_at_ns.get_or_insert(now_ns);
            let slot = now_ns / PUBLIC_METRICS_CADENCE_NS;
            if history
                .last_expired_slot
                .is_some_and(|previous| previous >= slot)
            {
                return;
            }
            history.last_expired_slot = Some(slot);
            let History { series, groups, .. } = history;
            series.retain(|_, series| {
                let latest_slot = series.latest_observed_at_ns / PUBLIC_METRICS_CADENCE_NS;
                let retained = slot.saturating_sub(latest_slot) < PUBLIC_HISTORY_SLOTS as u64;
                if !retained {
                    release(groups, series);
                }
                retained
            });
            history.series.shrink_to_fit();
            history.recount();
        });
    }

    pub fn record(family: PublicMetricFamily, now_ns: u64, metrics: &[PublicMetricSample]) {
        Self::expire(now_ns);
        HISTORY.with_borrow_mut(|history| {
            for metric in metrics {
                let slot = metric.observed_at_ns / PUBLIC_METRICS_CADENCE_NS;
                let now_slot = now_ns / PUBLIC_METRICS_CADENCE_NS;
                if now_slot.saturating_sub(slot) < PUBLIC_HISTORY_SLOTS as u64 {
                    history.record(family, metric);
                }
            }
            history.series.shrink_to_fit();
        });
    }

    #[must_use]
    pub fn series(
        family: PublicMetricFamily,
        name: String,
        canister_id: Option<Principal>,
    ) -> Option<PublicHistorySeries> {
        HISTORY.with_borrow(|history| {
            let series = history.series.get(&SeriesKey {
                family,
                name,
                canister_id,
            })?;
            Some(
                history.groups[series.group]
                    .as_ref()
                    .unwrap()
                    .project(series),
            )
        })
    }

    #[must_use]
    pub fn heap_started_at_ns() -> Option<u64> {
        HISTORY.with_borrow(|history| history.heap_started_at_ns)
    }

    #[must_use]
    pub fn reserved_bytes() -> usize {
        HISTORY.with_borrow(|history| history.reserved_bytes)
    }

    #[must_use]
    pub fn truncated() -> bool {
        HISTORY.with_borrow(|history| history.truncated)
    }
}

// Charge every group cell, including vacant series and uninitialised rows.
const fn group_reservation() -> usize {
    PUBLIC_HISTORY_SLOTS * size_of::<[PublicHistorySample; GROUP_WIDTH]>() + size_of::<Group>()
}

// Charge the bounded group index even when expiration leaves holes.
const fn group_index_reservation() -> usize {
    MAX_GROUPS * size_of::<Option<Group>>()
}

// Copied labels and conservative sparse-index allowance, independently of shared cells.
const fn reservation(key: &SeriesKey, unit: &str) -> usize {
    size_of::<SeriesKey>() + size_of::<StoredSeries>() + key.name.len() + unit.len() + 2048
}
