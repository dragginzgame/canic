//! One bounded application queue. Jobs alone owns attempt/state transitions.
use super::view::{JobView, JobsView, StateView};
use canic::Error;
use ic_jobs::{
    Attempt, ExecutionId, Job, JobId, JobRecord, JobState, Outcome, RetryPolicy, Schedule,
};
use serde::{Deserialize, Serialize};

const MAX_JOBS: usize = 8;
const FORMAT: &[u8] = b"CJQ\x01";
const LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 16_380,
    // Wire conservatively charges repeated record visitors before allocation.
    allocation: 262_144,
    nodes: 2048,
    depth: 32,
};

pub const fn invalid() -> Error {
    Error::from_registered(canic::diagnostics::codes::STATE_INVALID)
}
pub const fn conflict() -> Error {
    Error::from_registered(canic::diagnostics::codes::STATE_CONFLICT)
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DeliveryRecord {
    pub attempt: Attempt,
    pub delta: u64,
}
#[derive(Deserialize, Serialize)]
struct EntryRecord {
    job: JobRecord,
    delta: u64,
}
#[derive(Deserialize, Serialize)]
struct QueueRecord {
    history_epoch: u64,
    next_id: u64,
    entries: Vec<EntryRecord>,
    receipts: Vec<DeliveryRecord>,
    total: u64,
}
struct Entry {
    job: Job,
    delta: u64,
}
pub struct Queue {
    history_epoch: u64,
    next_id: u64,
    entries: Vec<Entry>,
    receipts: Vec<DeliveryRecord>,
    total: u64,
}
impl Queue {
    pub const fn empty(history_epoch: u64) -> Self {
        Self {
            history_epoch,
            next_id: 1,
            entries: Vec::new(),
            receipts: Vec::new(),
            total: 0,
        }
    }
    pub fn decode(bytes: &[u8]) -> Result<Self, Error> {
        let payload = bytes.strip_prefix(FORMAT).ok_or_else(invalid)?;
        let record: QueueRecord =
            ic_wire::native::decode(payload, LIMITS).map_err(|_| invalid())?;
        if record.next_id == 0
            || record.entries.len() > MAX_JOBS
            || record.receipts.len() > MAX_JOBS
        {
            return Err(invalid());
        }
        let mut previous = 0;
        let entries = record
            .entries
            .into_iter()
            .map(|entry| {
                let job = Job::restore(entry.job).map_err(|_| invalid())?;
                let id = job.record().id.0;
                let local_id = id & u128::from(u64::MAX);
                let in_history = id >> 64 == u128::from(record.history_epoch)
                    && local_id > 0
                    && local_id < u128::from(record.next_id);
                let one_shot_policy = matches!(job.record().schedule, Schedule::Once { .. })
                    && job.record().retry == RetryPolicy::NONE;
                let immutable_payload = (1..=1000).contains(&entry.delta);
                if id <= previous || !in_history || !one_shot_policy || !immutable_payload {
                    return Err(invalid());
                }
                previous = id;
                Ok(Entry {
                    job,
                    delta: entry.delta,
                })
            })
            .collect::<Result<Vec<_>, Error>>()?;
        let queue = Self {
            history_epoch: record.history_epoch,
            next_id: record.next_id,
            entries,
            receipts: record.receipts,
            total: record.total,
        };
        queue.validate_receipts()?;
        Ok(queue)
    }
    fn validate_receipts(&self) -> Result<(), Error> {
        let mut sum = 0u64;
        for (index, receipt) in self.receipts.iter().enumerate() {
            let entry = self
                .entries
                .iter()
                .find(|entry| entry.job.record().id == receipt.attempt.execution.job)
                .ok_or_else(invalid)?;
            let record = entry.job.record();
            let expected = DeliveryRecord {
                attempt: Attempt {
                    execution: ExecutionId {
                        job: record.id,
                        occurrence: 0,
                    },
                    sequence: record.sequence,
                },
                delta: entry.delta,
            };
            let retained_attempt = record.sequence > 0 && *receipt == expected;
            let permits_receipt = matches!(
                record.state,
                JobState::Running | JobState::Uncertain { .. } | JobState::Completed
            );
            let duplicate_execution = self.receipts[..index]
                .iter()
                .any(|other| other.attempt.execution == receipt.attempt.execution);
            if !retained_attempt || !permits_receipt || duplicate_execution {
                return Err(invalid());
            }
            sum = sum.checked_add(receipt.delta).ok_or_else(invalid)?;
        }
        if sum != self.total
            || self.entries.iter().any(|entry| {
                matches!(entry.job.record().state, JobState::Completed)
                    && !self
                        .receipts
                        .iter()
                        .any(|receipt| receipt.attempt.execution.job == entry.job.record().id)
            })
        {
            return Err(invalid());
        }
        Ok(())
    }
    pub fn encode(&self) -> Result<Vec<u8>, Error> {
        let record = QueueRecord {
            history_epoch: self.history_epoch,
            next_id: self.next_id,
            entries: self
                .entries
                .iter()
                .map(|entry| EntryRecord {
                    job: entry.job.record(),
                    delta: entry.delta,
                })
                .collect(),
            receipts: self.receipts.clone(),
            total: self.total,
        };
        let mut bytes = FORMAT.to_vec();
        bytes.extend(ic_wire::native::encode(&record, LIMITS).map_err(|_| invalid())?);
        Ok(bytes)
    }
    pub fn enqueue(&mut self, delta: u64, at_ns: u64) -> Result<u128, Error> {
        if self.entries.len() == MAX_JOBS || !(1..=1000).contains(&delta) {
            return Err(conflict());
        }
        let id = self.next_id;
        self.next_id = id.checked_add(1).ok_or_else(invalid)?;
        let identity = (u128::from(self.history_epoch) << 64) | u128::from(id);
        self.entries.push(Entry {
            job: Job::new(JobId(identity), Schedule::Once { at_ns }, RetryPolicy::NONE),
            delta,
        });
        Ok(identity)
    }
    pub fn due(&self) -> Option<u64> {
        ic_jobs::next_due_ns(self.entries.iter().map(|entry| &entry.job))
    }
    pub fn begin(&mut self, now_ns: u64) -> Result<Option<DeliveryRecord>, Error> {
        Ok(
            ic_jobs::start_next(self.entries.iter_mut().map(|entry| &mut entry.job), now_ns)
                .map_err(|_| conflict())?
                .map(|(index, attempt)| DeliveryRecord {
                    attempt,
                    delta: self.entries[index].delta,
                }),
        )
    }
    pub fn receive(&mut self, delivery: DeliveryRecord) -> Result<(), Error> {
        let entry = self
            .entries
            .iter()
            .find(|entry| entry.job.record().id == delivery.attempt.execution.job)
            .ok_or_else(conflict)?;
        if delivery.delta != entry.delta {
            return Err(conflict());
        }
        if let Some(receipt) = self
            .receipts
            .iter()
            .find(|receipt| receipt.attempt.execution == delivery.attempt.execution)
        {
            return if *receipt == delivery {
                Ok(())
            } else {
                Err(conflict())
            };
        }
        if entry.job.active_attempt() != Some(delivery.attempt) {
            return Err(conflict());
        }
        self.total = self.total.checked_add(delivery.delta).ok_or_else(invalid)?;
        self.receipts.push(delivery);
        Ok(())
    }
    pub fn finish(
        &mut self,
        delivery: DeliveryRecord,
        now_ns: u64,
        confirmed: bool,
    ) -> Result<(), Error> {
        let entry = self
            .entries
            .iter_mut()
            .find(|entry| entry.job.record().id == delivery.attempt.execution.job)
            .ok_or_else(conflict)?;
        let outcome = if confirmed && self.receipts.contains(&delivery) {
            Outcome::Success
        } else {
            Outcome::Uncertain
        };
        entry
            .job
            .finish(delivery.attempt, now_ns, outcome)
            .map_err(|_| conflict())
    }
    pub fn restore_running(&mut self, now_ns: u64) -> Result<(), Error> {
        for entry in &mut self.entries {
            if matches!(entry.job.record().state, JobState::Running) {
                let attempt = entry.job.active_attempt().ok_or_else(invalid)?;
                entry
                    .job
                    .finish(attempt, now_ns, Outcome::Uncertain)
                    .map_err(|_| invalid())?;
            }
        }
        Ok(())
    }
    pub fn reconcile(&mut self, id: u128, sequence: u64, now_ns: u64) -> Result<(), Error> {
        let entry = self
            .entries
            .iter_mut()
            .find(|entry| entry.job.record().id.0 == id)
            .ok_or_else(conflict)?;
        let receipt = self
            .receipts
            .iter()
            .find(|receipt| {
                receipt.attempt.execution.job == entry.job.record().id
                    && receipt.attempt.sequence == sequence
                    && receipt.delta == entry.delta
            })
            .ok_or_else(conflict)?;
        if matches!(entry.job.record().state, JobState::Completed)
            && entry.job.record().sequence == sequence
        {
            return Ok(());
        }
        if matches!(entry.job.record().state, JobState::Running) {
            entry
                .job
                .finish(receipt.attempt, now_ns, Outcome::Success)
                .map_err(|_| conflict())
        } else {
            entry
                .job
                .resolve(receipt.attempt, now_ns, Outcome::Success)
                .map_err(|_| conflict())
        }
    }
    pub fn view(&self) -> JobsView {
        JobsView {
            total: self.total,
            receipts: u64::try_from(self.receipts.len()).expect("bounded receipts"),
            jobs: self
                .entries
                .iter()
                .map(|entry| {
                    let record = entry.job.record();
                    let state = match record.state {
                        JobState::Pending { .. } => StateView::Pending,
                        JobState::Running => StateView::Running,
                        JobState::Uncertain { .. } => StateView::Uncertain,
                        JobState::Completed => StateView::Completed,
                        JobState::Failed => StateView::Failed,
                        JobState::Paused { .. } => StateView::Paused,
                        JobState::Cancelled => StateView::Cancelled,
                    };
                    JobView {
                        id: record.id.0,
                        state,
                        sequence: record.sequence,
                    }
                })
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn running_restore_blocks_dispatch_until_exact_receipt_disposition() {
        let mut queue = Queue::empty(0);
        queue.enqueue(7, 10).unwrap();
        let delivery = queue.begin(10).unwrap().unwrap();
        queue.receive(delivery).unwrap();
        let mut restored = Queue::decode(&queue.encode().unwrap()).unwrap();
        restored.restore_running(11).unwrap();
        assert_eq!(restored.due(), None);
        assert!(restored.begin(100).unwrap().is_none());
        assert!(restored.reconcile(1, 2, 100).is_err());
        restored.reconcile(1, 1, 100).unwrap();
        restored.reconcile(1, 1, 100).unwrap();
        assert_eq!(
            restored.view(),
            JobsView {
                total: 7,
                receipts: 1,
                jobs: vec![JobView {
                    id: 1,
                    state: StateView::Completed,
                    sequence: 1
                }]
            }
        );
    }
    #[test]
    fn post_await_reload_preserves_independent_work_and_rejects_foreign_receipts() {
        let mut queue = Queue::empty(0);
        queue.enqueue(7, 10).unwrap();
        let delivery = queue.begin(10).unwrap().unwrap();
        let mut committed = Queue::decode(&queue.encode().unwrap()).unwrap();
        committed.enqueue(3, 20).unwrap();
        let mut wrong = delivery;
        wrong.delta = 8;
        assert!(committed.receive(wrong).is_err());
        committed.receive(delivery).unwrap();
        committed.receive(delivery).unwrap();
        committed.finish(delivery, 11, true).unwrap();
        assert_eq!(
            committed.view(),
            JobsView {
                total: 7,
                receipts: 1,
                jobs: vec![
                    JobView {
                        id: 1,
                        state: StateView::Completed,
                        sequence: 1
                    },
                    JobView {
                        id: 2,
                        state: StateView::Pending,
                        sequence: 0
                    }
                ]
            }
        );
        assert_eq!(committed.due(), Some(20));
    }
    #[test]
    fn missing_receipt_never_grants_retry_and_corrupt_or_future_records_refuse() {
        let mut queue = Queue::empty(0);
        queue.enqueue(7, 10).unwrap();
        let delivery = queue.begin(10).unwrap().unwrap();
        queue.finish(delivery, 11, false).unwrap();
        assert!(queue.reconcile(1, 1, 12).is_err());
        assert_eq!(queue.due(), None);
        let bytes = queue.encode().unwrap();
        assert!(Queue::decode(&bytes[..bytes.len() - 1]).is_err());
        assert!(Queue::decode(b"CJQ\x02").is_err());
        let mut record: QueueRecord =
            ic_wire::native::decode(&bytes[FORMAT.len()..], LIMITS).unwrap();
        record.entries[0].job.sequence = 0;
        let mut bad = FORMAT.to_vec();
        bad.extend(ic_wire::native::encode(&record, LIMITS).unwrap());
        assert!(Queue::decode(&bad).is_err());
    }
    #[test]
    fn queue_and_frame_bounds_are_explicit() {
        let mut queue = Queue::empty(0);
        for _ in 0..MAX_JOBS {
            queue.enqueue(1, 10).unwrap();
        }
        assert!(queue.enqueue(1, 10).is_err());
        assert!(queue.enqueue(0, 10).is_err());
        let bytes = queue.encode().unwrap();
        assert!(bytes.len() <= LIMITS.bytes + FORMAT.len());
        let _: QueueRecord = ic_wire::native::decode(&bytes[FORMAT.len()..], LIMITS)
            .expect("full queue fits decode budget");
        assert_eq!(Queue::decode(&bytes).unwrap().view(), queue.view());
        while let Some(delivery) = queue.begin(10).unwrap() {
            queue.receive(delivery).unwrap();
            queue.finish(delivery, 11, true).unwrap();
        }
        assert_eq!(
            Queue::decode(&queue.encode().unwrap()).unwrap().view(),
            queue.view()
        );
        assert_eq!(queue.view().receipts, 8);
    }
    #[test]
    fn new_installation_epochs_never_reuse_job_identity() {
        let first = Queue::empty(1).enqueue(1, 10).unwrap();
        let second = Queue::empty(2).enqueue(1, 10).unwrap();
        assert_ne!(first, second);
    }
}
