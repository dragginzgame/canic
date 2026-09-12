//! Module: fixture_provisioning
//!
//! Responsibility: qualify application-owned IcyDB rows and a separate durable checkpoint.
//! Does not own: production fixture transport, source grants, or Fleet placement.
//! Boundary: controller-only probe calls retain Canic's Prepared fence; commits never await.

pub mod consumer;
mod transport;

use std::cell::RefCell;

use candid::{CandidType, Deserialize, Principal};
use canic::{
    __internal::core::api::fleet_activation::FleetActivationApi, Error,
    dto::fleet_activation::FleetActivationIdentity, prelude::*,
};
use ic_memory::{
    RuntimeMemory,
    ic_stable_structures::{Cell, DefaultMemoryImpl},
};
use icydb::{
    db::{DynamicQuery, StructuralPatch, WriteCell},
    prelude::{FilterExpr, asc},
    value::{InputValue, OutputValue, PublicValue},
};
use sha2::{Digest, Sha256};

canic::memory::ic_memory_range!(authority = "test", start = 200, end = 200, mode = Allowed);

struct FixtureCheckpoint;

// These dimensions define a small qualification dataset, not product limits.
const MAX_PROBE_ROWS: usize = 256;
const ENTITY: &str = "FixtureProbeRow";

thread_local! {
    static CHECKPOINT: RefCell<Cell<Vec<u8>, RuntimeMemory<DefaultMemoryImpl>>> = RefCell::new(
        Cell::init(canic::memory::ic_memory_key!(authority = "test", key = "test.fixture_checkpoint.v1", ty = FixtureCheckpoint, id = 200), Vec::new())
    );
}

/// Probe identity distinguishes same-Principal installations and selected content.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
struct ImportBinding {
    target: Principal,
    installation: [u8; 32],
    content: [u8; 32],
}

/// Probe fault placement around the synchronous application commit.
#[derive(CandidType, Clone, Copy, Deserialize)]
enum CommitFault {
    None,
    ErrorBeforeRows,
    TrapBeforeRows,
    TrapAfterRows,
    TrapAfterCheckpoint,
    // Negative control: demonstrates that a returned error is not rollback.
    ReturnErrorAfterRows,
}

/// Application-owned stable state, independent from IcyDB's row storage.
#[derive(CandidType, Clone, Deserialize)]
struct ImportRecord {
    binding: ImportBinding,
    runtime_identity: FleetActivationIdentity,
    source: Option<Principal>,
    chunks: Vec<[u8; 32]>,
    next: u64,
    validated: u64,
    receipt: Option<ImportBinding>,
}

/// Bounded observation of the application cursor and exact completion receipt.
#[derive(CandidType)]
struct ImportSnapshot {
    binding: ImportBinding,
    next: u64,
    validated: u64,
    receipt: Option<ImportBinding>,
    instructions: u64,
}

/// Closed qualification outcomes; these are not shipped product diagnostics.
#[derive(CandidType, Debug, Deserialize, Eq, PartialEq)]
enum ImportError {
    Binding,
    Bounds,
    Busy,
    Conflict,
    Database,
    Injected,
    NotBegun,
    NotReady,
    Sequence,
    Source,
    Validation,
}

#[icydb::request_execution]
#[canic_update(requires(caller::is_controller()))]
async fn fixture_begin(
    binding: ImportBinding,
    chunks: Vec<[u8; 32]>,
    source: Option<Principal>,
) -> Result<Result<ImportSnapshot, ImportError>, Error> {
    Ok(begin(binding, chunks, source))
}

#[icydb::request_execution]
#[canic_update(requires(caller::is_controller()))]
async fn fixture_commit(
    binding: ImportBinding,
    index: u64,
    bytes: Vec<u8>,
    fault: CommitFault,
) -> Result<Result<ImportSnapshot, ImportError>, Error> {
    Ok(commit(&binding, index, &bytes, fault))
}

#[icydb::request_execution]
#[canic_update(requires(caller::is_controller()))]
async fn fixture_validate(
    binding: ImportBinding,
) -> Result<Result<ImportSnapshot, ImportError>, Error> {
    Ok(validate(&binding))
}

#[ic_cdk::query(guard = "crate::require_test_controller")]
async fn fixture_progress() -> Result<Result<ImportSnapshot, ImportError>, Error> {
    Ok(load().map(snapshot))
}

#[icydb::request_execution]
#[ic_cdk::query(guard = "crate::require_test_controller")]
async fn fixture_first_row() -> Result<Result<Vec<(u64, u64)>, ImportError>, Error> {
    Ok(crate::db()
        .map_err(|_| ImportError::Database)
        .and_then(|database| {
            database
                .execute_trusted_live_page(&query(), None)
                .map_err(|_| ImportError::Database)
                .and_then(|page| page.rows.iter().map(|row| read_row(row)).collect())
        }))
}

fn begin(
    binding: ImportBinding,
    chunks: Vec<[u8; 32]>,
    source: Option<Principal>,
) -> Result<ImportSnapshot, ImportError> {
    let identity = installed_identity()?;
    if binding.target != ic_cdk::api::canister_self()
        || binding.installation != identity.operation_id
    {
        return Err(ImportError::Binding);
    }
    if chunks.is_empty() || chunks.len() > MAX_PROBE_ROWS {
        return Err(ImportError::Bounds);
    }
    let content: [u8; 32] =
        Sha256::digest(chunks.iter().flatten().copied().collect::<Vec<_>>()).into();
    if content != binding.content {
        return Err(ImportError::Binding);
    }
    begin_selected(binding, chunks, source)
}

fn begin_selected(
    binding: ImportBinding,
    chunks: Vec<[u8; 32]>,
    source: Option<Principal>,
) -> Result<ImportSnapshot, ImportError> {
    let runtime_identity = installed_identity()?;
    if binding.target != ic_cdk::api::canister_self()
        || binding.installation != runtime_identity.operation_id
    {
        return Err(ImportError::Binding);
    }
    if chunks.is_empty() || chunks.len() > MAX_PROBE_ROWS {
        return Err(ImportError::Bounds);
    }
    if let Ok(record) = load() {
        return if record.binding == binding
            && record.runtime_identity == runtime_identity
            && record.source == source
            && record.chunks == chunks
        {
            Ok(snapshot(record))
        } else {
            Err(ImportError::Conflict)
        };
    }
    let database = crate::db().map_err(|_| ImportError::Database)?;
    if !database
        .execute_trusted_live_page(&query(), None)
        .map_err(|_| ImportError::Database)?
        .rows
        .is_empty()
    {
        return Err(ImportError::Conflict);
    }
    let record = ImportRecord {
        binding,
        runtime_identity,
        source,
        chunks,
        next: 0,
        validated: 0,
        receipt: None,
    };
    persist(&record);
    Ok(snapshot(record))
}

fn commit(
    binding: &ImportBinding,
    index: u64,
    bytes: &[u8],
    fault: CommitFault,
) -> Result<ImportSnapshot, ImportError> {
    let mut record = bound(binding)?;
    if bytes.len() != 16 {
        return Err(ImportError::Bounds);
    }
    let slot = usize::try_from(index).map_err(|_| ImportError::Bounds)?;
    let digest: [u8; 32] = Sha256::digest(bytes).into();
    if record.chunks.get(slot) != Some(&digest) {
        return Err(ImportError::Conflict);
    }
    if index < record.next {
        return Ok(snapshot(record));
    }
    if index != record.next {
        return Err(ImportError::Sequence);
    }
    let (id, value) = decode_row(bytes)?;
    if id != index {
        return Err(ImportError::Conflict);
    }
    match fault {
        CommitFault::ErrorBeforeRows => return Err(ImportError::Injected),
        CommitFault::TrapBeforeRows => ic_cdk::trap("fixture fault before rows"),
        _ => {}
    }
    let patch = StructuralPatch::new()
        .field("id", WriteCell::Value(InputValue::nat64(id)))
        .field("value", WriteCell::Value(InputValue::nat64(value)));
    crate::db()
        .map_err(|_| ImportError::Database)?
        .execute_trusted_structural_insert_batch(ENTITY, vec![patch])
        .map_err(|_| ImportError::Database)?;
    match fault {
        CommitFault::TrapAfterRows => ic_cdk::trap("fixture fault after rows"),
        CommitFault::ReturnErrorAfterRows => return Err(ImportError::Injected),
        _ => {}
    }
    // From the first successful row mutation through checkpoint persistence,
    // failure must trap. A returned error here would commit only the rows.
    record.next += 1;
    persist(&record);
    if matches!(fault, CommitFault::TrapAfterCheckpoint) {
        ic_cdk::trap("fixture fault after checkpoint");
    }
    Ok(snapshot(record))
}

fn validate(binding: &ImportBinding) -> Result<ImportSnapshot, ImportError> {
    let mut record = bound(binding)?;
    if record.receipt.is_some() {
        return Ok(snapshot(record));
    }
    if record.next != record.chunks.len() as u64 {
        return Err(ImportError::NotReady);
    }
    // One stable-key page per invocation; finalization never rescans the dataset.
    let page = crate::db()
        .map_err(|_| ImportError::Database)?
        .execute_trusted_live_page(
            &query().filter(FilterExpr::gte("id", record.validated)),
            None,
        )
        .map_err(|_| ImportError::Database)?;
    let exhausted = page.rows.is_empty();
    for row in page.rows {
        let (id, value) = read_row(&row)?;
        let bytes = [id.to_le_bytes(), value.to_le_bytes()].concat();
        let digest: [u8; 32] = Sha256::digest(bytes).into();
        let slot = usize::try_from(record.validated).map_err(|_| ImportError::Validation)?;
        if id != record.validated || record.chunks.get(slot) != Some(&digest) {
            return Err(ImportError::Validation);
        }
        record.validated += 1;
    }
    if exhausted {
        if record.validated != record.next {
            return Err(ImportError::Validation);
        }
        record.receipt = Some(record.binding.clone());
    }
    persist(&record);
    Ok(snapshot(record))
}

fn query() -> DynamicQuery {
    DynamicQuery::new(ENTITY)
        .select(["id", "value"])
        .order_by(asc("id"))
        .limit(1)
}

fn bound(binding: &ImportBinding) -> Result<ImportRecord, ImportError> {
    let record = load()?;
    if record.binding != *binding
        || binding.target != ic_cdk::api::canister_self()
        || record.runtime_identity != installed_identity()?
    {
        return Err(ImportError::Binding);
    }
    Ok(record)
}

fn decode_row(bytes: &[u8]) -> Result<(u64, u64), ImportError> {
    let id = bytes
        .get(..8)
        .and_then(|part| part.try_into().ok())
        .map(u64::from_le_bytes)
        .ok_or(ImportError::Bounds)?;
    let value = bytes
        .get(8..16)
        .and_then(|part| part.try_into().ok())
        .map(u64::from_le_bytes)
        .ok_or(ImportError::Bounds)?;
    Ok((id, value))
}

fn load() -> Result<ImportRecord, ImportError> {
    CHECKPOINT.with_borrow(|cell| {
        if cell.get().is_empty() {
            return Err(ImportError::NotBegun);
        }
        Ok(candid::decode_one(cell.get()).expect("exact probe checkpoint codec"))
    })
}

fn persist(record: &ImportRecord) {
    let bytes = candid::encode_one(record).expect("bounded probe checkpoint encoding");
    CHECKPOINT.with_borrow_mut(|cell| {
        cell.set(bytes);
    });
}

fn snapshot(record: ImportRecord) -> ImportSnapshot {
    ImportSnapshot {
        binding: record.binding,
        next: record.next,
        validated: record.validated,
        receipt: record.receipt,
        instructions: ic_cdk::api::performance_counter(0),
    }
}

const fn read_row(row: &[OutputValue]) -> Result<(u64, u64), ImportError> {
    let [id, value] = row else {
        return Err(ImportError::Validation);
    };
    let (PublicValue::Nat64(id), PublicValue::Nat64(value)) = (id.as_public(), value.as_public())
    else {
        return Err(ImportError::Validation);
    };
    Ok((*id, *value))
}

fn installed_identity() -> Result<FleetActivationIdentity, ImportError> {
    FleetActivationApi::status()
        .map(|status| status.identity)
        .map_err(|_| ImportError::Binding)
}
