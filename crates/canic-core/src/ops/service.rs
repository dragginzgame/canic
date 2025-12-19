use crate::{
    Error,
    ops::{
        OpsError,
        random::RandomOps,
        reserve::ReserveOps,
        runtime::{cycles::CycleTrackerOps, log::LogOps},
        storage::env::EnvOps,
    },
};

///
/// TimerService
/// Coordinates periodic background services (timers) for Canic canisters.
///

pub struct TimerService;

impl TimerService {
    /// Start timers that should run on all canisters.
    pub fn start_all() -> Result<(), Error> {
        // Ensure env is initialized (subnet type present) before starting timers.
        EnvOps::try_get_subnet_role()?;

        CycleTrackerOps::start();
        LogOps::start_retention();
        RandomOps::start();

        Ok(())
    }

    /// Start timers that should run only on root canisters.
    pub fn start_all_root() -> Result<(), Error> {
        OpsError::require_root()?;

        // start shared timers too
        Self::start_all()?;

        // root-only services
        ReserveOps::start();

        Ok(())
    }
}
