//! Module: fixture_importer
//!
//! Responsibility: qualify neutral fixture receipts in the disposable Hub/Shard apps.
//! Does not own: provisioning, grants, Canic readiness or production application schemas.
//! Boundary: one application cell owns imported bytes and the durable completion checkpoint.

use canic::__internal::core::cdk::structures::{DefaultMemoryImpl, StableCell as Cell};
use canic::{
    api::fixture_provisioning::{FixtureImporter, FixtureProvisioningApi},
    dto::fixture_provisioning::{
        FixtureAssignment, FixtureImportError, FixtureImportProgress, FixtureImportReceipt,
    },
};
use std::cell::RefCell;

pub const MEMORY_KEY: &str = "test.fixture_import.v1";

const PAYLOAD: &[u8] = b"reviewed fixture source";

#[derive(ic_platform::candid::contract::CandidType, canic::__internal::serde::Deserialize)]
#[candid_path("ic_platform::candid::contract")]
#[serde(crate = "canic::__internal::serde")]
struct ImportRecord {
    bytes: Vec<u8>,
    progress: Option<FixtureImportProgress>,
}

thread_local! {
    static IMPORT: RefCell<Cell<Vec<u8>, DefaultMemoryImpl>> = RefCell::new(Cell::init(
        canic::memory::open(MEMORY_KEY),
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
                bytes: Vec::new(),
                progress: None,
            }
        } else {
            ic_platform::candid::decoder::decode_one(cell.get())
                .expect("decode application import record")
        }
    })
}

fn persist(record: &ImportRecord) {
    IMPORT.with_borrow_mut(|cell| {
        cell.set(
            ic_platform::candid::contract::encode_one(record)
                .expect("encode application import record"),
        );
    });
}

impl FixtureImporter for Importer {
    fn progress(
        &self,
        _: &FixtureAssignment,
    ) -> Result<Option<FixtureImportProgress>, FixtureImportError> {
        Ok(load().progress)
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
