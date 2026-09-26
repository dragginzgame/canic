//! Passive inputs for supplied-infrastructure review after current artifact generation.

use crate::fleet_ensure::model::{
    DesiredFleet, infrastructure_bootstrap::BootstrapCoordinatorSelection,
};
use std::path::Path;

/// Exact generated authority, disposition declarations and durable identity destination.
pub struct InfrastructureBootstrapReviewRequest<'a> {
    pub workspace: &'a Path,
    pub desired: &'a DesiredFleet,
    pub coordinator: BootstrapCoordinatorSelection,
    pub declarations_toml: &'a str,
    pub seed: &'a Path,
    pub planned_at_time: u64,
}
