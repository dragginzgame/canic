//! The hub retains an exact ordinal or batch before awaiting its result.
use super::{LIMITS, MAX_WORKERS, Pending, Pool, Worker};
use canic::{
    method::{CallError, Contract, call},
    sharding::{LookupPlacement, Placement, ShardTarget},
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
            let request = (canic::worker_pool!("miner").to_owned(), index);
            let acquire = async || {
                use canic::sharding::payment::{Demand, Payment, acquire_placement};
                let maximum = 5_000_000_000_000;
                match acquire_placement(
                    state.root,
                    Demand::Worker(request.0.clone(), index),
                    maximum,
                    maximum,
                )
                .await?
                {
                    Payment::Placed(placement, _) => Ok(placement),
                    Payment::Quoted(_) => Ok(Placement::Busy),
                }
            };
            let placement = if retry {
                match call::<LookupPlacement>(
                    state.root,
                    10,
                    &canic::sharding::Lookup::Worker(request.0.clone(), request.1),
                )
                .await
                {
                    Ok(Placement::Unknown) if generation.is_none() => {
                        // Another continuation may have admitted or superseded this claim.
                        if Pool::read().pending != Some(pending.clone()) {
                            return "advanced".into();
                        }
                        acquire().await
                    }
                    reply => reply.map_err(canic::sharding::payment::AcquireError::Call),
                }
            } else {
                acquire().await
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
                    assert_eq!(
                        latest.workers.len() + latest.released.len(),
                        usize::from(index)
                    );
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
                        attempts: 0,
                        blocks: 0,
                        last_batch: None,
                    });
                    // Commit the batch that requested growth before accepting a newer goal.
                    latest.pending = None;
                    latest.begin(latest.round, latest.goal);
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
                Err(canic::sharding::payment::AcquireError::Call(error)) => {
                    return format!("worker {index}: Root {}", call_failure(&error));
                }
                Err(canic::sharding::payment::AcquireError::Refused(
                    canic::sharding::payment::Refusal::UnknownGrant,
                )) => return format!("worker {index}: Root has no approved worker grant"),
                Err(error) => return format!("worker {index}: Root {error:?}"),
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
            let reply = call::<miner::mine>(
                target.shard,
                10,
                &(
                    ic_wire::Bytes::new(target.allocation.generation.to_vec())
                        .expect("allocation generation"),
                    round,
                    ordinal,
                ),
            )
            .await;
            let mut latest = Pool::read();
            if latest.pending.as_ref() != Some(&pending) {
                return "advanced".into();
            }
            let found = match reply {
                Ok((value, found)) if value == expected => found,
                Ok((value, _)) => {
                    return format!(
                        "worker {index}, batch {round}/{ordinal}: expected attempt {expected}, received {value}"
                    );
                }
                Err(error) => {
                    return format!(
                        "worker {index}, batch {round}/{ordinal}: {}",
                        call_failure(&error)
                    );
                }
            };
            let worker = &mut latest.workers[usize::from(index)];
            assert_eq!(&worker.target, target, "worker allocation changed");
            worker.attempts = expected;
            worker.blocks += u64::from(found);
            worker.last_batch = Some((round, ordinal));
            if latest.round == round {
                latest.completed += 1;
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
            allocation: worker.target.allocation.clone(),
            principal: worker.target.shard,
            status: status.into(),
            attempts: worker.attempts,
            blocks: worker.blocks,
            last_batch: worker.last_batch,
        }
    }).collect();
    Report {
        round: state.round,
        goal: state.goal,
        completed: state.completed,
        workers,
    }
}
#[derive(serde::Deserialize, serde::Serialize)]
/// Public snapshot of one pool round.
pub struct Report {
    /// Operator-selected round number.
    pub round: u64,
    /// Requested attempts for this round.
    pub goal: u8,
    /// Acknowledged attempts in the current round.
    pub completed: u8,
    /// All allocated workers in ordinal order.
    pub workers: Vec<WorkerReport>,
}
#[derive(serde::Deserialize, serde::Serialize)]
/// A single worker’s retained identity and production.
pub struct WorkerReport {
    /// Exact installation to name when relinquishing this worker.
    pub allocation: canic::sharding::Allocation,
    /// Canister to which the hub dispatches this ordinal.
    pub principal: ic_platform_types::Principal,
    /// Current dispatch state: active, draining or parked.
    pub status: String,
    /// Total acknowledged attempts over all rounds.
    pub attempts: u64,
    /// Total acknowledged finds over all rounds.
    pub blocks: u64,
    /// Last acknowledged round and ordinal.
    pub last_batch: Option<(u64, u8)>,
}

/// Relinquish a parked trailing worker under its exact original assignment.
pub struct Release;
impl Contract for Release {
    const NAME: &'static str = "release_worker";
    const ID: u64 = ic_wire::method_id(Self::NAME);
    const REQUEST: ic_wire::Limits = canic::sharding::LIMITS;
    type Input = ShardTarget;
    type Output = Result<(), String>;
}

// release_worker
// Remove this assignment from dispatch and retain its exact release receipt.
// guard  : pool operator
// limits : default
#[canic::update(contract = Release, guard = operator)]
fn release_worker(target: ShardTarget) -> Result<(), String> {
    let mut state = Pool::read();
    if state.released.contains(&target) {
        return Ok(());
    }
    let active = usize::from(state.goal.div_ceil(2));
    if state.pending.is_some() || state.workers.len() <= active {
        return Err("worker must be parked and dispatch settled".into());
    }
    if state.workers.last().is_none_or(|w| w.target != target) {
        return Err("exact trailing worker allocation required".into());
    }
    state.workers.pop();
    state.released.push(target);
    state.save();
    Ok(())
}

// worker_released
// Let Root check the hub's durable release receipt for this exact installation.
// guard  : retained Root
// limits : default
#[canic::query(contract = canic::sharding::standby::WorkerReleased, guard = routing_root)]
fn worker_released(target: ShardTarget) -> bool {
    Pool::read().released.contains(&target)
}

fn routing_root() -> Result<(), &'static str> {
    (system::caller() == Pool::read().root)
        .then_some(())
        .ok_or("routing Root required")
}
