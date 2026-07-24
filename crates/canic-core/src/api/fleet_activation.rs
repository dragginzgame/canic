//! Module: api::fleet_activation
//!
//! Responsibility: expose protected Fleet activation diagnostics to endpoint callers.
//! Does not own: storage projection, phase validation, or controller authorization.
//! Boundary: maps the typed internal status failure into Canic's public error contract.

use crate::{
    dto::{error::Error, fleet_activation::FleetActivationStatusResponse},
    workflow::runtime::fleet_activation::FleetActivationWorkflow,
};

///
/// FleetActivationApi
///

pub struct FleetActivationApi;

impl FleetActivationApi {
    pub fn status() -> Result<FleetActivationStatusResponse, Error> {
        FleetActivationWorkflow::status().map_err(Error::from)
    }
}
