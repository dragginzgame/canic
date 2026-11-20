use super::*;
use std::cell::Cell;

struct Timers;
impl Source for Timers {
    type Storage = Cell<u64>;
}

struct Endpoints;
impl Source for Endpoints {
    type Storage = Cell<u64>;
}

#[test]
fn sources_remain_distinct_and_reads_do_not_record() {
    assert!(get::<Timers>().is_none());
    let timers = open::<Timers>();
    let endpoints = open::<Endpoints>();
    timers.set(1);
    endpoints.set(2);

    assert!(Rc::ptr_eq(&open::<Timers>(), &timers));
    for _ in 0..2 {
        assert_eq!(get::<Timers>().unwrap().get(), 1);
        assert_eq!(get::<Endpoints>().unwrap().get(), 2);
    }
}
