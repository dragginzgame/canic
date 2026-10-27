//! Exact next hook projected from retained Root checkpoint state.

use canic_contracts::ids::{ApplicationCheckpointBinding, ApplicationCheckpointPhase};

/// One idempotent hook and the phase its exact acknowledgement must establish.
pub struct RootCheckpointStep {
    pub binding: ApplicationCheckpointBinding,
    pub expected: ApplicationCheckpointPhase,
}
