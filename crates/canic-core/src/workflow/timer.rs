use crate::{
    ops::OpsError,
    workflow::{pool::PoolWorkflow, random::RandomWorkflow, runtime::cycles::CycleTrackerWorkflow},
};

///
/// TimerWorkflow
/// Coordinates periodic background services (timers) for Canic canisters.
///

pub struct TimerWorkflow;

impl TimerWorkflow {
    /// Start timers that should run on all canisters.
    pub fn start_all() {
        CycleTrackerWorkflow::start();
        LogWorkflow::start_retention();
        RandomWorkflow::start();
    }

    /// Start timers that should run only on root canisters.
    pub fn start_all_root() {
        OpsError::require_root();

        // start shared timers too
        Self::start_all();

        // root-only services
        PoolWorkflow::start();
    }
}
