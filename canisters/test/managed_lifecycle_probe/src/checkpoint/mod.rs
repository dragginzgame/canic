//! Application-owned durable freeze and counter used to qualify the checkpoint SDK.
//! No Root sequencing, snapshot authority or production application policy lives here.

mod ops;
mod protocol;

use canic::__internal::core::cdk::structures::{DefaultMemoryImpl, StableCell as Cell};
use canic::api::application_checkpoint::{
    ApplicationCheckpointPhase as Phase, ApplicationCheckpointRecord as Record,
    ApplicationCheckpointRootPredicate, ApplicationCheckpointTransition as Transition,
};
use canic::{Error, prelude::*};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;

pub const KEY: &str = "managed_probe.checkpoint.v1";
type Storage = Cell<Vec<u8>, DefaultMemoryImpl>;

/// Business state and its freeze record are committed in one application-owned cell.
#[derive(Default, Deserialize, Serialize)]
struct BusinessRecord {
    checkpoint: Option<Record>,
    counter: u64,
    prepare_runs: u64,
    release_runs: u64,
}

/// Read-only evidence of business admission and exactly-once completed hooks.
#[derive(Deserialize, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct CheckpointView {
    retained: Option<protocol::ParticipantStatus>,
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
    serde_cbor::to_writer(&mut initial, &BusinessRecord::default()).unwrap();
    STORAGE.with_borrow_mut(|storage| *storage = Some(Cell::init(memory, initial)));
}

fn read() -> BusinessRecord {
    STORAGE.with_borrow(|storage| {
        serde_cbor::from_reader(
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
    serde_cbor::to_writer(&mut bytes, record).unwrap();
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

#[update(requires(custom(ApplicationCheckpointRootPredicate::new(request.binding.clone().into()))), wire(id = protocol::TRANSITION, request = protocol::REQUEST_LIMITS, response = crate::protocol::RESPONSE_LIMITS))]
fn checkpoint_transition(request: protocol::TransitionRequest) -> Result<Phase, Error> {
    let binding = request.binding.into();
    let phase = request.phase;
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

#[update(requires(custom(ApplicationCheckpointRootPredicate::new(request.binding.clone().into()))), wire(id = protocol::RESTORE, request = protocol::REQUEST_LIMITS, response = crate::protocol::RESPONSE_LIMITS))]
fn checkpoint_restore(request: protocol::RestoreRequest) -> Result<Phase, Error> {
    let captured = request.captured.into();
    let binding = request.binding.into();
    let mut state = read();
    let previous = state.checkpoint.as_ref().ok_or_else(invalid)?;
    let record = selected(Record::restore(previous, &captured, binding).map_err(|_| invalid())?);
    let phase = record.phase();
    state.checkpoint = Some(record);
    commit(&state);
    Ok(phase)
}

#[update(public, wire(id = protocol::INCREMENT, request = crate::protocol::REQUEST_LIMITS, response = crate::protocol::RESPONSE_LIMITS))]
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

#[query(public, wire(id = protocol::VIEW, request = crate::protocol::REQUEST_LIMITS, response = protocol::RESPONSE_LIMITS))]
fn checkpoint_view() -> Result<crate::checkpoint::CheckpointView, Error> {
    let state = read();
    Ok(CheckpointView {
        retained: state.checkpoint.map(|record| protocol::ParticipantStatus {
            binding: record.binding().clone().into(),
            phase: record.phase(),
        }),
        counter: state.counter,
        prepare_runs: state.prepare_runs,
        release_runs: state.release_runs,
    })
}

/// Application-owned synchronous business freeze and idempotent release hook.
#[update(
    requires(custom(canic::api::application_checkpoint::ApplicationCheckpointCommandPredicate::new(request.clone()))),
    wire(id = canic::protocol::application_checkpoint::PARTICIPANT,
         request = canic::protocol::application_checkpoint::LIMITS,
         response = canic::protocol::application_checkpoint::LIMITS)
)]
fn canic_application_checkpoint(
    request: canic::dto::application_checkpoint::ApplicationCheckpointCommand,
) -> Result<canic::dto::application_checkpoint::ApplicationCheckpointCommand, Error> {
    let binding = canic::api::application_checkpoint::binding_from_command(&request)?;
    let mut state = read();
    let previous = state.checkpoint.as_ref();
    let record = match request.phase {
        Phase::Preparing => {
            selected(Record::prepare(previous, binding.clone()).map_err(|_| invalid())?)
        }
        Phase::Releasing => {
            selected(Record::release(previous, binding.clone()).map_err(|_| invalid())?)
        }
        _ => return Err(invalid()),
    };
    let next = match record.phase() {
        Phase::Preparing => {
            state.prepare_runs += 1;
            record.acknowledge_prepared(&binding)
        }
        Phase::Releasing => {
            state.release_runs += 1;
            record.acknowledge_released(&binding)
        }
        Phase::Prepared | Phase::Released => Ok(record),
    }
    .map_err(|_| invalid())?;
    let phase = next.phase();
    state.checkpoint = Some(next);
    commit(&state);
    Ok(canic::dto::application_checkpoint::ApplicationCheckpointCommand { phase, ..request })
}
