//! Journal-derived deadlines with independent bounded asynchronous dispatch.
use super::{api, next, observing, snapshot, step, upgrade};
use canic_management_protocol::{Phase, children::ChildId};
use ic_timers::{Lifetime, Observation, Once, OnceDemand, OnceNext, OnceResult, OnceStatus};
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
};

const PARALLEL: usize = 4;
const BUSY_CHECK_NS: u64 = 1_000_000_000;

struct Worker {
    timer: Once,
    child: Option<ChildId>,
}

struct Scheduler {
    wake: Once,
    workers: Vec<Worker>,
    deadlines: BTreeMap<ChildId, u64>,
    due: BTreeSet<(u64, ChildId)>,
    blocked: BTreeSet<ChildId>,
}

thread_local! { static SCHEDULER: RefCell<Option<Scheduler>> = const { RefCell::new(None) }; }

fn demand(slot: ChildId) -> Option<u64> {
    next(&snapshot(slot)).or_else(|| upgrade::next(slot))
}

impl Scheduler {
    fn queue(&mut self, slot: ChildId, deadline: Option<u64>) {
        if let Some(old) = self.deadlines.remove(&slot) {
            self.due.remove(&(old, slot));
        }
        if let Some(at) = deadline
            && !self.blocked.contains(&slot)
        {
            self.deadlines.insert(slot, at);
            self.due.insert((at, slot));
        }
    }

    fn wake_at(&self) -> OnceDemand {
        self.due
            .first()
            .map_or(OnceDemand::Inactive, |&(at, _)| OnceDemand::At(at))
    }

    fn reap(&mut self) {
        for worker in &mut self.workers {
            let Some(slot) = worker.child else { continue };
            match worker.timer.status().expect("child timer status") {
                OnceStatus::RecoveryRequired(token) => {
                    // Fence this child before reusing the executor for another child.
                    self.blocked.insert(slot);
                    if let Some(at) = self.deadlines.remove(&slot) {
                        self.due.remove(&(at, slot));
                    }
                    worker
                        .timer
                        .recover(token, OnceDemand::Inactive)
                        .expect("release fenced timer");
                    worker.child = None;
                }
                OnceStatus::Inactive => worker.child = None,
                OnceStatus::Armed(_) | OnceStatus::Running => {}
                OnceStatus::Faulted => panic!("child timer fault"),
            }
        }
    }

    fn take_due(&mut self, now: u64) -> Option<ChildId> {
        let &(at, slot) = self.due.first()?;
        if at > now {
            return None;
        }
        self.due.pop_first();
        self.deadlines.remove(&slot);
        Some(slot)
    }

    fn dispatch(&mut self) -> OnceResult {
        self.reap();
        let now = api::time();
        for index in 0..self.workers.len() {
            if self.workers[index].child.is_some() {
                continue;
            }
            let Some(slot) = self.take_due(now) else {
                break;
            };
            let worker = &mut self.workers[index];
            worker.child = Some(slot);
            worker
                .timer
                .set(OnceDemand::At(now))
                .expect("dispatch child");
        }
        let next = self.due.first().map_or(OnceNext::Stop, |&(at, _)| {
            // A settled callback wakes us immediately. Check abandoned callbacks
            // while due work is waiting behind all four physical requests.
            OnceNext::At(if at <= now {
                now.saturating_add(BUSY_CHECK_NS)
            } else {
                at
            })
        });
        OnceResult {
            observation: Observation::Success,
            next,
        }
    }
}

async fn run(index: usize) -> OnceResult {
    let slot = SCHEDULER.with_borrow(|s| {
        s.as_ref().unwrap().workers[index]
            .child
            .expect("assigned child")
    });
    let outcome = if let Some(guard) = observing(slot) {
        if snapshot(slot).phase == Phase::Ready {
            upgrade::step(slot).await;
        } else {
            step(slot).await;
        }
        drop(guard);
        if snapshot(slot).failure.is_some() || upgrade::status(slot).failure.is_some() {
            Observation::ExpectedFailure
        } else {
            Observation::Success
        }
    } else {
        Observation::NoWork
    };
    let at = demand(slot).map(|at| {
        if outcome == Observation::NoWork {
            at.max(api::time().saturating_add(BUSY_CHECK_NS))
        } else {
            at
        }
    });
    let successor = SCHEDULER.with_borrow_mut(|s| {
        let s = s.as_mut().unwrap();
        s.queue(slot, at);
        let child = s.take_due(api::time());
        s.workers[index].child = child;
        s.wake.set(s.wake_at()).expect("wake child dispatcher");
        child.map_or(OnceNext::Stop, |_| OnceNext::At(api::time()))
    });
    OnceResult {
        observation: outcome,
        next: successor,
    }
}

pub(super) fn open(slots: Vec<ChildId>) {
    let wake = ic_timers::bind_once(
        ["canic", "children", "dispatch"],
        Lifetime::Retained,
        || async { SCHEDULER.with_borrow_mut(|s| s.as_mut().unwrap().dispatch()) },
    )
    .expect("child dispatcher");
    let workers = (0..PARALLEL)
        .map(|i| Worker {
            child: None,
            timer: ic_timers::bind_once(
                ["canic", "children", &i.to_string()],
                Lifetime::Retained,
                move || run(i),
            )
            .expect("child execution timer"),
        })
        .collect();
    let mut scheduler = Scheduler {
        wake,
        workers,
        deadlines: BTreeMap::new(),
        due: BTreeSet::new(),
        blocked: BTreeSet::new(),
    };
    for slot in slots {
        scheduler.queue(slot, demand(slot));
    }
    scheduler
        .wake
        .set(scheduler.wake_at())
        .expect("open child dispatcher");
    SCHEDULER.set(Some(scheduler));
}

pub(super) fn schedule(slot: ChildId) {
    refresh(slot, false);
}
pub(super) fn resume_after_reconciliation(slot: ChildId) {
    refresh(slot, true);
}

fn refresh(slot: ChildId, reconciled: bool) {
    let at = demand(slot);
    SCHEDULER.with_borrow_mut(|s| {
        let s = s.as_mut().expect("open child dispatcher");
        s.reap();
        if reconciled {
            s.blocked.remove(&slot);
        }
        if s.workers.iter().any(|worker| worker.child == Some(slot)) {
            return;
        }
        s.queue(slot, at);
        s.wake.set(s.wake_at()).expect("schedule child dispatcher");
    });
}
