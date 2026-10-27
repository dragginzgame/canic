//! Prepared-root Registry lifecycle coverage.

#[cfg(test)]
mod audit_root;
mod baseline;
mod build;
mod fixture;
#[cfg(test)]
mod growth;
#[cfg(test)]
mod role_attestation;

pub use baseline::{
    ActiveComponentRegistryFixture, setup_active_component_registry,
    setup_fresh_active_component_registry,
};
#[cfg(test)]
pub(super) use baseline::{
    governed_fleet_journey_cases, governed_pocketic_cases, governed_recovery_cases,
};

#[cfg(test)]
pub(in crate::pic) use baseline::provision_component_request;
pub(in crate::pic) use baseline::{install_configured_root, reset_pool_assets};
