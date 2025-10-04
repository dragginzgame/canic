pub mod ic;
pub mod icrc;
pub mod nns;
pub mod sns;

pub mod prelude {
    pub use crate::{
        cdk::candid::{CandidType, Principal},
        types::{Account, CanisterType, Cycles, Int, Nat, Subaccount},
    };
    pub use serde::Deserialize;
    pub use serde_bytes::ByteBuf;
}
