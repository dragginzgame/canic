pub mod orchestrator;

use crate::workflow::prelude::*;

///
/// LifecycleEvent
///

pub enum LifecycleEvent {
    Create {
        role: CanisterRole,
        parent: Principal,
        extra_arg: Option<Vec<u8>>,
    },
    Upgrade {
        pid: Principal,
    },
}

///
/// LifecycleResult
///

#[derive(Default)]
pub struct LifecycleResult {
    pub new_canister_pid: Option<Principal>,
}

impl LifecycleResult {
    #[must_use]
    pub const fn created(pid: Principal) -> Self {
        Self {
            new_canister_pid: Some(pid),
        }
    }
}
