//! Application-owned durable freeze and counter used to qualify the checkpoint SDK.
//! No Root sequencing, snapshot authority or production application policy lives here.

use canic::api::application_checkpoint::{
    ApplicationCheckpointPhase as Phase, ApplicationCheckpointRecord as Record,
    ApplicationCheckpointRootPredicate, ApplicationCheckpointTransition as Transition,
};
use canic::{Error, prelude::*};
use canic::{
    dto::application_checkpoint::ApplicationCheckpointParticipantStatus,
    ids::ApplicationCheckpointBinding,
};
use ic_memory::DefaultMemoryImpl;
use ic_platform::candid::contract::CandidType;
use ic_storage::adapter::StableCell;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;

pub const KEY: &str = "managed_probe.checkpoint.v1";
type Storage = StableCell<Vec<u8>, DefaultMemoryImpl>;

/// Business state and its freeze record are committed in one application-owned cell.
#[derive(Default, Deserialize, Serialize)]
struct BusinessRecord {
    checkpoint: Option<Record>,
    counter: u64,
    prepare_runs: u64,
    release_runs: u64,
}

/// Read-only evidence of business admission and exactly-once completed hooks.
#[derive(CandidType)]
#[candid_path("ic_platform::candid::contract")]
struct CheckpointView {
    retained: Option<ApplicationCheckpointParticipantStatus>,
    counter: u64,
    prepare_runs: u64,
    release_runs: u64,
}

thread_local! {
    static STORAGE: RefCell<Option<Storage>> = const { RefCell::new(None) };
}

pub fn open() {
    let memory = canic::memory::open(KEY);
    let mut initial = Vec::new();
    ciborium::ser::into_writer(&BusinessRecord::default(), &mut initial).unwrap();
    STORAGE.with_borrow_mut(|storage| *storage = Some(StableCell::init(memory, initial)));
}

fn read() -> BusinessRecord {
    STORAGE.with_borrow(|storage| {
        ciborium::de::from_reader(
            storage
                .as_ref()
                .expect("restored checkpoint")
                .get()
                .as_slice(),
        )
        .expect("valid business record")
    })
}

fn commit(record: &BusinessRecord) {
    let mut bytes = Vec::new();
    ciborium::ser::into_writer(record, &mut bytes).unwrap();
    STORAGE.with_borrow_mut(|storage| {
        storage.as_mut().expect("restored checkpoint").set(bytes);
    });
}

const fn invalid() -> Error {
    Error::from_registered(canic::diagnostics::codes::AUTHORITY_INVALID_STATE)
}

fn selected(transition: Transition) -> Record {
    match transition {
        Transition::Changed(record) | Transition::Retained(record) => record,
    }
}

#[canic_update(requires(custom(ApplicationCheckpointRootPredicate::new(binding.clone()))))]
fn checkpoint_transition(
    binding: ApplicationCheckpointBinding,
    phase: Phase,
) -> Result<Phase, Error> {
    let mut state = read();
    let previous = state.checkpoint.as_ref();
    let record = match phase {
        Phase::Preparing => Record::prepare(previous, binding).map(selected),
        Phase::Releasing => Record::release(previous, binding).map(selected),
        Phase::Prepared | Phase::Released => {
            let previous = previous.ok_or_else(invalid)?;
            let changed = previous.phase() != phase;
            let next = if phase == Phase::Prepared {
                previous.acknowledge_prepared(&binding)
            } else {
                previous.acknowledge_released(&binding)
            };
            if next.is_ok() && changed {
                if phase == Phase::Prepared {
                    state.prepare_runs += 1;
                } else {
                    state.release_runs += 1;
                }
            }
            next
        }
    }
    .map_err(|_| invalid())?;
    let phase = record.phase();
    state.checkpoint = Some(record);
    commit(&state);
    Ok(phase)
}

#[canic_update(requires(custom(ApplicationCheckpointRootPredicate::new(binding.clone()))))]
fn checkpoint_restore(
    captured: ApplicationCheckpointBinding,
    binding: ApplicationCheckpointBinding,
) -> Result<Phase, Error> {
    let mut state = read();
    let previous = state.checkpoint.as_ref().ok_or_else(invalid)?;
    let record = selected(Record::restore(previous, &captured, binding).map_err(|_| invalid())?);
    let phase = record.phase();
    state.checkpoint = Some(record);
    commit(&state);
    Ok(phase)
}

#[canic_update(public)]
fn checkpoint_increment() -> Result<u64, Error> {
    let mut state = read();
    if state
        .checkpoint
        .as_ref()
        .is_some_and(|record| !record.mutation_allowed())
    {
        return Err(invalid());
    }
    state.counter += 1;
    commit(&state);
    Ok(state.counter)
}

#[canic_query(public)]
fn checkpoint_view() -> Result<CheckpointView, Error> {
    let state = read();
    Ok(CheckpointView {
        retained: state
            .checkpoint
            .map(|record| ApplicationCheckpointParticipantStatus {
                binding: record.binding().clone(),
                phase: record.phase(),
            }),
        counter: state.counter,
        prepare_runs: state.prepare_runs,
        release_runs: state.release_runs,
    })
}
