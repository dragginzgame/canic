use crate::{
    PublicError,
    dto::cascade::{StateSnapshotView, TopologySnapshotView},
    workflow::cascade::{state::StateCascadeWorkflow, topology::TopologyCascadeWorkflow},
};

///
/// CascadeApi
///

pub struct CascadeApi;

impl CascadeApi {
    pub async fn sync_state(view: StateSnapshotView) -> Result<(), PublicError> {
        StateCascadeWorkflow::nonroot_cascade_state(view)
            .await
            .map_err(PublicError::from)
    }

    pub async fn sync_topology(view: TopologySnapshotView) -> Result<(), PublicError> {
        TopologyCascadeWorkflow::nonroot_cascade_topology(view)
            .await
            .map_err(PublicError::from)
    }
}
