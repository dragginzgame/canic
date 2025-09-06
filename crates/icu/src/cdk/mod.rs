///
/// IMPORT IC CRATES
///
pub use ic_cdk::*;

pub mod api {
    pub use ic_cdk::api::*;
}

pub mod candid {
    pub use ::candid::*;
}

pub mod mgmt {
    pub use ::ic_cdk::management_canister::*;
}

pub mod timers {
    pub use ::ic_cdk_timers::*;
}

pub mod structures;
