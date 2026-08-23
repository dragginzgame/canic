//! Prepared-root Registry lifecycle coverage.

mod baseline;
mod build;
mod fixture;
#[cfg(test)]
mod role_attestation;

#[cfg(test)]
pub(super) use baseline::governed_pocketic_cases;
pub use baseline::{
    ActiveComponentRegistryFixture, setup_active_component_registry,
    setup_fresh_active_component_registry,
};
