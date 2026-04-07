// Category C - Artifact / deployment test (embedded config).
// This test relies on embedded production config by design.

#![allow(dead_code)]
pub mod assertions;
pub mod harness;
mod profile;
pub mod workers;

pub use profile::RootSetupProfile;
