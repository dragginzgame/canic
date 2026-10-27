//! Lossless native state receipt conversions; no mutation or reconciliation policy.
use crate::dto::{
    cascade as stored, fleet_state_control as native, state::FleetStateCommandResult,
};

impl From<stored::StateCascadeReport> for native::CascadeReport {
    fn from(value: stored::StateCascadeReport) -> Self {
        Self {
            successful_targets: value.successful_targets,
            reconciliation_failures: value.reconciliation_failures,
            unconfirmed_targets: value.unconfirmed_targets,
            omitted_targets: value.omitted_targets,
            targets: value
                .targets
                .into_iter()
                .map(|target| native::TargetOutcome {
                    canister_id: target.canister_id.into(),
                    result: target.result,
                })
                .collect(),
        }
    }
}
impl From<native::CascadeReport> for stored::StateCascadeReport {
    fn from(value: native::CascadeReport) -> Self {
        Self {
            successful_targets: value.successful_targets,
            reconciliation_failures: value.reconciliation_failures,
            unconfirmed_targets: value.unconfirmed_targets,
            omitted_targets: value.omitted_targets,
            targets: value
                .targets
                .into_iter()
                .map(|target| stored::StateCascadeTargetOutcome {
                    canister_id: target.canister_id.into(),
                    result: target.result,
                })
                .collect(),
        }
    }
}
impl<T> From<FleetStateCommandResult<T>> for native::CommandResult<T> {
    fn from(value: FleetStateCommandResult<T>) -> Self {
        Self {
            change: value.change,
            propagation: value.propagation.into(),
            reconciliation_error: value.reconciliation_error,
        }
    }
}
impl<T> From<native::CommandResult<T>> for FleetStateCommandResult<T> {
    fn from(value: native::CommandResult<T>) -> Self {
        Self {
            change: value.change,
            propagation: value.propagation.into(),
            reconciliation_error: value.reconciliation_error,
        }
    }
}
