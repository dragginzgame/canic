pub mod app;
pub(crate) mod bootstrap;
pub mod cascade;
pub mod command;
pub mod directory;
pub mod env;
pub mod ic;
pub(crate) mod lifecycle;
pub mod orchestrator;
pub mod placement;
pub mod pool;
pub mod query;
pub mod registry;
pub mod rpc;
pub(crate) mod runtime;
pub mod snapshot;
pub mod state;

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
