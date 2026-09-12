//! Module: fixture_importer
//!
//! Responsibility: qualify neutral fixture receipts in the disposable Hub/Shard apps.
//! Does not own: provisioning, grants, Canic readiness or production application schemas.
//! Boundary: one application cell owns imported bytes and the durable completion checkpoint.

use canic::{
    api::fixture_provisioning::{FixtureImporter, FixtureProvisioningApi},
    dto::fixture_provisioning::{
        FixtureAssignment, FixtureImportError, FixtureImportProgress, FixtureImportReceipt,
    },
};
use ic_memory::{
    RuntimeMemory,
    ic_stable_structures::{Cell, DefaultMemoryImpl},
};
use std::cell::RefCell;

const PAYLOAD: &[u8] = b"reviewed fixture source";

#[derive(candid::CandidType, canic::__internal::serde::Deserialize)]
#[serde(crate = "canic::__internal::serde")]
struct ImportRecord {
    held: bool,
    bytes: Vec<u8>,
    progress: Option<FixtureImportProgress>,
}

thread_local! {
    static IMPORT: RefCell<Cell<Vec<u8>, RuntimeMemory<DefaultMemoryImpl>>> = RefCell::new(Cell::init(
        canic::memory::ic_memory_key!(authority = "test", key = "test.fixture_import.v1", ty = ImportRecord, id = 201),
        Vec::new(),
    ));
}

struct Importer;
static IMPORTER: Importer = Importer;

/// Restore the application checkpoint before registering each fresh heap's participant.
///
/// # Panics
/// Traps on corrupt application state or duplicate importer registration.
pub fn restore() {
    let _ = load();
    FixtureProvisioningApi::register(&IMPORTER).expect("register disposable fixture importer");
}

fn load() -> ImportRecord {
    IMPORT.with_borrow(|cell| {
        if cell.get().is_empty() {
            ImportRecord {
                held: true,
                bytes: Vec::new(),
                progress: None,
            }
        } else {
            candid::decode_one(cell.get()).expect("decode application import record")
        }
    })
}

fn persist(record: &ImportRecord) {
    IMPORT.with_borrow_mut(|cell| {
        cell.set(candid::encode_one(record).expect("encode application import record"));
    });
}

impl FixtureImporter for Importer {
    fn progress(
        &self,
        _: &FixtureAssignment,
    ) -> Result<Option<FixtureImportProgress>, FixtureImportError> {
        let record = load();
        if record.held {
            return Err(FixtureImportError::NotReady);
        }
        Ok(record.progress)
    }

    fn begin(&self, assignment: &FixtureAssignment) -> Result<(), FixtureImportError> {
        let mut record = load();
        record.progress = Some(FixtureImportProgress {
            binding: assignment.grant.binding.clone(),
            next_chunk: 0,
            receipt: None,
        });
        persist(&record);
        Ok(())
    }

    fn apply_chunk(
        &self,
        _: &FixtureAssignment,
        index: u32,
        bytes: &[u8],
    ) -> Result<(), FixtureImportError> {
        if index != 0 || bytes != PAYLOAD {
            return Err(FixtureImportError::Application { code: 1 });
        }
        let mut record = load();
        record.bytes = bytes.to_vec();
        record
            .progress
            .as_mut()
            .ok_or(FixtureImportError::Progress)?
            .next_chunk = 1;
        persist(&record);
        Ok(())
    }

    fn validate_step(&self, assignment: &FixtureAssignment) -> Result<(), FixtureImportError> {
        let mut record = load();
        if record.bytes != PAYLOAD || assignment.descriptor.completion_summary != [3; 32] {
            return Err(FixtureImportError::Application { code: 2 });
        }
        record
            .progress
            .as_mut()
            .ok_or(FixtureImportError::Progress)?
            .receipt = Some(Box::new(FixtureImportReceipt {
            binding: assignment.grant.binding.clone(),
            completion_summary: [3; 32],
        }));
        persist(&record);
        Ok(())
    }
}

fn require_controller() -> Result<(), String> {
    if ic_cdk::api::is_controller(&ic_cdk::api::msg_caller()) {
        Ok(())
    } else {
        Err("test instrumentation requires a controller".to_string())
    }
}

/// Release the test-only database hold without importing rows or writing a receipt.
#[ic_cdk::update(guard = "require_controller")]
fn test_release_fixture() {
    let mut record = load();
    record.held = false;
    persist(&record);
}
