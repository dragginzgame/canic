use crate::{
    Error,
    dto::cascade::{StateSnapshotView, TopologySnapshotView},
    ops::{prelude::*, rpc::RpcOps},
    protocol,
};

///
/// CascadeOps
///

pub struct CascadeOps;

impl CascadeOps {
    pub async fn send_state_snapshot(
        pid: Principal,
        snapshot: &StateSnapshotView,
    ) -> Result<(), Error> {
        RpcOps::call_rpc_result::<()>(pid, protocol::CANIC_SYNC_STATE, snapshot).await
    }

    pub async fn send_topology_snapshot(
        pid: Principal,
        snapshot: &TopologySnapshotView,
    ) -> Result<(), Error> {
        RpcOps::call_rpc_result::<()>(pid, protocol::CANIC_SYNC_TOPOLOGY, snapshot).await
    }
}
