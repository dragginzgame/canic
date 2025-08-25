pub mod ic;
pub mod icrc;
pub mod sns;

pub mod prelude {
    pub use crate::cdk::candid::{CandidType, Int, Nat, Principal};
    pub use serde::Deserialize;
    pub use serde_bytes::ByteBuf;
}
