//! Module: api::state
//!
//! Responsibility: expose root Fleet-state mutation through control-plane authority.
//! Does not own: endpoint authorization, state records, or cascade transport.
//! Boundary: maps the root workflow result into the public error envelope.

use crate::workflow::state::FleetStateWorkflow;
use canic_core::dto::{
    error::Error,
    state::{FleetCommand, FleetCommandResponse},
};

///
/// FleetStateApi
///
/// Root control-plane facade for Fleet-state mutation and exact child fanout.
///

pub struct FleetStateApi;

impl FleetStateApi {
    pub async fn execute_command(cmd: FleetCommand) -> Result<FleetCommandResponse, Error> {
        FleetStateWorkflow::execute_command(cmd)
            .await
            .map_err(Error::from)
    }
}
