use crate::ops::runtime::ready::ReadyOps;

// Internal readiness barrier for bootstrap synchronization.
// Not a public diagnostic or state view.
///
/// ReadyApi
///

pub struct ReadyApi;

impl ReadyApi {
    #[must_use]
    pub fn is_ready() -> bool {
        ReadyOps::is_ready()
    }
}
