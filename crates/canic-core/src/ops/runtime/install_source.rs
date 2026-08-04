//! Module: ops::runtime::install_source
//!
//! Responsibility: resolve approved wasm module sources for install workflows.
//! Does not own: control-plane publication, wasm-store storage, or install execution.
//! Boundary: delegates to the registered resolver and returns Store-backed chunk sources.

use crate::{
    InternalError, InternalErrorOrigin,
    cdk::types::Principal,
    domain::metrics::{
        WasmStoreMetricOperation, WasmStoreMetricOutcome, WasmStoreMetricReason,
        WasmStoreMetricSource,
    },
    format::byte_size,
    ids::CanisterRole,
    ops::runtime::metrics::wasm_store::WasmStoreMetrics,
};
use async_trait::async_trait;
use std::sync::OnceLock;

///
/// ApprovedModuleSource
///
/// Approved install source metadata and payload for one canister role.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApprovedModuleSource {
    source_canister: Principal,
    source_label: String,
    module_hash: Vec<u8>,
    chunk_hashes: Vec<Vec<u8>>,
    payload_size_bytes: u64,
}

impl ApprovedModuleSource {
    /// Construct one chunk-store-backed module source.
    #[must_use]
    pub const fn chunked(
        source_canister: Principal,
        source_label: String,
        module_hash: Vec<u8>,
        chunk_hashes: Vec<Vec<u8>>,
        payload_size_bytes: u64,
    ) -> Self {
        Self {
            source_canister,
            source_label,
            module_hash,
            chunk_hashes,
            payload_size_bytes,
        }
    }

    /// Return the Store canister that owns the approved chunk set.
    #[must_use]
    pub const fn source_canister(&self) -> &Principal {
        &self.source_canister
    }

    /// Return the logical source label used for logs and status output.
    #[must_use]
    pub fn source_label(&self) -> &str {
        &self.source_label
    }

    /// Return the installable wasm module hash.
    #[must_use]
    pub fn module_hash(&self) -> &[u8] {
        &self.module_hash
    }

    /// Return the formatted module payload size for logs and status output.
    #[must_use]
    pub fn payload_size(&self) -> String {
        byte_size(self.payload_size_bytes)
    }

    /// Return the raw payload size in bytes.
    #[must_use]
    pub const fn payload_size_bytes(&self) -> u64 {
        self.payload_size_bytes
    }

    /// Return the approved chunk hashes in deterministic install order.
    #[must_use]
    pub fn chunk_hashes(&self) -> &[Vec<u8>] {
        &self.chunk_hashes
    }

    /// Return the approved chunk count.
    #[must_use]
    pub const fn chunk_count(&self) -> usize {
        self.chunk_hashes.len()
    }
}

///
/// ModuleSourceResolver
///
/// Driver interface for resolving approved install sources outside the runtime.
///

#[async_trait]
pub trait ModuleSourceResolver: Send + Sync {
    /// Resolve the currently approved install source for one canister role.
    async fn approved_module_source(
        &self,
        role: &CanisterRole,
    ) -> Result<ApprovedModuleSource, InternalError>;
}

static MODULE_SOURCE_RESOLVER: OnceLock<&'static dyn ModuleSourceResolver> = OnceLock::new();

///
/// ModuleSourceRuntimeApi
///
/// Process-local registry and resolver facade for approved module sources.
///

pub struct ModuleSourceRuntimeApi;

impl ModuleSourceRuntimeApi {
    /// Register the control-plane resolver used by root-owned installation flows.
    pub fn register_module_source_resolver(resolver: &'static dyn ModuleSourceResolver) {
        let _ = MODULE_SOURCE_RESOLVER.set(resolver);
    }

    /// Resolve the approved install source for one canister role through the registered driver.
    pub(crate) async fn approved_module_source(
        role: &CanisterRole,
    ) -> Result<ApprovedModuleSource, InternalError> {
        let resolver = MODULE_SOURCE_RESOLVER.get().ok_or_else(|| {
            WasmStoreMetrics::record(
                WasmStoreMetricOperation::SourceResolve,
                WasmStoreMetricSource::Resolver,
                WasmStoreMetricOutcome::Failed,
                WasmStoreMetricReason::InvalidState,
            );
            InternalError::workflow(
                InternalErrorOrigin::Workflow,
                "module source resolver is not registered; root/control-plane install flows are unavailable".to_string(),
            )
        })?;

        match resolver.approved_module_source(role).await {
            Ok(source) => {
                WasmStoreMetrics::record(
                    WasmStoreMetricOperation::SourceResolve,
                    WasmStoreMetricSource::Resolver,
                    WasmStoreMetricOutcome::Completed,
                    WasmStoreMetricReason::Ok,
                );
                Ok(source)
            }
            Err(err) => {
                WasmStoreMetrics::record(
                    WasmStoreMetricOperation::SourceResolve,
                    WasmStoreMetricSource::Resolver,
                    WasmStoreMetricOutcome::Failed,
                    WasmStoreMetricReason::StoreCall,
                );
                Err(err)
            }
        }
    }
}
