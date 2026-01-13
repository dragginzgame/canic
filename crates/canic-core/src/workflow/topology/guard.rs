use crate::{InternalError, ThisError, workflow::topology::TopologyWorkflowError};
use std::cell::Cell;

///
/// TopologyGuardError
///

#[derive(Debug, ThisError)]
pub enum TopologyGuardError {
    #[error("topology is currently being mutated")]
    TopologyMutating,
}

impl From<TopologyGuardError> for InternalError {
    fn from(err: TopologyGuardError) -> Self {
        TopologyWorkflowError::from(err).into()
    }
}

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
            Err(TopologyGuardError::TopologyMutating.into())
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
