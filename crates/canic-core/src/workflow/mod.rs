pub mod bootstrap;
pub mod canister;
pub mod canister_lifecycle;
pub mod cascade;
pub mod config;
pub mod env;
pub mod http;
pub mod ic;
pub mod icrc;
pub mod log;
pub mod memory;
pub mod metrics;
pub mod placement;
pub mod pool;
pub mod rpc;
pub mod runtime;
pub mod state;
pub mod topology;
pub mod view;

///
/// Prelude
///

pub mod prelude {
    pub use crate::{
        cdk::{
            candid::CandidType,
            types::{Account, Principal},
        },
        ids::CanisterRole,
        log,
        log::Topic,
        ops::ic::{Cycles, canister_self, msg_caller},
    };
}

use crate::ThisError;

///
/// WorkflowError
///

#[derive(Debug, ThisError)]
pub enum WorkflowError {
    #[error(transparent)]
    Bootstrap(#[from] bootstrap::BootstrapError),

    #[error(transparent)]
    Cascade(#[from] cascade::CascadeWorkflowError),

    #[error(transparent)]
    Ic(#[from] ic::IcWorkflowError),

    #[error(transparent)]
    Placement(#[from] placement::PlacementWorkflowError),

    #[error(transparent)]
    Rpc(#[from] rpc::RpcWorkflowError),

    #[error(transparent)]
    Topology(#[from] topology::TopologyWorkflowError),
}
