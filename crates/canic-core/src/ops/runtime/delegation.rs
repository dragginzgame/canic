use std::cell::Cell;

///
/// DelegationRotationState
///

#[derive(Clone, Copy, Debug, Default)]
pub struct DelegationRotationState {
    pub active: bool,
    pub interval_secs: Option<u64>,
    pub last_rotation_at: Option<u64>,
}

thread_local! {
    static ROTATION_STATE: Cell<DelegationRotationState> =
         Cell::new(DelegationRotationState::default());
}

///
/// DelegationRuntimeOps
///

pub struct DelegationRuntimeOps;

impl DelegationRuntimeOps {
    #[must_use]
    pub fn rotation_state() -> DelegationRotationState {
        ROTATION_STATE.with(Cell::get)
    }

    pub fn start_rotation(interval_secs: u64) {
        ROTATION_STATE.with(|state| {
            let last = state.get().last_rotation_at;
            state.set(DelegationRotationState {
                active: true,
                interval_secs: Some(interval_secs),
                last_rotation_at: last,
            });
        });
    }

    pub fn stop_rotation() {
        ROTATION_STATE.with(|state| {
            let last = state.get().last_rotation_at;
            state.set(DelegationRotationState {
                active: false,
                interval_secs: None,
                last_rotation_at: last,
            });
        });
    }

    pub fn record_rotation(now_secs: u64) {
        ROTATION_STATE.with(|state| {
            let mut snapshot = state.get();
            snapshot.last_rotation_at = Some(now_secs);
            state.set(snapshot);
        });
    }
}
