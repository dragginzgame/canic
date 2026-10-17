//! Host-owned single-flight secure entropy acquisition and cancellation recovery.

#[cfg(target_arch = "wasm32")]
use std::cell::RefCell;

#[cfg(any(target_arch = "wasm32", test))]
#[derive(Default)]
enum BootEntropy {
    #[default]
    Empty,
    Pending,
    Ready,
}

#[cfg(any(target_arch = "wasm32", test))]
impl BootEntropy {
    const fn begin(&mut self) -> bool {
        if matches!(self, Self::Empty) {
            *self = Self::Pending;
            true
        } else {
            false
        }
    }

    const fn finish(&mut self, accepted: bool) {
        if matches!(self, Self::Pending) {
            *self = if accepted { Self::Ready } else { Self::Empty };
        }
    }
}

#[cfg(target_arch = "wasm32")]
thread_local! {
    static ENTROPY: RefCell<BootEntropy> = const { RefCell::new(BootEntropy::Empty) };
}

#[cfg(target_arch = "wasm32")]
struct EntropyRequest;

#[cfg(target_arch = "wasm32")]
impl Drop for EntropyRequest {
    fn drop(&mut self) {
        ENTROPY.with_borrow_mut(|state| state.finish(false));
    }
}

/// Begin secure boot entropy acquisition from a replicated host callback.
///
/// Coalesces pending calls and retries after refusal, bad replies or cancellation.
/// The database accepts the seed before the host records completion. Native
/// database execution uses its operating-system source and needs no IC request.
///
/// # Panics
/// Traps if called from a Wasm context that forbids asynchronous IC calls.
pub fn request_boot_entropy<E: 'static>(accept: fn([u8; 32]) -> Result<(), E>) {
    #[cfg(not(target_arch = "wasm32"))]
    let _ = accept;
    #[cfg(target_arch = "wasm32")]
    {
        if ic0::in_replicated_execution() == 0 || !ENTROPY.with_borrow_mut(BootEntropy::begin) {
            return;
        }
        let request = EntropyRequest;
        ic_cdk::futures::spawn_migratory(async move {
            let _request = request;
            let seed = ic_cdk_management_canister::raw_rand()
                .await
                .ok()
                .and_then(|bytes| <[u8; 32]>::try_from(bytes).ok())
                .filter(|seed| *seed != [0; 32]);
            let accepted = seed.is_some_and(|seed| accept(seed).is_ok());
            ENTROPY.with_borrow_mut(|state| state.finish(accepted));
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failure_and_cancellation_allow_retry_without_duplicate_pending_requests() {
        let mut state = BootEntropy::Empty;
        assert!(state.begin());
        assert!(!state.begin());
        state.finish(false);
        assert!(state.begin());
        state.finish(true);
        assert!(!state.begin());
        state.finish(false);
        assert!(!state.begin());
    }
}
