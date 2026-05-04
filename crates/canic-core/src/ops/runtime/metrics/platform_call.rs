use std::{cell::RefCell, collections::HashMap};

thread_local! {
    static PLATFORM_CALL_METRICS: RefCell<HashMap<PlatformCallMetricKey, u64>> =
        RefCell::new(HashMap::new());
}

///
/// PlatformCallMetricSurface
///

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[remain::sorted]
pub enum PlatformCallMetricSurface {
    Ecdsa,
    Generic,
    Http,
    Ledger,
    Management,
    Xrc,
}

impl PlatformCallMetricSurface {
    /// Return the stable public metrics label for this surface.
    #[must_use]
    pub const fn metric_label(self) -> &'static str {
        match self {
            Self::Ecdsa => "ecdsa",
            Self::Generic => "generic",
            Self::Http => "http",
            Self::Ledger => "ledger",
            Self::Management => "management",
            Self::Xrc => "xrc",
        }
    }
}

///
/// PlatformCallMetricMode
///

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[remain::sorted]
pub enum PlatformCallMetricMode {
    BoundedWait,
    LocalVerify,
    Query,
    UnboundedWait,
    Update,
}

impl PlatformCallMetricMode {
    /// Return the stable public metrics label for this mode.
    #[must_use]
    pub const fn metric_label(self) -> &'static str {
        match self {
            Self::BoundedWait => "bounded_wait",
            Self::LocalVerify => "local_verify",
            Self::Query => "query",
            Self::UnboundedWait => "unbounded_wait",
            Self::Update => "update",
        }
    }
}

///
/// PlatformCallMetricOutcome
///

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[remain::sorted]
pub enum PlatformCallMetricOutcome {
    Completed,
    Failed,
    Started,
}

impl PlatformCallMetricOutcome {
    /// Return the stable public metrics label for this outcome.
    #[must_use]
    pub const fn metric_label(self) -> &'static str {
        match self {
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Started => "started",
        }
    }
}

///
/// PlatformCallMetricReason
///

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[remain::sorted]
pub enum PlatformCallMetricReason {
    CandidDecode,
    CandidEncode,
    HttpStatus,
    Infra,
    InvalidPublicKey,
    InvalidSignature,
    LedgerRejected,
    Ok,
    Rejected,
    Unavailable,
}

impl PlatformCallMetricReason {
    /// Return the stable public metrics label for this reason.
    #[must_use]
    pub const fn metric_label(self) -> &'static str {
        match self {
            Self::CandidDecode => "candid_decode",
            Self::CandidEncode => "candid_encode",
            Self::HttpStatus => "http_status",
            Self::Infra => "infra",
            Self::InvalidPublicKey => "invalid_public_key",
            Self::InvalidSignature => "invalid_signature",
            Self::LedgerRejected => "ledger_rejected",
            Self::Ok => "ok",
            Self::Rejected => "rejected",
            Self::Unavailable => "unavailable",
        }
    }
}

///
/// PlatformCallMetricKey
///

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct PlatformCallMetricKey {
    pub surface: PlatformCallMetricSurface,
    pub mode: PlatformCallMetricMode,
    pub outcome: PlatformCallMetricOutcome,
    pub reason: PlatformCallMetricReason,
}

///
/// PlatformCallMetrics
///

pub struct PlatformCallMetrics;

impl PlatformCallMetrics {
    /// Record one platform call event.
    pub fn record(
        surface: PlatformCallMetricSurface,
        mode: PlatformCallMetricMode,
        outcome: PlatformCallMetricOutcome,
        reason: PlatformCallMetricReason,
    ) {
        PLATFORM_CALL_METRICS.with_borrow_mut(|counts| {
            let key = PlatformCallMetricKey {
                surface,
                mode,
                outcome,
                reason,
            };
            let entry = counts.entry(key).or_insert(0);
            *entry = entry.saturating_add(1);
        });
    }

    /// Snapshot the current platform call metric table as stable rows.
    #[must_use]
    pub fn snapshot() -> Vec<(PlatformCallMetricKey, u64)> {
        PLATFORM_CALL_METRICS
            .with_borrow(std::clone::Clone::clone)
            .into_iter()
            .collect()
    }

    /// Test-only helper: clear all platform call metrics.
    #[cfg(test)]
    pub fn reset() {
        PLATFORM_CALL_METRICS.with_borrow_mut(HashMap::clear);
    }
}

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::*;

    // Convert snapshots into a map for concise count assertions.
    fn snapshot_map() -> HashMap<PlatformCallMetricKey, u64> {
        PlatformCallMetrics::snapshot().into_iter().collect()
    }

    #[test]
    fn platform_call_metrics_accumulate_by_surface_mode_outcome_and_reason() {
        PlatformCallMetrics::reset();

        PlatformCallMetrics::record(
            PlatformCallMetricSurface::Generic,
            PlatformCallMetricMode::BoundedWait,
            PlatformCallMetricOutcome::Started,
            PlatformCallMetricReason::Ok,
        );
        PlatformCallMetrics::record(
            PlatformCallMetricSurface::Generic,
            PlatformCallMetricMode::BoundedWait,
            PlatformCallMetricOutcome::Started,
            PlatformCallMetricReason::Ok,
        );
        PlatformCallMetrics::record(
            PlatformCallMetricSurface::Ledger,
            PlatformCallMetricMode::Update,
            PlatformCallMetricOutcome::Failed,
            PlatformCallMetricReason::LedgerRejected,
        );

        let map = snapshot_map();
        assert_eq!(
            map.get(&PlatformCallMetricKey {
                surface: PlatformCallMetricSurface::Generic,
                mode: PlatformCallMetricMode::BoundedWait,
                outcome: PlatformCallMetricOutcome::Started,
                reason: PlatformCallMetricReason::Ok,
            }),
            Some(&2)
        );
        assert_eq!(
            map.get(&PlatformCallMetricKey {
                surface: PlatformCallMetricSurface::Ledger,
                mode: PlatformCallMetricMode::Update,
                outcome: PlatformCallMetricOutcome::Failed,
                reason: PlatformCallMetricReason::LedgerRejected,
            }),
            Some(&1)
        );
    }
}
