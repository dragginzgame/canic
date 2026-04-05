// Category C - Artifact / deployment test (embedded config).
// This test relies on embedded production config by design.

mod root;

#[path = "root_cases/hierarchy.rs"]
mod root_hierarchy_cases;

#[path = "root_cases/scaling.rs"]
mod root_scaling_cases;

#[path = "root_cases/sharding.rs"]
mod root_sharding_cases;

#[path = "root_cases/replay.rs"]
mod root_replay_cases;
