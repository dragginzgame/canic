//! Explicit operator ticks drive a bounded pool. Root retains the worker allowance.
mod endpoints;
pub use endpoints::{PoolReport, Release, Report, Tick, WorkerReport};

use canic::sharding::{Allocation, ShardTarget};
use ic_platform_runtime::system;
use ic_platform_types::Principal;
use ic_storage::{DefaultMemoryImpl, adapter::StableCell};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;

const KEY: &str = "pool.round.v3";
const LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 4096,
    nodes: 1024,
    ..canic::sharding::LIMITS
};
const MAX_WORKERS: u8 = 4;
thread_local! {
    static CELL: RefCell<Option<StableCell<Vec<u8>>>> = const { RefCell::new(None) };
}

#[derive(Deserialize, Serialize)]
struct Worker {
    target: ShardTarget,
    attempts: u64,
    blocks: u64,
    last_batch: Option<(u64, u8)>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
enum Pending {
    Grow(u8, Option<Allocation>),
    Mine {
        index: u8,
        round: u64,
        ordinal: u8,
        expected: u64,
    },
}

#[derive(Deserialize, Serialize)]
struct Pool {
    root: Principal,
    operator: Principal,
    round: u64,
    goal: u8,
    completed: u8,
    workers: Vec<Worker>,
    released: Vec<ShardTarget>,
    pending: Option<Pending>,
}

impl Pool {
    fn encode(&self) -> Vec<u8> {
        ic_wire::encode(self, ic_wire::Encoding::Payload, LIMITS).expect("bounded pool record")
    }
    fn read() -> Self {
        CELL.with_borrow(|c| {
            ic_wire::decode(
                c.as_ref().expect("pool open").get(),
                ic_wire::Encoding::Payload,
                LIMITS,
            )
            .expect("retained pool record")
        })
    }
    fn save(&self) {
        CELL.with_borrow_mut(|c| {
            c.as_mut().expect("pool open").set(self.encode());
        });
    }
    fn begin(&mut self, round: u64, goal: u8) {
        assert!(round > 0 && round >= self.round, "stale round");
        if round > self.round && matches!(self.pending, Some(Pending::Grow(..))) {
            // The retained round identifies the first batch owed to the new worker.
            return;
        }
        if round > self.round {
            self.round = round;
            self.goal = goal;
            self.completed = 0;
        } else {
            assert_eq!(self.goal, goal, "round already bound to another goal");
        }
        if self.pending.is_some() || self.completed >= goal.min(MAX_WORKERS * 2) {
            return;
        }
        let index = self.completed / 2;
        self.pending = Some(match self.workers.get(usize::from(index)) {
            None => Pending::Grow(
                index + u8::try_from(self.released.len()).expect("bounded releases"),
                None,
            ),
            Some(worker) => Pending::Mine {
                index,
                round,
                ordinal: self.completed % 2 + 1,
                expected: worker
                    .attempts
                    .checked_add(1)
                    .expect("worker attempt count exhausted"),
            },
        });
    }
}

fn decode_operator(bytes: &[u8]) -> Result<Principal, &'static str> {
    let p = Principal::try_from(bytes).map_err(|_| "invalid pool operator")?;
    if p == Principal::anonymous() || bytes.is_empty() {
        return Err("named pool operator required");
    }
    Ok(p)
}

fn install(operator: Principal) {
    let root = system::caller();
    assert_eq!(
        root.as_slice().last(),
        Some(&1),
        "Root must install the pool"
    );
    open(Some(Pool {
        root,
        operator,
        round: 0,
        goal: 0,
        completed: 0,
        workers: Vec::new(),
        pending: None,
        released: Vec::new(),
    }));
}
fn reopen() {
    open(None);
}
fn open(initial: Option<Pool>) {
    let runtime = canic::storage::bootstrap(
        DefaultMemoryImpl::default(),
        initial.is_some(),
        "pool",
        &[KEY],
        16,
    );
    let cell = canic::storage::cell(&runtime, KEY, LIMITS.bytes, initial.map(|s| s.encode()));
    CELL.with_borrow_mut(|c| *c = Some(cell));
    let state = Pool::read();
    assert!(
        state.workers.len() + state.released.len() <= usize::from(MAX_WORKERS)
            && state.completed <= state.goal.min(MAX_WORKERS * 2)
    );
}
