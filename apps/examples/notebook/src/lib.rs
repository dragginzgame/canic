//! One public scratch note for the local Backup walkthrough.

mod endpoints;

use canic::recovery::Command;
use ic_platform_runtime::system as api;
use ic_platform_types::Principal;
use ic_storage::{DefaultMemoryImpl, adapter::StableCell};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;

const KEY: &str = "notebook.record.v1";
const MAX_BYTES: usize = 256;

thread_local! {
    static NOTE: RefCell<Option<StableCell<Vec<u8>>>> = const { RefCell::new(None) };
}

/// The installer owns recovery. Public edits preserve its generation and fence.
#[derive(Deserialize, Serialize)]
struct Note {
    text: String,
    owner: Principal,
    generation: u64,
    fenced: bool,
}

impl Note {
    fn recover(&mut self, command: Command) {
        assert_eq!(api::caller(), self.owner, "notebook recovery owner");
        match command {
            Command::Fence(generation) => {
                assert!(
                    (self.fenced && generation == self.generation)
                        || (!self.fenced && generation > self.generation),
                    "invalid fence generation"
                );
                self.generation = generation;
                self.fenced = true;
            }
            Command::Accept {
                checkpoint,
                generation,
            } => {
                assert!(
                    self.fenced && self.generation == checkpoint && generation > checkpoint,
                    "invalid recovery generation"
                );
                self.generation = generation;
            }
            Command::Open(generation) => {
                assert!(
                    self.fenced && self.generation == generation,
                    "invalid opening generation"
                );
                self.fenced = false;
            }
        }
    }

    fn encode(&self) -> Vec<u8> {
        ic_wire::encode(self, ic_wire::Encoding::Payload, canic::method::LIMITS)
            .expect("encode notebook record")
    }
}

fn read_note() -> Note {
    NOTE.with_borrow(|slot| {
        ic_wire::decode(
            slot.as_ref().expect("notebook open").get(),
            ic_wire::Encoding::Payload,
            canic::method::LIMITS,
        )
        .expect("decode notebook record")
    })
}

fn write_note(note: &Note) {
    NOTE.with_borrow_mut(|slot| {
        slot.as_mut().expect("notebook open").set(note.encode());
    });
}

fn install() {
    open(true);
}

fn reopen() {
    open(false);
}

fn open(fresh: bool) {
    let runtime =
        canic::storage::bootstrap(DefaultMemoryImpl::default(), fresh, "notebook", &[KEY], 8);
    let initial = fresh.then(|| {
        let owner = api::caller();
        assert_ne!(
            owner,
            Principal::anonymous(),
            "named notebook installer required"
        );
        Note {
            text: String::new(),
            owner,
            generation: 0,
            fenced: false,
        }
        .encode()
    });
    let cell = canic::storage::cell(&runtime, KEY, canic::method::LIMITS.bytes, initial);
    NOTE.with_borrow_mut(|slot| *slot = Some(cell));
    let note = read_note();
    assert!(
        note.text.len() <= MAX_BYTES
            && note.owner != Principal::anonymous()
            && (!note.fenced || note.generation != 0),
        "invalid notebook record"
    );
}
