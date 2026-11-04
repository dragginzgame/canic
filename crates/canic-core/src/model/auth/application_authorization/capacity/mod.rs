//! Protected receiver capacity and checked, proportional retention budgets.

/// Default number of simultaneous browser sessions per receiving canister.
pub const DEFAULT_MAX_SESSIONS: u32 = 2_048;

/// Validated capacity profile. Byte limits include retention headroom; they are
/// ceilings rather than preallocated memory or throughput guarantees.
#[cfg(any(
    not(target_arch = "wasm32"),
    test,
    feature = "auth-local-application-authorization"
))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReceiverCapacity {
    pub sessions: usize,
    pub replays: usize,
    pub replays_per_caller: usize,
    pub admitted_logins: usize,
    pub unknown_logouts: usize,
    pub stable_bytes: usize,
    pub index_bytes: usize,
}

#[cfg(any(
    not(target_arch = "wasm32"),
    test,
    feature = "auth-local-application-authorization"
))]
impl ReceiverCapacity {
    /// Derive one consistent profile using Wasm32-safe arithmetic on every host.
    /// The stable-byte ceiling stays below 2 GiB to leave address-space headroom.
    #[must_use]
    pub const fn new(max_sessions: u32) -> Option<Self> {
        if max_sessions == 0 || max_sessions > 2_048 {
            return None;
        }
        let sessions = max_sessions as usize;
        Some(Self {
            sessions,
            replays: sessions * 2,
            replays_per_caller: 256,
            admitted_logins: sessions * 2,
            unknown_logouts: if sessions / 8 > 256 {
                sessions / 8
            } else {
                256
            },
            stable_bytes: if sessions * 4096 > 1_048_576 {
                sessions * 4096
            } else {
                1_048_576
            },
            index_bytes: if sessions * 2048 > 1_048_576 {
                sessions * 2048
            } else {
                1_048_576
            },
        })
    }
}
