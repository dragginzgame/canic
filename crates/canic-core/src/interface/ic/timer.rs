use crate::cdk::timers::{
    clear_timer as cdk_clear_timer, set_timer as cdk_set_timer,
    set_timer_interval as cdk_set_timer_interval,
};

pub use crate::cdk::timers::TimerId;
use std::{cell::RefCell, future::Future, rc::Rc, time::Duration};

///
/// Timer
///

pub struct Timer;

impl Timer {
    pub fn set(delay: Duration, task: impl Future<Output = ()> + 'static) -> TimerId {
        cdk_set_timer(delay, task)
    }

    pub fn set_interval<F, Fut>(interval: Duration, task: F) -> TimerId
    where
        F: FnMut() -> Fut + 'static,
        Fut: Future<Output = ()> + 'static,
    {
        let task = Rc::new(RefCell::new(task));

        cdk_set_timer_interval(interval, move || {
            let task = Rc::clone(&task);
            async move {
                let fut = { (task.borrow_mut())() };
                fut.await;
            }
        })
    }

    pub fn clear(id: TimerId) {
        cdk_clear_timer(id);
    }
}
