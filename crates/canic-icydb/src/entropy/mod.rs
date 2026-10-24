//! Host-owned single-flight secure entropy acquisition and cancellation recovery.

#[cfg(target_arch = "wasm32")]
use std::cell::RefCell;

#[cfg(target_arch = "wasm32")]
use ic_platform::{cdk, ic0, principal::Principal};

// Management's raw_rand is an external Candid protocol. Owned database methods
// remain Wire; Platform owns both codecs and the shared executor/system boundary.
#[cfg(any(target_arch = "wasm32", test))]
fn decode_boot_entropy(reply: &[u8]) -> Option<[u8; 32]> {
    let bytes: Vec<u8> = ic_platform::candid::decode_one(
        reply,
        ic_platform::candid::DecodeLimits {
            reply_bytes: 256,
            decoding_quota: 4_096,
            skipping_quota: 64,
            type_entries: 8,
            header_bytes: 64,
        },
    )
    .ok()?;
    <[u8; 32]>::try_from(bytes)
        .ok()
        .filter(|seed| *seed != [0; 32])
}

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
        cdk::futures::spawn_migratory(async move {
            let _request = request;
            // Keep raw_rand's empty Candid arguments, bounded wait, zero attached
            // cycles and single result. Only reply resource admission is tighter.
            let Ok(arguments) = ic_platform::candid::contract::encode_args(()) else {
                return;
            };
            let seed = cdk::call::Call::bounded_wait(Principal::management_canister(), "raw_rand")
                .with_raw_args(&arguments)
                .await
                .ok()
                .and_then(|response| decode_boot_entropy(&response.into_bytes()));
            let accepted = seed.is_some_and(|seed| accept(seed).is_ok());
            ENTROPY.with_borrow_mut(|state| state.finish(accepted));
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn management_entropy_preserves_candid_bytes_and_seed_admission() {
        use ic_platform::candid::contract::{encode_args, encode_one};

        assert_eq!(encode_args(()).unwrap(), b"DIDL\0\0");
        let seed = [37_u8; 32];
        assert_eq!(
            decode_boot_entropy(&encode_one(seed.to_vec()).unwrap()),
            Some(seed)
        );
        for seed in [vec![0_u8; 32], vec![1_u8; 31], vec![1_u8; 33]] {
            assert_eq!(decode_boot_entropy(&encode_one(seed).unwrap()), None);
        }
    }

    #[test]
    fn management_entropy_refuses_malformed_and_unbounded_replies() {
        use ic_platform::candid::contract::encode_one;

        let valid = encode_one(vec![37_u8; 32]).unwrap();
        for end in 0..valid.len() {
            assert_eq!(decode_boot_entropy(&valid[..end]), None);
        }
        let mut trailing = valid;
        trailing.push(0);
        assert_eq!(decode_boot_entropy(&trailing), None);
        assert_eq!(
            decode_boot_entropy(&encode_one(vec![1_u8; 257]).unwrap()),
            None
        );
        assert_eq!(
            decode_boot_entropy(&encode_one("not entropy").unwrap()),
            None
        );
    }

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
