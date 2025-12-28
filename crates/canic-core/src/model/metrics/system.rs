use std::{cell::RefCell, collections::HashMap};

thread_local! {
    static SYSTEM_METRICS: RefCell<HashMap<SystemMetricKind, u64>> = RefCell::new(HashMap::new());
}

///
/// SystemMetricKind
/// Enumerates the resource-heavy actions we track.
///

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[remain::sorted]
pub enum SystemMetricKind {
    CanisterCall,
    CanisterStatus,
    CreateCanister,
    DeleteCanister,
    DepositCycles,
    HttpOutcall,
    InstallCode,
    ReinstallCode,
    TimerScheduled,
    UninstallCode,
    UpdateSettings,
    UpgradeCode,
}

///
/// SystemMetrics
/// Thin facade over the action metrics counters.
///

pub struct SystemMetrics;

impl SystemMetrics {
    /// Increment a counter and return the new value.
    pub fn increment(kind: SystemMetricKind) {
        SYSTEM_METRICS.with_borrow_mut(|counts| {
            let entry = counts.entry(kind).or_insert(0);
            *entry = entry.saturating_add(1);
        });
    }

    #[must_use]
    pub fn export_raw() -> HashMap<SystemMetricKind, u64> {
        SYSTEM_METRICS.with_borrow(|counts| counts.clone())
    }

    #[cfg(test)]
    pub fn reset() {
        SYSTEM_METRICS.with_borrow_mut(HashMap::clear);
    }
}
