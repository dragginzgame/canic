use std::{cell::RefCell, collections::HashMap};

thread_local! {
    static LIFECYCLE_METRICS: RefCell<HashMap<LifecycleMetricKey, u64>> =
        RefCell::new(HashMap::new());
}

///
/// LifecycleMetricPhase
///

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[remain::sorted]
pub enum LifecycleMetricPhase {
    Init,
    PostUpgrade,
}

impl LifecycleMetricPhase {
    pub const fn metric_label(self) -> &'static str {
        match self {
            Self::Init => "init",
            Self::PostUpgrade => "post_upgrade",
        }
    }
}

///
/// LifecycleMetricRole
///

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[remain::sorted]
pub enum LifecycleMetricRole {
    Nonroot,
    Root,
}

impl LifecycleMetricRole {
    pub const fn metric_label(self) -> &'static str {
        match self {
            Self::Nonroot => "nonroot",
            Self::Root => "root",
        }
    }
}

///
/// LifecycleMetricStage
///

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[remain::sorted]
pub enum LifecycleMetricStage {
    Bootstrap,
    Runtime,
}

impl LifecycleMetricStage {
    pub const fn metric_label(self) -> &'static str {
        match self {
            Self::Bootstrap => "bootstrap",
            Self::Runtime => "runtime",
        }
    }
}

///
/// LifecycleMetricOutcome
///

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[remain::sorted]
pub enum LifecycleMetricOutcome {
    Completed,
    Failed,
    Scheduled,
    Skipped,
    Started,
    Waiting,
}

impl LifecycleMetricOutcome {
    pub const fn metric_label(self) -> &'static str {
        match self {
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Scheduled => "scheduled",
            Self::Skipped => "skipped",
            Self::Started => "started",
            Self::Waiting => "waiting",
        }
    }
}

///
/// LifecycleMetricKey
///

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct LifecycleMetricKey {
    pub phase: LifecycleMetricPhase,
    pub role: LifecycleMetricRole,
    pub stage: LifecycleMetricStage,
    pub outcome: LifecycleMetricOutcome,
}

///
/// LifecycleMetrics
///

pub struct LifecycleMetrics;

impl LifecycleMetrics {
    /// Record one lifecycle stage event.
    pub fn record(
        phase: LifecycleMetricPhase,
        role: LifecycleMetricRole,
        stage: LifecycleMetricStage,
        outcome: LifecycleMetricOutcome,
    ) {
        LIFECYCLE_METRICS.with_borrow_mut(|counts| {
            let key = LifecycleMetricKey {
                phase,
                role,
                stage,
                outcome,
            };
            let entry = counts.entry(key).or_insert(0);
            *entry = entry.saturating_add(1);
        });
    }

    /// Snapshot the current lifecycle metric table as stable rows.
    #[must_use]
    pub fn snapshot() -> Vec<(LifecycleMetricKey, u64)> {
        LIFECYCLE_METRICS
            .with_borrow(std::clone::Clone::clone)
            .into_iter()
            .collect()
    }

    /// Test-only helper: clear all lifecycle metrics.
    #[cfg(test)]
    pub fn reset() {
        LIFECYCLE_METRICS.with_borrow_mut(HashMap::clear);
    }
}
