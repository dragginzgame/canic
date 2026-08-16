//! Module: workflow::topology::guard
//!
//! Responsibility: guard topology mutation sections against concurrent re-entry.
//! Does not own: topology storage mutation, endpoint authorization, or DTO schemas.
//! Boundary: workflow-local mutation guard used by topology orchestration.

use crate::InternalError;
use std::cell::Cell;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TopologyState {
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
            Err(InternalError::invariant())
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
