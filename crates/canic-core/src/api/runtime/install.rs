use crate::{
    InternalError, InternalErrorOrigin, cdk::types::Principal, dto::error::Error,
    format::byte_size, ids::CanisterRole,
};
use async_trait::async_trait;
use std::sync::OnceLock;

///
/// ApprovedModuleSource
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApprovedModuleSource {
    pub source_canister: Principal,
    pub source_label: String,
    pub module_hash: Vec<u8>,
    pub chunk_hashes: Vec<Vec<u8>>,
    pub payload_size_bytes: u64,
}

impl ApprovedModuleSource {
    /// Return the formatted module payload size for logs and status output.
    #[must_use]
    pub fn payload_size(&self) -> String {
        byte_size(self.payload_size_bytes)
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

///
/// ModuleSourceRuntimeApi
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
            InternalError::workflow(
                InternalErrorOrigin::Workflow,
                "module source resolver is not registered; root/control-plane install flows are unavailable".to_string(),
            )
        })?;

        resolver
            .approved_module_source(role)
            .await
            .map_err(InternalError::public)
    }
}
