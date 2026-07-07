//! Module: workflow::pool::reset
//!
//! Responsibility: reset one canister into parked pool state.
//! Does not own: pool admission policy, stable pool records, or endpoint authorization.
//! Boundary: workflow helper coordinating management calls and reset metrics.

use crate::{
    InternalError,
    cdk::types::{Cycles, Principal},
    ops::{
        ic::mgmt::{CanisterSettings, MgmtOps, UpdateSettingsArgs},
        runtime::metrics::{
            pool::{PoolMetricOperation as MetricOperation, PoolMetricReason as MetricReason},
            recording::PoolMetricEvent as MetricEvent,
        },
    },
    workflow::pool::PoolWorkflow,
};

impl PoolWorkflow {
    pub async fn reset_into_pool(pid: Principal) -> Result<Cycles, InternalError> {
        MetricEvent::started(MetricOperation::Reset);
        let controllers = match Self::pool_controllers() {
            Ok(controllers) => controllers,
            Err(err) => {
                MetricEvent::failed(MetricOperation::Reset, &err);
                return Err(err);
            }
        };

        if let Err(err) = MgmtOps::update_settings(&UpdateSettingsArgs {
            canister_id: pid,
            settings: CanisterSettings {
                controllers: Some(controllers),
                ..Default::default()
            },
            sender_canister_version: None,
        })
        .await
        {
            MetricEvent::failed(MetricOperation::Reset, &err);
            return Err(err);
        }

        if let Err(err) = MgmtOps::uninstall_code(pid).await {
            MetricEvent::failed(MetricOperation::Reset, &err);
            return Err(err);
        }

        match MgmtOps::get_cycles(pid).await {
            Ok(cycles) => {
                MetricEvent::completed(MetricOperation::Reset, MetricReason::Ok);
                Ok(cycles)
            }
            Err(err) => {
                MetricEvent::failed(MetricOperation::Reset, &err);
                Err(err)
            }
        }
    }
}
