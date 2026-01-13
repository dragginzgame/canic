use crate::{InternalError, InternalErrorOrigin};
use std::cell::Cell;

///
/// TopologyState
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TopologyState {
    Stable,
    Mutating,
}

thread_local! {
    static TOPOLOGY_STATE: Cell<TopologyState> = const {
        Cell::new(TopologyState::Stable)
    };
}

///
/// TopologyGuard
///

pub struct TopologyGuard;

impl TopologyGuard {
    pub fn try_enter() -> Result<Self, InternalError> {
        let entered = TOPOLOGY_STATE.with(|state| {
            if state.get() == TopologyState::Mutating {
                false
            } else {
                state.set(TopologyState::Mutating);
                true
            }
        });

        if entered {
            Ok(Self)
        } else {
            Err(InternalError::invariant(
                InternalErrorOrigin::Workflow,
                "topology is currently being mutated",
            ))
        }
    }
}

impl Drop for TopologyGuard {
    fn drop(&mut self) {
        TOPOLOGY_STATE.with(|state| {
            debug_assert_eq!(state.get(), TopologyState::Mutating);
            state.set(TopologyState::Stable);
        });
    }
}
