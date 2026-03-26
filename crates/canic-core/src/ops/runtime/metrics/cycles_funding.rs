use crate::cdk::types::Principal;
use std::{cell::RefCell, collections::HashMap};

thread_local! {
    static CYCLES_FUNDING_METRICS: RefCell<HashMap<CyclesFundingMetricStorageKey, u128>> =
        RefCell::new(HashMap::new());
}

///
/// CyclesFundingMetricKey
///

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub enum CyclesFundingMetricKey {
    RequestedTotal,
    GrantedTotal,
    DeniedTotal,
    RequestedByChild,
    GrantedToChild,
    DeniedToChild,
    DeniedGlobalKillSwitch,
}

impl CyclesFundingMetricKey {
    #[must_use]
    pub const fn metric_label(self) -> &'static str {
        match self {
            Self::RequestedTotal => "cycles_requested_total",
            Self::GrantedTotal => "cycles_granted_total",
            Self::DeniedTotal => "cycles_denied_total",
            Self::RequestedByChild => "cycles_requested_by_child",
            Self::GrantedToChild => "cycles_granted_to_child",
            Self::DeniedToChild => "cycles_denied_to_child",
            Self::DeniedGlobalKillSwitch => "cycles_denied_global_kill_switch",
        }
    }
}

///
/// CyclesFundingDeniedReason
///

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub enum CyclesFundingDeniedReason {
    ChildNotFound,
    NotDirectChild,
    KillSwitchDisabled,
    InsufficientCycles,
    MaxPerChildExceeded,
    CooldownActive,
    ExecutionError,
}

impl CyclesFundingDeniedReason {
    #[must_use]
    pub const fn metric_label(self) -> &'static str {
        match self {
            Self::ChildNotFound => "child_not_found",
            Self::NotDirectChild => "not_direct_child",
            Self::KillSwitchDisabled => "kill_switch_disabled",
            Self::InsufficientCycles => "insufficient_cycles",
            Self::MaxPerChildExceeded => "max_per_child_exceeded",
            Self::CooldownActive => "cooldown_active",
            Self::ExecutionError => "execution_error",
        }
    }
}

///
/// CyclesFundingMetricStorageKey
///

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
struct CyclesFundingMetricStorageKey {
    metric: CyclesFundingMetricKey,
    child_principal: Option<Principal>,
    reason: Option<CyclesFundingDeniedReason>,
}

///
/// CyclesFundingMetrics
///

pub struct CyclesFundingMetrics;

impl CyclesFundingMetrics {
    // Record a single metrics point.
    fn record(
        metric: CyclesFundingMetricKey,
        child_principal: Option<Principal>,
        reason: Option<CyclesFundingDeniedReason>,
        cycles: u128,
    ) {
        CYCLES_FUNDING_METRICS.with_borrow_mut(|counts| {
            let key = CyclesFundingMetricStorageKey {
                metric,
                child_principal,
                reason,
            };
            let entry = counts.entry(key).or_insert(0);
            *entry = entry.saturating_add(cycles);
        });
    }

    // Record accepted funding demand from a child.
    pub fn record_requested(child_principal: Principal, cycles: u128) {
        Self::record(CyclesFundingMetricKey::RequestedTotal, None, None, cycles);
        Self::record(
            CyclesFundingMetricKey::RequestedByChild,
            Some(child_principal),
            None,
            cycles,
        );
    }

    // Record successful funding granted to a child.
    pub fn record_granted(child_principal: Principal, cycles: u128) {
        Self::record(CyclesFundingMetricKey::GrantedTotal, None, None, cycles);
        Self::record(
            CyclesFundingMetricKey::GrantedToChild,
            Some(child_principal),
            None,
            cycles,
        );
    }

    // Record denied funding and denial reason.
    pub fn record_denied(
        child_principal: Principal,
        cycles: u128,
        reason: CyclesFundingDeniedReason,
    ) {
        Self::record(CyclesFundingMetricKey::DeniedTotal, None, None, cycles);
        Self::record(
            CyclesFundingMetricKey::DeniedToChild,
            Some(child_principal),
            Some(reason),
            cycles,
        );

        if reason == CyclesFundingDeniedReason::KillSwitchDisabled {
            Self::record(
                CyclesFundingMetricKey::DeniedGlobalKillSwitch,
                None,
                Some(reason),
                cycles,
            );
        }
    }

    #[must_use]
    pub fn snapshot() -> Vec<(
        CyclesFundingMetricKey,
        Option<Principal>,
        Option<CyclesFundingDeniedReason>,
        u128,
    )> {
        CYCLES_FUNDING_METRICS
            .with_borrow(std::clone::Clone::clone)
            .into_iter()
            .map(|(key, cycles)| (key.metric, key.child_principal, key.reason, cycles))
            .collect()
    }

    #[cfg(test)]
    pub fn reset() {
        CYCLES_FUNDING_METRICS.with_borrow_mut(HashMap::clear);
    }
}

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn snapshot_map() -> HashMap<
        (
            CyclesFundingMetricKey,
            Option<Principal>,
            Option<CyclesFundingDeniedReason>,
        ),
        u128,
    > {
        CyclesFundingMetrics::snapshot()
            .into_iter()
            .map(|(metric, child_principal, reason, cycles)| {
                ((metric, child_principal, reason), cycles)
            })
            .collect()
    }

    #[test]
    fn requested_and_granted_track_total_and_child_scopes() {
        CyclesFundingMetrics::reset();
        let child = p(7);

        CyclesFundingMetrics::record_requested(child, 10);
        CyclesFundingMetrics::record_requested(child, 15);
        CyclesFundingMetrics::record_granted(child, 20);

        let map = snapshot_map();
        assert_eq!(
            map.get(&(CyclesFundingMetricKey::RequestedTotal, None, None)),
            Some(&25)
        );
        assert_eq!(
            map.get(&(CyclesFundingMetricKey::RequestedByChild, Some(child), None)),
            Some(&25)
        );
        assert_eq!(
            map.get(&(CyclesFundingMetricKey::GrantedTotal, None, None)),
            Some(&20)
        );
        assert_eq!(
            map.get(&(CyclesFundingMetricKey::GrantedToChild, Some(child), None)),
            Some(&20)
        );
    }

    #[test]
    fn denied_tracks_reason_and_global_kill_switch_amount() {
        CyclesFundingMetrics::reset();
        let child = p(11);

        CyclesFundingMetrics::record_denied(child, 30, CyclesFundingDeniedReason::NotDirectChild);
        CyclesFundingMetrics::record_denied(
            child,
            40,
            CyclesFundingDeniedReason::KillSwitchDisabled,
        );

        let map = snapshot_map();
        assert_eq!(
            map.get(&(CyclesFundingMetricKey::DeniedTotal, None, None)),
            Some(&70)
        );
        assert_eq!(
            map.get(&(
                CyclesFundingMetricKey::DeniedToChild,
                Some(child),
                Some(CyclesFundingDeniedReason::NotDirectChild),
            )),
            Some(&30)
        );
        assert_eq!(
            map.get(&(
                CyclesFundingMetricKey::DeniedToChild,
                Some(child),
                Some(CyclesFundingDeniedReason::KillSwitchDisabled),
            )),
            Some(&40)
        );
        assert_eq!(
            map.get(&(
                CyclesFundingMetricKey::DeniedGlobalKillSwitch,
                None,
                Some(CyclesFundingDeniedReason::KillSwitchDisabled),
            )),
            Some(&40)
        );
    }
}
