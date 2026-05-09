//! ops::ic::mgmt
//!
//! Ops-level wrappers over IC management canister calls.
//! Adds metrics, logging, and normalizes errors into `InternalError`.

mod cycles;
mod lifecycle;
mod snapshots;
mod status_settings;
mod types;

use crate::{
    InternalError,
    ids::SystemMetricKind,
    infra::{InfraError, ic::mgmt::MgmtInfra},
    ops::{
        ic::IcOpsError,
        prelude::*,
        runtime::metrics::{
            canister_ops::{
                CanisterOpsMetricOperation, CanisterOpsMetricOutcome, CanisterOpsMetricReason,
                CanisterOpsMetrics,
            },
            management_call::{
                ManagementCallMetricOperation, ManagementCallMetricOutcome,
                ManagementCallMetricReason, ManagementCallMetrics,
            },
            platform_call::{
                PlatformCallMetricMode, PlatformCallMetricOutcome, PlatformCallMetricReason,
                PlatformCallMetricSurface, PlatformCallMetrics,
            },
            system::SystemMetrics,
        },
    },
};
use std::future::Future;

#[allow(
    unused_imports,
    reason = "part of the public management ops type surface"
)]
pub use types::UpgradeFlags;
pub use types::{
    CanisterInstallMode, CanisterSettings, CanisterSettingsSnapshot, CanisterSnapshot,
    CanisterStatus, CanisterStatusType, EnvironmentVariable, LogVisibility, MemoryMetricsSnapshot,
    QueryStatsSnapshot, UpdateSettingsArgs,
};

use types::{
    canister_snapshot_from_infra, canister_status_from_infra, install_mode_to_infra,
    update_settings_to_infra,
};

///
/// MgmtOps
///

pub struct MgmtOps;

// Execute one management-canister call and record low-cardinality outcomes.
async fn management_call<T>(
    operation: ManagementCallMetricOperation,
    fut: impl Future<Output = Result<T, InfraError>>,
) -> Result<T, InternalError> {
    record_management_call(
        operation,
        PlatformCallMetricOutcome::Started,
        PlatformCallMetricReason::Ok,
        ManagementCallMetricOutcome::Started,
        ManagementCallMetricReason::Ok,
    );

    match fut.await {
        Ok(value) => {
            record_management_call(
                operation,
                PlatformCallMetricOutcome::Completed,
                PlatformCallMetricReason::Ok,
                ManagementCallMetricOutcome::Completed,
                ManagementCallMetricReason::Ok,
            );
            Ok(value)
        }
        Err(err) => {
            record_management_call(
                operation,
                PlatformCallMetricOutcome::Failed,
                PlatformCallMetricReason::Infra,
                ManagementCallMetricOutcome::Failed,
                ManagementCallMetricReason::Infra,
            );
            Err(IcOpsError::from(err).into())
        }
    }
}

// Record management-call metrics with no target or method labels.
fn record_management_call(
    operation: ManagementCallMetricOperation,
    platform_outcome: PlatformCallMetricOutcome,
    platform_reason: PlatformCallMetricReason,
    management_outcome: ManagementCallMetricOutcome,
    management_reason: ManagementCallMetricReason,
) {
    PlatformCallMetrics::record(
        PlatformCallMetricSurface::Management,
        PlatformCallMetricMode::Update,
        platform_outcome,
        platform_reason,
    );
    ManagementCallMetrics::record(operation, management_outcome, management_reason);
}

fn record_unscoped_canister_op(
    operation: CanisterOpsMetricOperation,
    outcome: CanisterOpsMetricOutcome,
    reason: CanisterOpsMetricReason,
) {
    CanisterOpsMetrics::record_unscoped(operation, outcome, reason);
}
