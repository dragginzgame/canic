pub mod app;
pub mod bootstrap;
pub mod canister;
pub mod cascade;
pub mod config;
pub mod env;
pub mod ic;
pub mod icrc;
pub mod lifecycle;
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
            api::{canister_self, msg_caller},
            candid::CandidType,
            types::{Cycles, Principal},
        },
        ids::CanisterRole,
        log,
        log::Topic,
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
    Cascade(#[from] cascade::CascadeError),

    #[error(transparent)]
    Ic(#[from] ic::IcWorkflowError),

    #[error(transparent)]
    Placement(#[from] placement::PlacementError),
}
