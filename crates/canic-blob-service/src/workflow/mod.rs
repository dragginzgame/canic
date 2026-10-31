//! Observe completed RPC results through Canic's sole metrics owner.
use canic::api::metrics::providers::MetricProviderApi;
use ic_blob_storage_contracts::dto::{
    reference::{ReferenceFailure, ReferenceMutationResponse},
    upload::admission::{UploadAdmissionFailure, UploadAdmissionMutation},
};

/// Observe an admitted RPC without inferring another logical operation on replay.
pub fn admission(
    result: &Result<UploadAdmissionMutation, UploadAdmissionFailure>,
    instructions: u64,
    bytes: u32,
) {
    let outcome = match result {
        Ok(value) if value.replayed => "admission.replayed",
        Ok(_) => "admission.applied",
        Err(_) => "admission.refused",
    };
    record(
        "admission.attempts",
        "admission.instructions",
        "admission.request_bytes",
        outcome,
        instructions,
        bytes,
    );
}
/// A replayed failed receipt remains a replay; fresh domain refusal is not applied.
pub fn reference(
    result: &Result<ReferenceMutationResponse, ReferenceFailure>,
    instructions: u64,
    bytes: u32,
) {
    let outcome = match result {
        Ok(value) if value.replayed => "reference.replayed",
        Ok(value) if value.receipt.result.is_ok() => "reference.applied",
        _ => "reference.refused",
    };
    record(
        "reference.attempts",
        "reference.instructions",
        "reference.request_bytes",
        outcome,
        instructions,
        bytes,
    );
}
fn record(
    attempts: &str,
    instructions_key: &str,
    bytes_key: &str,
    outcome: &str,
    instructions: u64,
    bytes: u32,
) {
    for (name, amount) in [(attempts, 1), (outcome, 1), (bytes_key, u128::from(bytes))] {
        // A typed diagnostic storage refusal does not reinterpret the RPC result.
        // The shared owner marks its projection unavailable instead of reporting
        // a silently incomplete count. IC traps retain their normal rollback.
        if MetricProviderApi::add("blob", name, amount).is_err() {
            return;
        }
    }
    let _ = MetricProviderApi::record("blob", instructions_key, instructions);
}
