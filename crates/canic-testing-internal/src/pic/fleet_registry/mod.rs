//! Prepared-root Registry lifecycle coverage.

mod baseline;
mod build;
mod fixture;
#[cfg(test)]
mod role_attestation;

pub use baseline::{ActiveComponentRegistryFixture, setup_active_component_registry};
