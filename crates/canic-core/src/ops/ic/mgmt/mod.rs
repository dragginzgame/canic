//! Module: ops::ic::mgmt
//!
//! Responsibility: expose observable IC management canister calls.
//! Does not own: canister lifecycle policy, placement decisions, or endpoint DTOs.
//! Boundary: records metrics and delegates management call mechanics to infra.

mod cycles;
mod lifecycle;
mod signing;
mod status_settings;
mod types;

use crate::{
    InternalError,
    domain::metrics::{
        ManagementCallMetricOperation, ManagementCallMetricOutcome, ManagementCallMetricReason,
        PlatformCallMetricMode, PlatformCallMetricOutcome, PlatformCallMetricReason,
        PlatformCallMetricSurface,
    },
    dto::canister::{
        CanisterSettings as CanisterSettingsDto, CanisterStatusResponse,
        EnvironmentVariable as EnvironmentVariableDto, MemoryMetrics, QueryStats,
    },
    ids::SystemMetricKind,
    infra::{InfraError, ic::mgmt::MgmtInfra},
    ops::{
        ic::IcOpsError,
        prelude::*,
        runtime::metrics::{
            management_call::ManagementCallMetrics, platform_call::PlatformCallMetrics,
            system::SystemMetrics,
        },
    },
};
use std::future::Future;

#[expect(
    unused_imports,
    reason = "part of the public management ops type surface"
)]
pub use types::UpgradeFlags;
pub use types::{
    CanisterInstallMode, CanisterSettings, CanisterSettingsSnapshot, CanisterStatus, EcdsaKeyId,
    EcdsaPublicKeyArgs, EcdsaPublicKeyResult, EnvironmentVariable, MemoryMetricsSnapshot,
    QueryStatsSnapshot, SignWithEcdsaArgs, SignWithEcdsaResult, UpdateSettingsArgs,
};
#[expect(
    unused_imports,
    reason = "preserves public management ops value paths while domain owns the values"
)]
pub use types::{CanisterStatusType, LogVisibility};

use types::{
    canister_status_from_infra, ecdsa_public_key_args_to_infra, ecdsa_public_key_from_infra,
    install_mode_to_infra, sign_with_ecdsa_args_to_infra, sign_with_ecdsa_from_infra,
    update_settings_to_infra,
};

///
/// MgmtOps
///
/// Operations-layer facade for IC management canister calls.
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
