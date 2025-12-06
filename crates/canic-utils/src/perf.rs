use crate::cdk::api::performance_counter;
use std::cell::RefCell;

thread_local! {
    pub static PERF_LAST: RefCell<u64> = RefCell::new(performance_counter(1));
}
