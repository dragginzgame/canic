pub mod abi;
pub mod canister;
pub mod directory;
pub mod log;
pub mod metrics;
pub mod page;
pub mod payment;
pub mod placement;
pub mod pool;
pub mod registry;
pub mod rpc;
pub mod snapshot;
pub mod state;
pub mod subnet;

///
/// PRELUDE
///

pub mod prelude {
    pub use crate::ids::{CanisterRole, SubnetRole};
    pub use candid::{CandidType, Principal};
    pub use derive_more::Display;
    pub use serde::{Deserialize, Serialize};
    pub use std::collections::HashMap;
}
