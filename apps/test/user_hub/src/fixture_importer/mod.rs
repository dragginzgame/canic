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

#[derive(canic::__internal::serde::Deserialize, canic::__internal::serde::Serialize)]
#[serde(crate = "canic::__internal::serde")]
struct ImportRecord {
    held: bool,
    fault: u8,
    applied_chunks: u32,
    bytes: Vec<u8>,
    progress: Option<FixtureImportProgress>,
}

thread_local! {
    static IMPORT: RefCell<Cell<Vec<u8>, DefaultMemoryImpl>> = RefCell::new({
        let memory = canic::memory::open(MEMORY_KEY);
        if ic_storage::Memory::size(&memory) == 0 {
            Cell::create(memory, Vec::new()).expect("open fixture collection")
        } else {
            Cell::load(memory, canic::__internal::contracts::serialization::STORAGE_LIMITS.bytes).expect("retained fixture import cell")
        }
    });
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
                fault: 0,
                applied_chunks: 0,
                bytes: Vec::new(),
                progress: None,
            }
        } else {
            canic::__internal::contracts::serialization::deserialize(cell.get())
                .expect("decode application import record")
        }
    })
}

fn persist(record: &ImportRecord) {
    IMPORT.with_borrow_mut(|cell| {
        cell.set(
            canic::__internal::contracts::serialization::serialize(record)
                .expect("encode application import record"),
        )
        .expect("persist application import record");
    });
}

impl FixtureImporter for Importer {
    fn progress(
        &self,
        _: &FixtureAssignment,
    ) -> Result<Option<FixtureImportProgress>, FixtureImportError> {
        let record = load();
        if record.fault == 6 {
            return Err(FixtureImportError::Application { code: 42 });
        }
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
        if record.fault == 1 {
            return Err(FixtureImportError::Application { code: 12 });
        }
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
        record.applied_chunks += 1;
        record
            .progress
            .as_mut()
            .ok_or(FixtureImportError::Progress)?
            .next_chunk = 1;
        persist(&record);
        match record.fault {
            2 => return Err(FixtureImportError::Application { code: 12 }),
            4 => ic_platform::cdk::api::trap("disposable import callback fault"),
            5 => {
                record.progress.as_mut().unwrap().next_chunk = 9;
                persist(&record);
            }
            _ => {}
        }
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
        if record.fault == 3 {
            return Err(FixtureImportError::Application { code: 12 });
        }
        Ok(())
    }
}

fn require_controller() -> Result<(), String> {
    if ic_platform::cdk::api::is_controller(&ic_platform::cdk::api::msg_caller()) {
        Ok(())
    } else {
        Err("test instrumentation requires a controller".to_string())
    }
}

/// Release the test-only database hold without importing rows or writing a receipt.
#[ic_platform::cdk::update(
    guard = "require_controller",
    decode_with = "decode_release",
    encode_with = "encode_release"
)]
fn test_release_fixture() {
    let mut record = load();
    record.held = false;
    persist(&record);
}

canic::__canic_register_raw_wire_schema!(
    "test_release_fixture",
    Update,
    crate::protocol::RELEASE_FIXTURE,
    (),
    (),
    crate::protocol::REQUEST_LIMITS,
    crate::protocol::RESPONSE_LIMITS
);

fn decode_release(bytes: Vec<u8>) {
    ic_wire::native::decode_request::<()>(
        crate::protocol::RELEASE_FIXTURE,
        &bytes,
        crate::protocol::REQUEST_LIMITS,
    )
    .unwrap_or_else(|error| ic_platform::cdk::api::trap(error.to_string()));
}

fn encode_release() -> Vec<u8> {
    ic_wire::native::encode_response(
        crate::protocol::RELEASE_FIXTURE,
        &(),
        crate::protocol::RESPONSE_LIMITS,
    )
    .expect("encode fixture release reply")
}

/// Read-only evidence independent of the deliberately failing application callback.
#[derive(canic::__internal::serde::Deserialize, canic::__internal::serde::Serialize)]
#[serde(crate = "canic::__internal::serde")]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
struct ImportProbe {
    applied_chunks: u32,
    next_chunk: Option<u32>,
    complete: bool,
    recovery:
        Option<canic::__internal::contracts::values::fixture_import::FixtureImportRecoveryStatus>,
    failure: Option<canic::dto::fixture_provisioning::FixtureImportFailure>,
}

/// Select a disposable fault and optionally request explicit same-release recovery.
#[ic_platform::cdk::update(
    guard = "require_controller",
    decode_with = "decode_fault",
    encode_with = "encode_fault"
)]
fn test_fixture_fault(
    fault: u8,
    retry: bool,
) -> Result<(), canic::dto::fixture_provisioning::FixtureImportError> {
    let mut record = load();
    record.fault = fault;
    record.held = false;
    persist(&record);
    if retry {
        FixtureProvisioningApi::retry_failed()?;
    }
    Ok(())
}

#[ic_platform::cdk::query(
    guard = "require_controller",
    decode_with = "decode_probe",
    encode_with = "encode_probe"
)]
fn test_fixture_probe() -> ImportProbe {
    let record = load();
    let failure = match FixtureProvisioningApi::status() {
        Ok(canic::dto::fixture_provisioning::FixtureProvisioningStatus::Failed(failure)) => {
            Some(failure)
        }
        _ => None,
    };
    ImportProbe {
        applied_chunks: record.applied_chunks,
        next_chunk: record.progress.as_ref().map(|progress| progress.next_chunk),
        complete: record
            .progress
            .is_some_and(|progress| progress.receipt.is_some()),
        recovery: FixtureProvisioningApi::recovery_status(),
        failure,
    }
}

canic::__canic_register_raw_wire_schema!("test_fixture_fault", Update, crate::protocol::IMPORT_FAULT, (u8, bool), Result<(), FixtureImportError>, crate::protocol::REQUEST_LIMITS, crate::protocol::RESPONSE_LIMITS);
canic::__canic_register_raw_wire_schema!(
    "test_fixture_probe",
    Query,
    crate::protocol::IMPORT_PROBE,
    (),
    ImportProbe,
    crate::protocol::REQUEST_LIMITS,
    crate::protocol::RESPONSE_LIMITS
);

fn decode_fault(bytes: Vec<u8>) -> (u8, bool) {
    ic_wire::native::decode_request(
        crate::protocol::IMPORT_FAULT,
        &bytes,
        crate::protocol::REQUEST_LIMITS,
    )
    .unwrap()
}
fn encode_fault(reply: Result<(), FixtureImportError>) -> Vec<u8> {
    ic_wire::native::encode_response(
        crate::protocol::IMPORT_FAULT,
        &reply,
        crate::protocol::RESPONSE_LIMITS,
    )
    .unwrap()
}
fn decode_probe(bytes: Vec<u8>) {
    ic_wire::native::decode_request(
        crate::protocol::IMPORT_PROBE,
        &bytes,
        crate::protocol::REQUEST_LIMITS,
    )
    .unwrap()
}
fn encode_probe(reply: ImportProbe) -> Vec<u8> {
    ic_wire::native::encode_response(
        crate::protocol::IMPORT_PROBE,
        &reply,
        crate::protocol::RESPONSE_LIMITS,
    )
    .unwrap()
}
