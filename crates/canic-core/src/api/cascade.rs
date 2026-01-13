use crate::{
    Error,
    dto::cascade::{StateSnapshotView, TopologySnapshotView},
    workflow::cascade::{state::StateCascadeWorkflow, topology::TopologyCascadeWorkflow},
};

///
/// CascadeApi
///

pub struct CascadeApi;

impl CascadeApi {
    pub async fn sync_state(view: StateSnapshotView) -> Result<(), Error> {
        StateCascadeWorkflow::nonroot_cascade_state(view)
            .await
            .map_err(Error::from)
    }

    pub async fn sync_topology(view: TopologySnapshotView) -> Result<(), Error> {
        TopologyCascadeWorkflow::nonroot_cascade_topology(view)
            .await
            .map_err(Error::from)
    }
}
