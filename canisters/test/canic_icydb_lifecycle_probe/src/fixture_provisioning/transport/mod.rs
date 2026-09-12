//! Bounded transport race probe for the application commit boundary.
//!
//! The source is a controllable test peer, not the retained production Store.
//! Its consensus-round barrier lets PocketIC interleave messages before a reply.

use crate::fixture_provisioning::{
    CHECKPOINT, CommitFault, ImportBinding, ImportError, ImportSnapshot, bound, commit, snapshot,
};
use std::cell::RefCell;

use candid::{CandidType, Deserialize, Principal};
use canic::dto::fixture_provisioning::{FixtureChunkRead, FixtureStoreError};
use canic::{Error, prelude::*};

// A real inter-canister caller for the retained Store endpoint qualification.
#[canic_update(requires(caller::is_controller()))]
async fn fixture_read_retained_source(
    source: Principal,
    request: FixtureChunkRead,
) -> Result<Result<Vec<u8>, FixtureStoreError>, Error> {
    Call::bounded_wait(source, canic::protocol::CANIC_WASM_STORE_FIXTURE_CHUNK)
        .with_arg(request)?
        .execute_candid::<Result<Result<Vec<u8>, FixtureStoreError>, Error>>()
        .await?
}

// Test scheduling envelope only; production delivery must use bounded backoff.
const MAX_BARRIER_ROUNDS: usize = 64;

thread_local! {
    static MALFORMED_READS: RefCell<u64> = const { RefCell::new(0) };
    static SOURCE: RefCell<Option<SourceRecord>> = const { RefCell::new(None) };
    static FETCH: RefCell<Option<ImportBinding>> = const { RefCell::new(None) };
}

/// One exact target grant with a controllable reply and observable read count.
struct SourceRecord {
    binding: ImportBinding,
    bytes: Vec<u8>,
    mode: ReplyMode,
    reads: u64,
    released: bool,
}

/// Test-only source behavior, selected by its controller.
#[derive(CandidType, Clone, Copy, Deserialize)]
enum ReplyMode {
    Held,
    Immediate,
    Reject,
}

/// A heap lease prevents concurrent pulls without duplicating the durable cursor.
struct FetchLease(ImportBinding);

impl FetchLease {
    fn acquire(binding: &ImportBinding) -> Result<Self, ImportError> {
        FETCH.with_borrow_mut(|active| {
            if active.is_some() {
                return Err(ImportError::Busy);
            }
            *active = Some(binding.clone());
            Ok(Self(binding.clone()))
        })
    }
}

impl Drop for FetchLease {
    fn drop(&mut self) {
        FETCH.with_borrow_mut(|active| {
            if active.as_ref() == Some(&self.0) {
                *active = None;
            }
        });
    }
}

#[canic_update(requires(caller::is_controller()))]
async fn fixture_pull(
    binding: ImportBinding,
) -> Result<Result<ImportSnapshot, ImportError>, Error> {
    Ok(pull(&binding, CommitFault::None).await)
}

#[canic_update(requires(caller::is_controller()))]
async fn fixture_pull_with_fault(
    binding: ImportBinding,
    fault: CommitFault,
) -> Result<Result<ImportSnapshot, ImportError>, Error> {
    Ok(pull(&binding, fault).await)
}

async fn pull(binding: &ImportBinding, fault: CommitFault) -> Result<ImportSnapshot, ImportError> {
    let selected = bound(binding)?;
    if selected.next == selected.chunks.len() as u64 {
        return Ok(snapshot(selected));
    }
    let source = selected.source.ok_or(ImportError::Source)?;
    let _lease = FetchLease::acquire(binding)?;
    let response = Call::bounded_wait(source, "fixture_source_read")
        .with_arg(binding.clone())
        .map_err(|_| ImportError::Source)?
        .execute_candid::<Result<Result<Vec<u8>, ImportError>, Error>>()
        .await
        .map_err(|_| ImportError::Source)?
        .map_err(|_| ImportError::Source)??;
    // No database request or borrowed state survives the await. The commit
    // independently reloads exact installation/content authority before writes.
    let current = bound(binding)?;
    if current.source != Some(source) {
        return Err(ImportError::Binding);
    }
    icydb::db::with_request_execution(|| commit(binding, selected.next, &response, fault))
}

#[canic_query(requires(caller::is_controller()))]
async fn fixture_application_ready(
    binding: ImportBinding,
) -> Result<Result<ImportBinding, ImportError>, Error> {
    Ok(bound(&binding).and_then(|record| {
        record
            .receipt
            .filter(|receipt| *receipt == binding)
            .ok_or(ImportError::NotReady)
    }))
}

// Only the test controller can invalidate an empty import while a reply is held.
// This is fault injection, not a product reset or readiness override.
#[canic_update(requires(caller::is_controller()))]
async fn fixture_invalidate_empty_import(
    binding: ImportBinding,
) -> Result<Result<(), ImportError>, Error> {
    Ok(bound(&binding).and_then(|record| {
        if record.next != 0 || record.validated != 0 || record.receipt.is_some() {
            return Err(ImportError::Conflict);
        }
        CHECKPOINT.with_borrow_mut(|cell| cell.set(Vec::new()));
        Ok(())
    }))
}

#[canic_update(requires(caller::is_controller()))]
async fn fixture_source_prepare(
    binding: ImportBinding,
    bytes: Vec<u8>,
    mode: ReplyMode,
) -> Result<Result<(), ImportError>, Error> {
    if bytes.len() != 16 {
        return Ok(Err(ImportError::Bounds));
    }
    SOURCE.with_borrow_mut(|source| {
        *source = Some(SourceRecord {
            binding,
            bytes,
            mode,
            reads: 0,
            released: false,
        });
    });
    Ok(Ok(()))
}

#[canic_update(public)]
async fn fixture_source_read(
    binding: ImportBinding,
) -> Result<Result<Vec<u8>, ImportError>, Error> {
    // Capture authenticated request authority before yielding to another message.
    let captured = SOURCE.with_borrow_mut(|source| {
        let record = source.as_mut().ok_or(ImportError::Source)?;
        if ic_cdk::api::msg_caller() != record.binding.target || binding != record.binding {
            return Err(ImportError::Binding);
        }
        record.reads += 1;
        Ok((record.bytes.clone(), record.mode))
    });
    let (bytes, mode) = match captured {
        Ok(captured) => captured,
        Err(error) => return Ok(Err(error)),
    };
    match mode {
        ReplyMode::Immediate => return Ok(Ok(bytes)),
        ReplyMode::Reject => return Ok(Err(ImportError::Source)),
        ReplyMode::Held => {}
    }
    for _ in 0..MAX_BARRIER_ROUNDS {
        // raw_rand waits for a consensus round. Self-calls can drain within one
        // round and therefore cannot establish an observable paused response.
        Call::bounded_wait(Principal::management_canister(), "raw_rand")
            .with_args(())?
            .execute_candid::<Vec<u8>>()
            .await?;
        let released = SOURCE.with_borrow(|source| {
            source
                .as_ref()
                .is_some_and(|record| record.binding == binding && record.released)
        });
        if released {
            return Ok(Ok(bytes));
        }
    }
    Ok(Err(ImportError::Source))
}

#[canic_update(requires(caller::is_controller()))]
async fn fixture_source_release() -> Result<(), Error> {
    SOURCE.with_borrow_mut(|source| {
        if let Some(record) = source {
            record.released = true;
        }
    });
    Ok(())
}

#[canic_query(requires(caller::is_controller()))]
async fn fixture_source_reads() -> Result<u64, Error> {
    Ok(SOURCE.with_borrow(|source| source.as_ref().map_or(0, |record| record.reads)))
}

/// Deliberately incompatible peer for the automatic consumer's codec failure proof.
/// This raw test endpoint remains reachable while the source probe is Prepared.
#[ic_cdk::update]
fn canic_wasm_store_fixture_chunk(_: FixtureChunkRead) -> u8 {
    MALFORMED_READS.with_borrow_mut(|reads| *reads += 1);
    0
}

/// Observe whether a permanent malformed reply was retried after target restart.
#[ic_cdk::query(guard = "crate::require_test_controller")]
fn fixture_malformed_reads() -> u64 {
    MALFORMED_READS.with_borrow(|reads| *reads)
}
