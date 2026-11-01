//! Deterministic queue access through the existing Canic memory grant.
use super::{
    model::{self, DeliveryRecord, Queue},
    protocol,
    view::{DeliveryEvidence, JobsView},
};
use canic::{
    __internal::core::cdk::structures::{DefaultMemoryImpl, Memory, StableCell},
    Error,
};
use ic_jobs::{Attempt, ExecutionId, JobId};
use std::cell::RefCell;

type Storage = StableCell<Vec<u8>, DefaultMemoryImpl>;
thread_local! { static STORAGE: RefCell<Option<Storage>> = const { RefCell::new(None) }; }

pub struct Dispatch {
    pub delivery: Option<DeliveryRecord>,
    pub next_due_ns: Option<u64>,
}

pub fn open(install: bool) {
    let memory = canic::memory::open(super::KEY);
    assert_eq!(
        memory.size() == 0,
        install,
        "installation/restore disposition"
    );
    // System canister version increases across installs. Retain this initial
    // epoch through upgrades so new installations cannot reuse prior job IDs.
    // https://docs.internetcomputer.org/references/ic-interface-spec/canister-interface/#canister-version
    let initial = if install {
        Queue::empty(ic_platform::cdk::api::canister_version())
            .encode()
            .expect("empty bounded queue")
    } else {
        Vec::new()
    };
    let mut storage = StableCell::init(memory, initial);
    let mut queue = Queue::decode(storage.get()).expect("validated Jobs restore");
    if !install {
        queue
            .restore_running(ic_platform::cdk::api::time())
            .expect("classify restored running intent as uncertain");
    }
    storage.set(queue.encode().expect("bounded restored queue"));
    STORAGE.with_borrow_mut(|slot| *slot = Some(storage));
}
fn read() -> Result<Queue, Error> {
    STORAGE.with_borrow(|slot| Queue::decode(slot.as_ref().ok_or_else(model::invalid)?.get()))
}
fn update<T>(operation: impl FnOnce(&mut Queue) -> Result<T, Error>) -> Result<T, Error> {
    let mut queue = read()?;
    let value = operation(&mut queue)?;
    let bytes = queue.encode()?;
    STORAGE.with_borrow_mut(|slot| slot.as_mut().expect("opened queue").set(bytes));
    Ok(value)
}
pub fn enqueue(delta: u64, delay_secs: u64) -> Result<u128, Error> {
    if delay_secs > 3600 {
        return Err(model::conflict());
    }
    let due = ic_platform::cdk::api::time()
        .checked_add(
            delay_secs
                .checked_mul(1_000_000_000)
                .ok_or_else(model::invalid)?,
        )
        .ok_or_else(model::invalid)?;
    update(|queue| queue.enqueue(delta, due))
}
pub fn begin() -> Result<Dispatch, Error> {
    update(|queue| {
        Ok(Dispatch {
            delivery: queue.begin(ic_platform::cdk::api::time())?,
            next_due_ns: queue.due(),
        })
    })
}
pub fn due() -> Result<Option<u64>, Error> {
    Ok(read()?.due())
}
pub fn view() -> Result<JobsView, Error> {
    Ok(read()?.view())
}
pub const fn evidence(delivery: DeliveryRecord) -> DeliveryEvidence {
    DeliveryEvidence {
        job: delivery.attempt.execution.job.0,
        sequence: delivery.attempt.sequence,
        delta: delivery.delta,
    }
}
pub fn receive(evidence: DeliveryEvidence) -> Result<DeliveryEvidence, Error> {
    let delivery = DeliveryRecord {
        attempt: Attempt {
            execution: ExecutionId {
                job: JobId(evidence.job),
                occurrence: 0,
            },
            sequence: evidence.sequence,
        },
        delta: evidence.delta,
    };
    update(|queue| queue.receive(delivery))?;
    Ok(evidence)
}
pub fn finish(delivery: DeliveryRecord, confirmed: bool) -> Result<(), Error> {
    // Reload after the await: the destination and other messages may have
    // committed receipts or new jobs independently of the old dispatch snapshot.
    update(|queue| queue.finish(delivery, ic_platform::cdk::api::time(), confirmed))
}
pub fn reconcile(job: u128, sequence: u64) -> Result<(), Error> {
    update(|queue| queue.reconcile(job, sequence, ic_platform::cdk::api::time()))
}
pub fn corrupt_for_qualification() {
    STORAGE.with_borrow_mut(|slot| slot.as_mut().expect("opened queue").set(vec![0xff]));
}

/// Submit one exact immutable attempt; the workflow reloads state after awaiting.
pub async fn confirm_delivery(delivery: DeliveryRecord) -> bool {
    let evidence = evidence(delivery);
    let bytes = ic_wire::native::encode_request(protocol::EFFECT, &evidence, protocol::LIMITS)
        .expect("bounded immutable delivery");
    let response = ic_platform::cdk::call::Call::unbounded_wait(
        ic_platform::cdk::api::canister_self(),
        "managed_jobs_effect",
    )
    .take_raw_args(bytes)
    .await;
    response
        .ok()
        .and_then(|response| {
            ic_wire::native::decode_response::<Result<DeliveryEvidence, Error>>(
                protocol::EFFECT,
                &response.into_bytes(),
                protocol::LIMITS,
            )
            .ok()
        })
        .and_then(Result::ok)
        .is_some_and(|receipt| receipt == evidence)
}
