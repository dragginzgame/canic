//! Module: ops::cascade
//!
//! Responsibility: send state and topology cascade snapshots through RPC.
//! Does not own: cascade workflow decisions, snapshot construction, or endpoint auth.
//! Boundary: ops wrapper around the RPC transport for cascade message names.

use crate::{
    InternalError,
    ops::{prelude::*, rpc::RpcOps},
    protocol,
};

///
/// CascadeOps
///
/// Operations-layer facade for cascade snapshot RPC sends.
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
