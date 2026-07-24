use crate::{
    dto::{
        error::Error,
        state::{FleetCommand, FleetCommandResponse},
    },
    workflow::state::FleetStateWorkflow,
};

/// Re-export of read-only state query surfaces.
pub use crate::workflow::state::query::FleetStateQuery;

///
/// FleetStateApi
///

pub struct FleetStateApi;

impl FleetStateApi {
    pub async fn execute_command(cmd: FleetCommand) -> Result<FleetCommandResponse, Error> {
        FleetStateWorkflow::execute_command(cmd)
            .await
            .map_err(Error::from)
    }
}
