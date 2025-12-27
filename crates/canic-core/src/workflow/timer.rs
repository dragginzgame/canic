use crate::{ops::OpsError, workflow};

///
/// TimerWorkflow
/// Coordinates periodic background services (timers) for Canic canisters.
///

pub struct TimerWorkflow;

impl TimerWorkflow {
    /// Start timers that should run on all canisters.
    pub fn start_all() {
        workflow::runtime::scheduler::start();
        workflow::log::scheduler::start_retention();
        workflow::random::scheduler::start();
    }

    /// Start timers that should run only on root canisters.
    pub fn start_all_root() {
        OpsError::require_root();

        // start shared timers too
        Self::start_all();

        // root-only services
        workflow::pool::scheduler::start();
    }
}
