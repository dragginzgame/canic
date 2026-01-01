use crate::{
    Error,
    cdk::types::Principal,
    dto::snapshot::{StateSnapshotView, TopologySnapshotView},
    ops::rpc::{call_rpc_result, methods},
};

pub async fn send_state_snapshot(
    pid: Principal,
    snapshot: &StateSnapshotView,
) -> Result<(), Error> {
    call_rpc_result::<()>(pid, methods::CANIC_SYNC_STATE, snapshot).await
}

pub async fn send_topology_snapshot(
    pid: Principal,
    snapshot: &TopologySnapshotView,
) -> Result<(), Error> {
    call_rpc_result::<()>(pid, methods::CANIC_SYNC_TOPOLOGY, snapshot).await
}
