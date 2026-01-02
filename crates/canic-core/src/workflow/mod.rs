pub mod app;
pub mod canister;
pub mod cascade;
pub mod children;
pub mod command;
pub mod directory;
pub mod env;
pub mod ic;
pub mod icrc;
pub mod log;
pub mod memory;
pub mod metrics;
pub mod orchestrator;
pub mod placement;
pub mod pool;
pub mod registry;
pub mod rpc;
pub mod state;
pub mod view;

pub(crate) mod bootstrap;
pub(crate) mod lifecycle;
pub(crate) mod runtime;

///
/// Prelude
///

pub mod prelude {
    pub use crate::{
        cdk::{
            api::{canister_self, msg_caller},
            candid::CandidType,
            types::{Account, Cycles, Int, Nat, Principal, Subaccount},
        },
        ids::CanisterRole,
        log,
        log::{Level, Topic},
    };
    pub use serde::{Deserialize, Serialize};
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
    Orchestrator(#[from] orchestrator::OrchestratorError),

    #[error(transparent)]
    Placement(#[from] placement::PlacementError),
}
