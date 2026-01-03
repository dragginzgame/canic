use crate::{
    PublicError,
    dto::cascade::{StateSnapshotView, TopologySnapshotView},
    workflow,
};

///
/// Cascade API
///

pub async fn sync_state(view: StateSnapshotView) -> Result<(), PublicError> {
    workflow::cascade::state::nonroot_cascade_state(view)
        .await
        .map_err(PublicError::from)
}

pub async fn sync_topology(view: TopologySnapshotView) -> Result<(), PublicError> {
    workflow::cascade::topology::nonroot_cascade_topology(view)
        .await
        .map_err(PublicError::from)
}
