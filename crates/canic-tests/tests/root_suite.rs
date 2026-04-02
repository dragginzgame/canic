// Category C - Artifact / deployment test (embedded config).
// This test relies on embedded production config by design.

mod root;

#[path = "root_cases/hierarchy.rs"]
mod root_hierarchy_cases;

#[path = "root_cases/replay.rs"]
mod root_replay_cases;
