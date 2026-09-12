//! Published-IcyDB participant for the production Canic fixture consumer.
//!
//! This probe owns its rows and checkpoint; controller fault selection is test-only.

use crate::fixture_provisioning::{
    CommitFault, ImportBinding, ImportError, begin_selected, bound, commit, load, persist, validate,
};
use candid::{CandidType, Deserialize};
use canic::{
    Error,
    api::fixture_provisioning::{FixtureImporter, FixtureProvisioningApi},
    dto::fixture_provisioning::{
        FixtureAssignment, FixtureImportError, FixtureImportProgress, FixtureImportReceipt,
        FixtureProvisioningStatus,
    },
};
use sha2::{Digest, Sha256};
use std::cell::Cell;

/// Faults deliberately violate the callback contract so Canic must roll back the message.
#[derive(CandidType, Clone, Copy, Deserialize, Eq, PartialEq)]
enum ConsumerFault {
    None,
    ErrorAfterRows,
    SkipCheckpoint,
    BadReceipt,
    PauseAfterFirstChunk,
    PauseBeforeFetch,
    WrongAuthority,
}

thread_local! {
    static FAULT: Cell<ConsumerFault> = const { Cell::new(ConsumerFault::None) };
}

struct ProbeImporter;
static IMPORTER: ProbeImporter = ProbeImporter;

pub fn register() {
    FixtureProvisioningApi::register(&IMPORTER).expect("one fixture importer per fresh heap");
}

impl FixtureImporter for ProbeImporter {
    fn progress(
        &self,
        assignment: &FixtureAssignment,
    ) -> Result<Option<FixtureImportProgress>, FixtureImportError> {
        if FAULT.get() == ConsumerFault::PauseBeforeFetch {
            return Err(FixtureImportError::NotReady);
        }
        if FAULT.get() == ConsumerFault::WrongAuthority {
            return Err(FixtureImportError::Authority);
        }
        if !crate::database_ready() {
            return Err(FixtureImportError::NotReady);
        }
        if matches!(load(), Err(ImportError::NotBegun)) {
            return Ok(None);
        }
        let record = bound(&binding(assignment)).map_err(application_error)?;
        if record.source != Some(assignment.store) || record.chunks != digests(assignment) {
            return Err(FixtureImportError::Authority);
        }
        let receipt = record
            .receipt
            .map(|receipt| {
                if receipt != record.binding {
                    return Err(FixtureImportError::Receipt);
                }
                let summary = if FAULT.get() == ConsumerFault::BadReceipt {
                    [0; 32]
                } else {
                    Sha256::digest(record.chunks.iter().flatten().copied().collect::<Vec<_>>())
                        .into()
                };
                Ok(Box::new(FixtureImportReceipt {
                    binding: assignment.grant.binding.clone(),
                    completion_summary: summary,
                }))
            })
            .transpose()?;
        Ok(Some(FixtureImportProgress {
            binding: assignment.grant.binding.clone(),
            next_chunk: u32::try_from(record.next).map_err(|_| FixtureImportError::Progress)?,
            receipt,
        }))
    }

    fn begin(&self, assignment: &FixtureAssignment) -> Result<(), FixtureImportError> {
        if assignment.descriptor.format_hash != [0x34; 32]
            || assignment
                .descriptor
                .chunks
                .iter()
                .any(|chunk| chunk.length != 16)
        {
            return Err(FixtureImportError::Application { code: 1 });
        }
        icydb::db::with_request_execution(|| {
            begin_selected(
                binding(assignment),
                digests(assignment),
                Some(assignment.store),
            )
            .map(|_| ())
            .map_err(application_error)
        })
    }

    fn apply_chunk(
        &self,
        assignment: &FixtureAssignment,
        index: u32,
        bytes: &[u8],
    ) -> Result<(), FixtureImportError> {
        if FAULT.get() == ConsumerFault::PauseAfterFirstChunk && index > 0 {
            return Err(FixtureImportError::NotReady);
        }
        icydb::db::with_request_execution(|| {
            let fault = if FAULT.get() == ConsumerFault::ErrorAfterRows {
                CommitFault::ReturnErrorAfterRows
            } else {
                CommitFault::None
            };
            commit(&binding(assignment), u64::from(index), bytes, fault)
                .map_err(application_error)?;
            if FAULT.get() == ConsumerFault::SkipCheckpoint {
                let mut record = load().map_err(application_error)?;
                record.next += 1;
                persist(&record);
            }
            Ok(())
        })
    }

    fn validate_step(&self, assignment: &FixtureAssignment) -> Result<(), FixtureImportError> {
        icydb::db::with_request_execution(|| {
            validate(&binding(assignment))
                .map(|_| ())
                .map_err(application_error)
        })
    }
}

fn binding(assignment: &FixtureAssignment) -> ImportBinding {
    ImportBinding {
        target: ic_cdk::api::canister_self(),
        installation: assignment.grant.binding.installation,
        content: assignment.grant.binding.content_id,
    }
}

fn digests(assignment: &FixtureAssignment) -> Vec<[u8; 32]> {
    assignment
        .descriptor
        .chunks
        .iter()
        .map(|chunk| chunk.digest)
        .collect()
}

const fn application_error(error: ImportError) -> FixtureImportError {
    let code = match error {
        ImportError::Binding => 2,
        ImportError::Bounds => 3,
        ImportError::Busy => 4,
        ImportError::Conflict => 5,
        ImportError::Database => 6,
        ImportError::Injected => 7,
        ImportError::NotBegun => 8,
        ImportError::NotReady => 9,
        ImportError::Sequence => 10,
        ImportError::Source => 11,
        ImportError::Validation => 12,
    };
    FixtureImportError::Application { code }
}

#[ic_cdk::query(guard = "crate::require_test_controller")]
async fn fixture_consumer_status()
-> Result<Result<FixtureProvisioningStatus, FixtureImportError>, Error> {
    Ok(FixtureProvisioningApi::status())
}

#[ic_cdk::update(guard = "crate::require_test_controller")]
async fn fixture_consumer_fault(fault: ConsumerFault) -> Result<(), Error> {
    FAULT.set(fault);
    Ok(())
}

#[ic_cdk::query(guard = "crate::require_test_controller")]
fn fixture_consumer_fetch_pending() -> bool {
    FixtureProvisioningApi::fetch_in_flight()
}
