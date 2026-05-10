use std::{cell::RefCell, collections::HashMap};

thread_local! {
    static MANAGEMENT_CALL_METRICS: RefCell<HashMap<ManagementCallMetricKey, u64>> =
        RefCell::new(HashMap::new());
}

///
/// ManagementCallMetricOperation
///

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[remain::sorted]
pub enum ManagementCallMetricOperation {
    CanisterStatus,
    ClearChunkStore,
    CreateCanister,
    DeleteCanister,
    DepositCycles,
    GetCycles,
    InstallChunkedCode,
    InstallCode,
    LoadCanisterSnapshot,
    RawRand,
    StopCanister,
    StoredChunks,
    TakeCanisterSnapshot,
    UninstallCode,
    UpdateSettings,
    UploadChunk,
}

///
/// ManagementCallMetricOutcome
///

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[remain::sorted]
pub enum ManagementCallMetricOutcome {
    Completed,
    Failed,
    Started,
}

///
/// ManagementCallMetricReason
///

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[remain::sorted]
pub enum ManagementCallMetricReason {
    Infra,
    Ok,
}

///
/// ManagementCallMetricKey
///

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct ManagementCallMetricKey {
    pub operation: ManagementCallMetricOperation,
    pub outcome: ManagementCallMetricOutcome,
    pub reason: ManagementCallMetricReason,
}

///
/// ManagementCallMetrics
///

pub struct ManagementCallMetrics;

impl ManagementCallMetrics {
    /// Record one management-canister call outcome.
    pub fn record(
        operation: ManagementCallMetricOperation,
        outcome: ManagementCallMetricOutcome,
        reason: ManagementCallMetricReason,
    ) {
        MANAGEMENT_CALL_METRICS.with_borrow_mut(|counts| {
            let key = ManagementCallMetricKey {
                operation,
                outcome,
                reason,
            };
            let entry = counts.entry(key).or_insert(0);
            *entry = entry.saturating_add(1);
        });
    }

    /// Snapshot the current management-call metric table as stable rows.
    #[must_use]
    #[cfg(test)]
    pub fn snapshot() -> Vec<(ManagementCallMetricKey, u64)> {
        MANAGEMENT_CALL_METRICS
            .with_borrow(std::clone::Clone::clone)
            .into_iter()
            .collect()
    }

    /// Test-only helper: clear all management-call metrics.
    #[cfg(test)]
    pub fn reset() {
        MANAGEMENT_CALL_METRICS.with_borrow_mut(HashMap::clear);
    }
}

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::*;

    // Convert snapshots into a map for concise count assertions.
    fn snapshot_map() -> HashMap<ManagementCallMetricKey, u64> {
        ManagementCallMetrics::snapshot().into_iter().collect()
    }

    // Verify management-call counters accumulate by operation, outcome, and reason.
    #[test]
    fn management_call_metrics_accumulate_by_operation_outcome_and_reason() {
        ManagementCallMetrics::reset();

        ManagementCallMetrics::record(
            ManagementCallMetricOperation::InstallCode,
            ManagementCallMetricOutcome::Started,
            ManagementCallMetricReason::Ok,
        );
        ManagementCallMetrics::record(
            ManagementCallMetricOperation::InstallCode,
            ManagementCallMetricOutcome::Failed,
            ManagementCallMetricReason::Infra,
        );
        ManagementCallMetrics::record(
            ManagementCallMetricOperation::InstallCode,
            ManagementCallMetricOutcome::Failed,
            ManagementCallMetricReason::Infra,
        );

        let map = snapshot_map();
        assert_eq!(
            map.get(&ManagementCallMetricKey {
                operation: ManagementCallMetricOperation::InstallCode,
                outcome: ManagementCallMetricOutcome::Started,
                reason: ManagementCallMetricReason::Ok,
            }),
            Some(&1)
        );
        assert_eq!(
            map.get(&ManagementCallMetricKey {
                operation: ManagementCallMetricOperation::InstallCode,
                outcome: ManagementCallMetricOutcome::Failed,
                reason: ManagementCallMetricReason::Infra,
            }),
            Some(&2)
        );
    }
}
