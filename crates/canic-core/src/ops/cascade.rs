use crate::{
    InternalError,
    ops::{prelude::*, rpc::RpcOps},
    protocol,
};

///
/// CascadeOps
///

pub struct CascadeOps;

impl CascadeOps {
    pub async fn send_state_snapshot<S: CandidType>(
        pid: Principal,
        snapshot: S,
    ) -> Result<(), InternalError> {
        RpcOps::call_rpc_result::<()>(pid, protocol::CANIC_SYNC_STATE, snapshot).await
    }

    pub async fn send_topology_snapshot<S: CandidType>(
        pid: Principal,
        snapshot: S,
    ) -> Result<(), InternalError> {
        RpcOps::call_rpc_result::<()>(pid, protocol::CANIC_SYNC_TOPOLOGY, snapshot).await
    }
}
