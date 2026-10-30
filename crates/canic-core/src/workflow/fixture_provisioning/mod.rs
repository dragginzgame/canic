//! Module: workflow::fixture_provisioning
//!
//! Responsibility: deliver one bounded source chunk or synchronous validation step.
//! Does not own: database records, the application cursor, grant mutation or timer policy.
//! Boundary: re-read installed assignment and durable progress after every source await.

pub mod timer;

use crate::{
    dto::{
        error::Error,
        fixture_provisioning::{
            FixtureChunkRead, FixtureImportError, FixtureProvisioningStatus, FixtureStoreError,
        },
    },
    ops::{
        fixture_content,
        fixture_importer::{self, FixtureImporter, ImportLease},
        ic::call::execute as execute_call,
        runtime::{env::EnvOps, fleet_activation::FleetActivationRuntimeOps},
    },
};
use ic_call::{Call, WaitPolicy};

pub fn register(importer: &'static dyn FixtureImporter) -> Result<(), FixtureImportError> {
    fixture_importer::register(importer)?;
    timer::FixtureImportTimer::start().map_err(|error| FixtureImportError::Runtime(error.into()))
}

pub fn status() -> Result<FixtureProvisioningStatus, FixtureImportError> {
    if FleetActivationRuntimeOps::is_standalone_local()
        || EnvOps::is_fleet_coordinator_runtime()
        || EnvOps::canister_role().is_ok_and(|role| role.is_root() || role.is_wasm_store())
    {
        return Ok(FixtureProvisioningStatus::NotRequired);
    }
    let Some(assignment) = fixture_importer::assignment()? else {
        return Ok(FixtureProvisioningStatus::NotRequired);
    };
    if let Some(failure) =
        crate::ops::storage::async_job_recovery::AsyncJobRecoveryOps::fixture_import_failure()
    {
        return Ok(FixtureProvisioningStatus::Failed(failure));
    }
    let Some(importer) = fixture_importer::registered() else {
        return Ok(FixtureProvisioningStatus::AwaitingImporter);
    };
    fixture_importer::status(&assignment, importer)
}

/// Admit application work only after the installed fixture has a validated receipt.
pub fn require_ready() -> Result<(), crate::InternalError> {
    if is_ready(&status()) {
        Ok(())
    } else {
        Err(crate::InternalError::unavailable())
    }
}

/// A missing, failed or invalid observation cannot satisfy data readiness.
#[must_use]
pub const fn is_ready(observed: &Result<FixtureProvisioningStatus, FixtureImportError>) -> bool {
    matches!(
        observed,
        Ok(FixtureProvisioningStatus::NotRequired | FixtureProvisioningStatus::Complete(_))
    )
}

/// Advance one timer-owned attempt; no application can bypass its recovery fence.
pub(super) async fn advance_owned(
    attempt: crate::ops::storage::async_job_recovery::AsyncJobAttempt,
) -> Result<FixtureProvisioningStatus, FixtureImportError> {
    let Some(assignment) = fixture_importer::assignment()? else {
        return Ok(FixtureProvisioningStatus::NotRequired);
    };
    fixture_importer::require_active()?;
    let importer = fixture_importer::registered().ok_or(FixtureImportError::ImporterMissing)?;
    let lease = ImportLease::acquire(&assignment)?;
    let Some(before) = fixture_importer::progress(importer, &assignment)? else {
        fixture_importer::begin(importer, &assignment);
        return fixture_importer::status(&assignment, importer);
    };
    if before.receipt.is_some() {
        return fixture_importer::status(&assignment, importer);
    }
    if before.next_chunk as usize == assignment.descriptor.chunks.len() {
        fixture_importer::validate_step(importer, &assignment);
        return fixture_importer::status(&assignment, importer);
    }
    let request = canic_contracts::dto::fixture_bytes::ReadRequest::from(FixtureChunkRead {
        grant: assignment.grant.clone(),
        index: before.next_chunk,
    });
    let payload = ic_wire::native::encode_request(
        crate::protocol::fixture_bytes::read::METHOD_ID,
        &request,
        crate::protocol::fixture_bytes::READ_REQUEST_LIMITS,
    )
    .map_err(|_| {
        FixtureImportError::Codec(Error::from_registered(
            crate::diagnostics::codes::CODEC_FAILED,
        ))
    })?;
    let reply = execute_call(
        Call::new(
            assignment.store.into(),
            crate::protocol::CANIC_WASM_STORE_FIXTURE_CHUNK,
            WaitPolicy::Bounded {
                timeout_seconds: 300,
            },
        )
        .take_raw_args(payload),
    )
    .await
    .map_err(|error| FixtureImportError::Transport(error.into()))?
    .into_bytes();
    let reply: Result<
        Result<canic_contracts::dto::fixture_bytes::ChunkResponse, FixtureStoreError>,
        Error,
    > = ic_wire::native::decode_response(
        crate::protocol::fixture_bytes::read::METHOD_ID,
        &reply,
        crate::protocol::fixture_bytes::CHUNK_LIMITS,
    )
    .map_err(|_| {
        FixtureImportError::Codec(Error::from_registered(
            crate::diagnostics::codes::CODEC_INVALID,
        ))
    })?;
    let bytes = reply
        .map_err(FixtureImportError::SourceRejected)?
        .map_err(FixtureImportError::Source)?
        .bytes;
    lease.require_current()?;
    if !crate::ops::storage::async_job_recovery::AsyncJobRecoveryOps::is_current(attempt) {
        return Err(FixtureImportError::Authority);
    }
    fixture_importer::require_active()?;
    if fixture_importer::assignment()?.as_deref() != Some(assignment.as_ref()) {
        return Err(FixtureImportError::Authority);
    }
    if fixture_importer::progress(importer, &assignment)?.as_ref() != Some(&before) {
        return Err(FixtureImportError::Progress);
    }
    fixture_content::verify_chunk(&assignment.descriptor, before.next_chunk, &bytes)
        .map_err(FixtureImportError::Source)?;
    fixture_importer::apply_chunk(importer, &assignment, before.next_chunk, &bytes);
    fixture_importer::status(&assignment, importer)
}
