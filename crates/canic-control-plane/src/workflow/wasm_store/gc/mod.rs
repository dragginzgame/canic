//! Store-local retirement driven by the existing Root operation and exact GC identity.
//!
//! Preparation fences access; collection resumes bounded fixture cleanup before
//! executable templates. Same-operation retries never imply a stronger intent.

use crate::{
    dto::template::{WasmStoreGcRequest, WasmStoreGcTarget},
    ids::WasmStoreGcMode,
    ops::{
        fixture_store,
        storage::template::{TemplateChunkedOps, WasmStoreGcOps},
    },
};
use canic_contracts::dto::error::Error;
use canic_core::{api::timer::TimerApi, control_plane_support::ops::ic::IcOps};
use ic_timers::{OnceDecision, OnceRunResult, TimerCompletion};
use std::time::Duration;

// Each entry is bounded by fixture admission. Installed qualification covers
// full batches of one-MiB chunks; Root retains continuation between batches.
const FIXTURE_CLEANUP_BATCH_ENTRIES: usize = 16;

/// Authenticated Root commands select one outcome without interpreting replay as escalation.

pub fn request(request: WasmStoreGcRequest) -> Result<(), Error> {
    match request.target {
        WasmStoreGcTarget::Prepared => {
            WasmStoreGcOps::prepare(request.operation_id, IcOps::now_secs())
        }
        WasmStoreGcTarget::Complete => {
            if WasmStoreGcOps::require_collection(request.operation_id)? {
                TimerApi::defer_lifecycle_result_required(
                    Duration::ZERO,
                    "canic:wasm_store:gc",
                    async move {
                        let result = collect(request.operation_id).await;
                        if let Err(error) = &result {
                            IcOps::println(&format!("Store collection failed: {error}"));
                        }
                        collection_completion(result)
                    },
                );
            }
            Ok(())
        }
    }
}

const fn collection_completion(result: Result<(), Error>) -> OnceRunResult {
    let completion = match result {
        Ok(()) => TimerCompletion::success(1),
        // Root owns retries of the exact GC operation; this callback only reports its result.
        Err(_) => TimerCompletion::retryable_failure(0),
    };
    OnceRunResult::new(completion, OnceDecision::Stop)
}

async fn collect(operation_id: [u8; 32]) -> Result<(), Error> {
    if !WasmStoreGcOps::require_collection(operation_id)? {
        return Ok(());
    }
    if WasmStoreGcOps::status().mode == WasmStoreGcMode::Prepared {
        WasmStoreGcOps::begin(IcOps::now_secs())?;
    }
    WasmStoreGcOps::begin_clearing(IcOps::now_secs())?;
    if !clear_fixture_batch()? {
        return Ok(());
    }
    TemplateChunkedOps::execute_local_store_gc()
        .await
        .map_err(Error::from)?;
    WasmStoreGcOps::complete(IcOps::now_secs())
}

fn clear_fixture_batch() -> Result<bool, Error> {
    for _ in 0..FIXTURE_CLEANUP_BATCH_ENTRIES {
        if fixture_store::clear_retired_step()? {
            return Ok(true);
        }
    }
    Ok(false)
}

#[cfg(test)]
mod tests;
