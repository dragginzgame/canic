use crate::{
    InternalError, InternalErrorOrigin,
    cdk::{types::Principal, utils::hash::wasm_hash},
    dto::error::Error,
    format::byte_size,
    ids::CanisterRole,
    ops::runtime::metrics::wasm_store::{
        WasmStoreMetricOperation, WasmStoreMetricOutcome, WasmStoreMetricReason,
        WasmStoreMetricSource, WasmStoreMetrics,
    },
};
use async_trait::async_trait;
use std::{
    borrow::Cow,
    collections::BTreeMap,
    sync::{Mutex, OnceLock},
};

///
/// ApprovedModulePayload
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ApprovedModulePayload {
    Chunked {
        source_canister: Principal,
        chunk_hashes: Vec<Vec<u8>>,
    },
    Embedded {
        wasm_module: Cow<'static, [u8]>,
    },
}

///
/// ApprovedModuleSource
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApprovedModuleSource {
    source_label: String,
    module_hash: Vec<u8>,
    payload_size_bytes: u64,
    payload: ApprovedModulePayload,
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
            source_label,
            module_hash,
            payload_size_bytes,
            payload: ApprovedModulePayload::Chunked {
                source_canister,
                chunk_hashes,
            },
        }
    }

    /// Construct one embedded module source from already packaged wasm bytes.
    #[must_use]
    pub fn embedded(source_label: String, wasm_module: &'static [u8]) -> Self {
        let payload_size_bytes = wasm_module.len() as u64;

        Self {
            source_label,
            module_hash: wasm_hash(wasm_module),
            payload_size_bytes,
            payload: ApprovedModulePayload::Embedded {
                wasm_module: Cow::Borrowed(wasm_module),
            },
        }
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

    /// Return the chunk count when the source is chunk-store-backed.
    #[must_use]
    pub const fn chunk_count(&self) -> usize {
        match &self.payload {
            ApprovedModulePayload::Chunked { chunk_hashes, .. } => chunk_hashes.len(),
            ApprovedModulePayload::Embedded { .. } => 0,
        }
    }

    /// Return the underlying payload representation.
    #[must_use]
    pub const fn payload(&self) -> &ApprovedModulePayload {
        &self.payload
    }
}

///
/// ModuleSourceResolver
///

#[async_trait]
pub trait ModuleSourceResolver: Send + Sync {
    /// Resolve the currently approved install source for one canister role.
    async fn approved_module_source(
        &self,
        role: &CanisterRole,
    ) -> Result<ApprovedModuleSource, Error>;
}

static MODULE_SOURCE_RESOLVER: OnceLock<&'static dyn ModuleSourceResolver> = OnceLock::new();
static EMBEDDED_MODULE_SOURCES: OnceLock<Mutex<BTreeMap<CanisterRole, ApprovedModuleSource>>> =
    OnceLock::new();

///
/// ModuleSourceRuntimeApi
///

pub struct ModuleSourceRuntimeApi;

impl ModuleSourceRuntimeApi {
    /// Register one built-in module source override for the current process.
    pub fn register_embedded_module_source(role: CanisterRole, source: ApprovedModuleSource) {
        let sources = EMBEDDED_MODULE_SOURCES.get_or_init(|| Mutex::new(BTreeMap::new()));
        let mut sources = sources
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        match sources.get(&role) {
            Some(existing) if existing == &source => {}
            Some(existing) => {
                panic!(
                    "embedded module source for role '{role}' was already registered with a different payload: existing='{}' new='{}'",
                    existing.source_label(),
                    source.source_label()
                );
            }
            None => {
                sources.insert(role, source);
            }
        }
    }

    /// Register one embedded wasm payload as the built-in install source for one role.
    pub fn register_embedded_module_wasm(
        role: CanisterRole,
        source_label: impl Into<String>,
        wasm_module: &'static [u8],
    ) {
        Self::register_embedded_module_source(
            role,
            ApprovedModuleSource::embedded(source_label.into(), wasm_module),
        );
    }

    /// Register the control-plane resolver used by root-owned installation flows.
    pub fn register_module_source_resolver(resolver: &'static dyn ModuleSourceResolver) {
        let _ = MODULE_SOURCE_RESOLVER.set(resolver);
    }

    /// Return whether one embedded module source override has been registered.
    #[must_use]
    pub fn has_embedded_module_source(role: &CanisterRole) -> bool {
        EMBEDDED_MODULE_SOURCES.get().is_some_and(|sources| {
            let sources = sources
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            sources.contains_key(role)
        })
    }

    /// Resolve the approved install source for one canister role through the registered driver.
    pub(crate) async fn approved_module_source(
        role: &CanisterRole,
    ) -> Result<ApprovedModuleSource, InternalError> {
        if let Some(source) = EMBEDDED_MODULE_SOURCES.get().and_then(|sources| {
            let sources = sources
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            sources.get(role).cloned()
        }) {
            WasmStoreMetrics::record(
                WasmStoreMetricOperation::SourceResolve,
                WasmStoreMetricSource::Embedded,
                WasmStoreMetricOutcome::Completed,
                WasmStoreMetricReason::Ok,
            );
            return Ok(source);
        }

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
                Err(InternalError::public(err))
            }
        }
    }
}
