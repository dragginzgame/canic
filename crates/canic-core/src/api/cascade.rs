use crate::{
    dto::{
        cascade::{StateSnapshotInput, TopologySnapshotInput},
        error::Error,
    },
    workflow::cascade::{state::StateCascadeWorkflow, topology::TopologyCascadeWorkflow},
};

///
/// CascadeApi
///

pub struct CascadeApi;

impl CascadeApi {
    pub async fn sync_state(view: StateSnapshotInput) -> Result<(), Error> {
        StateCascadeWorkflow::nonroot_cascade_state(view)
            .await
            .map_err(Error::from)
    }

    pub async fn sync_topology(view: TopologySnapshotInput) -> Result<(), Error> {
        TopologyCascadeWorkflow::nonroot_cascade_topology(view)
            .await
            .map_err(Error::from)
    }
}
