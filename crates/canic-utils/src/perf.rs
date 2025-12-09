use crate::cdk::api::performance_counter;
use std::cell::RefCell;

thread_local! {
    pub static PERF_LAST: RefCell<u64> = RefCell::new(performance_counter(1));
}

// wrapper around performance_counter just in case
#[must_use]
#[allow(clippy::missing_const_for_fn)]
pub fn perf_counter() -> u64 {
    #[cfg(target_arch = "wasm32")]
    {
        performance_counter(1)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        0
    }
}
