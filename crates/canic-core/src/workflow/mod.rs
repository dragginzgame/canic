pub mod app;
pub mod bootstrap;
pub mod cascade;
pub mod command;
pub mod directory;
pub mod ic;
pub mod lifecycle;
pub mod orchestrator;
pub mod placement;
pub mod pool;
pub mod rpc;
pub mod runtime;

///
/// Prelude
///

pub mod prelude {
    pub use crate::{
        cdk::{
            api::{canister_self, msg_caller},
            candid::CandidType,
            types::{Account, Int, Nat, Principal, Subaccount},
        },
        ids::CanisterRole,
        log,
        log::{Level, Topic},
        types::Cycles,
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
    CascadeError(#[from] cascade::CascadeError),

    #[error(transparent)]
    IcError(#[from] ic::IcError),

    #[error(transparent)]
    OrchestrationError(#[from] orchestrator::OrchestratorError),
}
