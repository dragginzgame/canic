//! Qualification-only access to the actual reservation workflow and request types.
//! Does not substitute storage, timers, capacity decisions or settlement behavior.

pub use crate::{
    model::replay::CommandKind,
    ops::cost_guard::CostGuardRequest,
    replay_policy::CostClass,
    workflow::cost_guard::{CostGuardWorkflow, map_cost_guard_reserve_error},
};

/// Exercise the same admission flag restored by the authority snapshot lifecycle.
/// This does not fabricate a durable snapshot or replace the native timer provider.
pub fn restore_suspension(sealed: bool) {
    crate::workflow::runtime::timer::TimerAuthorityWorkflow::restore_snapshot_suspension(sealed);
}
