use std::cell::Cell;

thread_local! {
    // Readiness is an internal synchronization barrier.
    // It must only be set once by bootstrap completion.
    // It must never be reset or inferred from public state.
    static READY: Cell<bool> = const { Cell::new(false) };
}

// Internal readiness barrier for bootstrap synchronization.
//
// Semantics:
// - Starts as false on each fresh runtime (init or post-upgrade).
// - Transitions to true exactly once after successful bootstrap.
// - Never transitions back to false within the same runtime.
///
/// ReadyOps
///

pub struct ReadyOps;

impl ReadyOps {
    #[must_use]
    pub fn is_ready() -> bool {
        READY.with(Cell::get)
    }

    pub(crate) fn mark_ready(_token: crate::workflow::bootstrap::ReadyToken) {
        READY.with(|ready| {
            if !ready.get() {
                ready.set(true);
                crate::log!(crate::log::Topic::Init, Info, "canister marked READY");
            }
        });
    }
}
