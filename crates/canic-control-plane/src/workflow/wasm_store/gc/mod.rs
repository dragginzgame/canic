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
use canic_core::{api::timer::TimerApi, control_plane_support::ops::ic::IcOps, dto::error::Error};
use std::time::Duration;

/// Authenticated Root commands select one outcome without interpreting replay as escalation.
pub fn request(request: WasmStoreGcRequest) -> Result<(), Error> {
    match request.target {
        WasmStoreGcTarget::Prepared => {
            WasmStoreGcOps::prepare(request.operation_id, IcOps::now_secs())
        }
        WasmStoreGcTarget::Complete => {
            if WasmStoreGcOps::require_collection(request.operation_id)? {
                TimerApi::defer_lifecycle_required(
                    Duration::ZERO,
                    "canic:wasm_store:gc",
                    async move {
                        let _ = collect(request.operation_id).await;
                    },
                );
            }
            Ok(())
        }
    }
}

async fn collect(operation_id: [u8; 32]) -> Result<(), Error> {
    if !WasmStoreGcOps::require_collection(operation_id)? {
        return Ok(());
    }
    if WasmStoreGcOps::status().mode == WasmStoreGcMode::Prepared {
        WasmStoreGcOps::begin(IcOps::now_secs())?;
    }
    WasmStoreGcOps::begin_clearing(IcOps::now_secs())?;
    if !fixture_store::clear_retired_step()? {
        return Ok(());
    }
    TemplateChunkedOps::execute_local_store_gc()
        .await
        .map_err(Error::from)?;
    WasmStoreGcOps::complete(IcOps::now_secs())
}
