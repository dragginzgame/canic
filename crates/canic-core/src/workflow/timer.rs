use crate::{
    ops::OpsError,
    workflow::{
        pool::PoolOps,
        random::RandomOps,
        runtime::{cycles::CycleTrackerOps, log::LogOps},
    },
};

///
/// TimerService
/// Coordinates periodic background services (timers) for Canic canisters.
///

pub struct TimerService;

impl TimerService {
    /// Start timers that should run on all canisters.
    pub fn start_all() {
        CycleTrackerOps::start();
        LogOps::start_retention();
        RandomOps::start();
    }

    /// Start timers that should run only on root canisters.
    pub fn start_all_root() {
        OpsError::require_root();

        // start shared timers too
        Self::start_all();

        // root-only services
        PoolOps::start();
    }
}
