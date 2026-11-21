//! The hub retains an exact ordinal or batch before awaiting its result.
use super::{LIMITS, MAX_WORKERS, Pending, Pool, Worker};
use canic::{
    method::{CallError, Contract, call},
    sharding::{AssignWorker, LookupWorker, Placement, ShardTarget},
};
use ic_platform_runtime::system;

canic::mount_lifecycle! {
    input = ic_platform_types::Principal, max_bytes = 29, decode = crate::decode_operator,
    install = crate::install, reopen = crate::reopen,
}
canic::mount_readiness_endpoint!(readiness);
fn readiness() -> canic::readiness::State {
    if crate::CELL.with_borrow(Option::is_some) {
        canic::readiness::State::Ready
    } else {
        canic::readiness::State::Unavailable
    }
}
fn operator() -> Result<(), &'static str> {
    (system::caller() == Pool::read().operator)
        .then_some(())
        .ok_or("pool operator required")
}

fn call_failure(error: &CallError) -> &'static str {
    match error {
        CallError::Request(_) => "request encoding failed",
        CallError::Dispatch(_) => "request could not be submitted",
        CallError::Rejected(_) => "call rejected",
        CallError::Unknown(_) => "outcome unknown",
        CallError::Reply(_) => "reply could not be decoded",
    }
}

mod miner {
    use canic::method_descriptor as update;
    include!("../../miner/src/mine.rs");
}

/// Advance one step of the operator-selected round and goal.
pub struct Tick;
impl Contract for Tick {
    const NAME: &'static str = "tick";
    const ID: u64 = ic_wire::method_id(Self::NAME);
    type Input = (u64, u8);
    type Output = String;
}

// tick
// Advance the selected round by one bounded allocation or batch operation.
// guard  : retained operator
// limits : default
#[canic::update(contract = Tick, guard = operator)]
async fn tick(input: (u64, u8)) -> String {
    let (round, goal) = input;
    let mut state = Pool::read();
    let retry = state.pending.is_some();
    state.begin(round, goal);
    state.save();
    let Some(pending) = state.pending.clone() else {
        return if goal > MAX_WORKERS * 2 {
            "capacity limit"
        } else {
            "settled"
        }
        .into();
    };
    match pending.clone() {
        Pending::Grow(index, generation) => {
            let placement = if retry {
                match call::<LookupWorker>(state.root, 10, &index).await {
                    Ok(Placement::Unknown) if generation.is_none() => {
                        // Another continuation may have admitted or superseded this claim.
                        if Pool::read().pending != Some(pending.clone()) {
                            return "advanced".into();
                        }
                        call::<AssignWorker>(state.root, 10, &index).await
                    }
                    reply => reply,
                }
            } else {
                call::<AssignWorker>(state.root, 10, &index).await
            };
            let mut latest = Pool::read();
            if latest.pending.as_ref() != Some(&pending) {
                return "advanced".into();
            }
            match placement {
                Ok(Placement::Pending(allocation)) => {
                    assert!(
                        generation.as_ref().is_none_or(|old| old == &allocation),
                        "allocation changed"
                    );
                    latest.pending = Some(Pending::Grow(index, Some(allocation)));
                    latest.save();
                    return format!("worker {index}: Root allocation pending");
                }
                Ok(ref assigned @ Placement::Assigned(..)) => {
                    let (allocation, shard) = assigned
                        .assigned_target()
                        .expect("worker identity")
                        .unwrap();
                    assert!(
                        generation.as_ref().is_none_or(|old| old == allocation),
                        "allocation changed"
                    );
                    assert_eq!(latest.workers.len(), usize::from(index));
                    assert!(
                        !latest
                            .workers
                            .iter()
                            .any(|w| w.target.shard == shard || w.target.allocation == *allocation)
                    );
                    latest.workers.push(Worker {
                        target: ShardTarget {
                            allocation: allocation.clone(),
                            shard,
                        },
                        blocks: 0,
                        last_batch: None,
                    });
                    latest.pending = None;
                }
                Ok(ref refusal @ (Placement::Full | Placement::GrowthRefused))
                    if generation.is_none() =>
                {
                    latest.pending = None;
                    latest.save();
                    return if *refusal == Placement::Full {
                        "Root capacity limit"
                    } else {
                        "growth refused"
                    }
                    .into();
                }
                Err(error) => return format!("worker {index}: Root {}", call_failure(&error)),
                Ok(Placement::Unavailable(..)) => {
                    return format!("worker {index}: original allocation unavailable");
                }
                Ok(Placement::Busy) => return format!("worker {index}: Root busy"),
                Ok(_) => return format!("worker {index}: original allocation unconfirmed"),
            }
            latest.save();
        }
        Pending::Mine {
            index,
            round,
            ordinal,
            expected,
        } => {
            let target = &state.workers[usize::from(index)].target;
            let reply = call::<miner::mine>(target.shard, 10, &(round, ordinal)).await;
            let mut latest = Pool::read();
            if latest.pending.as_ref() != Some(&pending) {
                return "advanced".into();
            }
            match reply {
                Ok(value) if value == expected => {}
                Ok(value) => {
                    return format!(
                        "worker {index}, batch {round}/{ordinal}: expected block {expected}, received {value}"
                    );
                }
                Err(error) => {
                    return format!(
                        "worker {index}, batch {round}/{ordinal}: {}",
                        call_failure(&error)
                    );
                }
            }
            let worker = &mut latest.workers[usize::from(index)];
            assert_eq!(&worker.target, target, "worker allocation changed");
            worker.blocks = expected;
            worker.last_batch = Some((round, ordinal));
            if latest.round == round {
                latest.produced += 1;
            }
            latest.pending = None;
            latest.save();
        }
    }
    "advanced".into()
}

/// Read the four-worker report under the same reply budget on both ends.
pub struct PoolReport;
impl Contract for PoolReport {
    const NAME: &'static str = "report";
    const ID: u64 = ic_wire::method_id(Self::NAME);
    const REPLY: ic_wire::Limits = LIMITS;
    type Input = ();
    type Output = Report;
}

// report
// Show retained identities, production and dispatch state for the current goal.
// limits : four workers
#[canic::query(contract = PoolReport)]
fn report() -> Report {
    let state = Pool::read();
    let active = usize::from(state.goal.div_ceil(2).min(MAX_WORKERS));
    let workers = state.workers.iter().enumerate().map(|(index, worker)| {
        let draining = matches!(state.pending, Some(Pending::Mine { index: pending, .. }) if usize::from(pending) == index);
        let status = if index < active {
            "active"
        } else if draining {
            "draining"
        } else {
            "parked"
        };
        WorkerReport {
            principal: worker.target.shard,
            status: status.into(),
            blocks: worker.blocks,
            last_batch: worker.last_batch,
        }
    }).collect();
    Report {
        round: state.round,
        goal: state.goal,
        produced: state.produced,
        workers,
    }
}
#[derive(serde::Deserialize, serde::Serialize)]
/// Public snapshot of one pool round.
pub struct Report {
    /// Operator-selected round number.
    pub round: u64,
    /// Requested toy blocks for this round.
    pub goal: u8,
    /// Acknowledged blocks in the current round.
    pub produced: u8,
    /// All allocated workers in ordinal order.
    pub workers: Vec<WorkerReport>,
}
#[derive(serde::Deserialize, serde::Serialize)]
/// A single worker’s retained identity and production.
pub struct WorkerReport {
    /// Canister to which the hub dispatches this ordinal.
    pub principal: ic_platform_types::Principal,
    /// Current dispatch state: active, draining or parked.
    pub status: String,
    /// Total acknowledged blocks over all rounds.
    pub blocks: u64,
    /// Last acknowledged round and ordinal.
    pub last_batch: Option<(u64, u8)>,
}
