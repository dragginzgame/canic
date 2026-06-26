use crate::{
    dto::template::{
        TemplateChunkResponse, TemplateChunkSetInfoResponse, TemplateChunkSetPrepareInput,
        TemplateManifestInput, WasmStoreCatalogEntryResponse, WasmStoreStatusResponse,
    },
    ids::{TemplateId, TemplateVersion},
};
use candid::{CandidType, utils::ArgumentEncoder};
use canic_core::{
    control_plane_support::{
        cdk::types::Principal, error::InternalError, ops::ic::call::CallOps, protocol,
    },
    dto::error::Error,
};

///
/// WasmStoreInternalClient
///
pub(in crate::workflow::runtime::template) struct WasmStoreInternalClient {
    store_pid: Principal,
}

impl WasmStoreInternalClient {
    const BEGIN_GC: WasmStoreEndpoint =
        WasmStoreEndpoint::root_update(protocol::CANIC_WASM_STORE_BEGIN_GC);
    const CATALOG: WasmStoreEndpoint =
        WasmStoreEndpoint::structural_query(protocol::CANIC_WASM_STORE_CATALOG);
    const CHUNK: WasmStoreEndpoint =
        WasmStoreEndpoint::root_update(protocol::CANIC_WASM_STORE_CHUNK);
    const COMPLETE_GC: WasmStoreEndpoint =
        WasmStoreEndpoint::root_update(protocol::CANIC_WASM_STORE_COMPLETE_GC);
    const INFO: WasmStoreEndpoint = WasmStoreEndpoint::root_update(protocol::CANIC_WASM_STORE_INFO);
    const PREPARE: WasmStoreEndpoint =
        WasmStoreEndpoint::root_update(protocol::CANIC_WASM_STORE_PREPARE);
    const PREPARE_GC: WasmStoreEndpoint =
        WasmStoreEndpoint::root_update(protocol::CANIC_WASM_STORE_PREPARE_GC);
    const PUBLISH_CHUNK: WasmStoreEndpoint =
        WasmStoreEndpoint::root_update(protocol::CANIC_WASM_STORE_PUBLISH_CHUNK);
    const STAGE_MANIFEST: WasmStoreEndpoint =
        WasmStoreEndpoint::root_update(protocol::CANIC_WASM_STORE_STAGE_MANIFEST);
    const STATUS: WasmStoreEndpoint =
        WasmStoreEndpoint::structural_query(protocol::CANIC_WASM_STORE_STATUS);
    #[cfg(test)]
    const ENDPOINTS: &[WasmStoreEndpoint] = &[
        Self::BEGIN_GC,
        Self::CATALOG,
        Self::CHUNK,
        Self::COMPLETE_GC,
        Self::INFO,
        Self::PREPARE,
        Self::PREPARE_GC,
        Self::PUBLISH_CHUNK,
        Self::STAGE_MANIFEST,
        Self::STATUS,
    ];

    pub(super) const fn new(store_pid: Principal) -> Self {
        Self { store_pid }
    }

    pub(super) async fn catalog(
        &self,
    ) -> Result<Vec<WasmStoreCatalogEntryResponse>, InternalError> {
        self.call_result(Self::CATALOG, ()).await
    }

    pub(super) async fn info(
        &self,
        template_id: &TemplateId,
        version: &TemplateVersion,
    ) -> Result<TemplateChunkSetInfoResponse, InternalError> {
        self.call_result(
            Self::INFO,
            (
                template_id.as_str().to_string(),
                version.as_str().to_string(),
            ),
        )
        .await
    }

    pub(super) async fn status(&self) -> Result<WasmStoreStatusResponse, InternalError> {
        self.call_result(Self::STATUS, ()).await
    }

    pub(super) async fn prepare_chunk_set(
        &self,
        request: TemplateChunkSetPrepareInput,
    ) -> Result<TemplateChunkSetInfoResponse, InternalError> {
        self.call_result(Self::PREPARE, (request,)).await
    }

    pub(super) async fn stage_manifest(
        &self,
        request: TemplateManifestInput,
    ) -> Result<(), InternalError> {
        self.call_result(Self::STAGE_MANIFEST, (request,)).await
    }

    pub(super) async fn publish_chunk(
        &self,
        template_id: &TemplateId,
        version: &TemplateVersion,
        chunk_index: u32,
        bytes: &[u8],
    ) -> Result<(), InternalError> {
        self.call_result(
            Self::PUBLISH_CHUNK,
            (TemplateChunkInputRef {
                template_id,
                version,
                chunk_index,
                bytes,
            },),
        )
        .await
    }

    pub(super) async fn prepare_gc(&self) -> Result<(), InternalError> {
        self.call_result(Self::PREPARE_GC, ()).await
    }

    pub(super) async fn begin_gc(&self) -> Result<(), InternalError> {
        self.call_result(Self::BEGIN_GC, ()).await
    }

    pub(super) async fn complete_gc(&self) -> Result<(), InternalError> {
        self.call_result(Self::COMPLETE_GC, ()).await
    }

    pub(super) async fn chunk(
        &self,
        template_id: &TemplateId,
        version: &TemplateVersion,
        chunk_index: u32,
    ) -> Result<Vec<u8>, InternalError> {
        let response: TemplateChunkResponse = self
            .call_result(
                Self::CHUNK,
                (
                    template_id.as_str().to_string(),
                    version.as_str().to_string(),
                    chunk_index,
                ),
            )
            .await?;

        Ok(response.bytes)
    }

    #[cfg(test)]
    pub(super) fn method_requires_internal_proof(method: &str) -> bool {
        Self::ENDPOINTS
            .iter()
            .find(|endpoint| endpoint.method == method)
            .is_some_and(WasmStoreEndpoint::requires_internal_proof)
    }

    async fn call_result<T, A>(
        &self,
        endpoint: WasmStoreEndpoint,
        arg: A,
    ) -> Result<T, InternalError>
    where
        T: CandidType + serde::de::DeserializeOwned,
        A: ArgumentEncoder,
    {
        debug_assert!(!endpoint.requires_internal_proof());
        let call = CallOps::bounded_wait(self.store_pid, endpoint.method)
            .with_args(arg)?
            .execute()
            .await?;
        let call_res: Result<T, Error> = call.candid::<Result<T, Error>>()?;

        call_res.map_err(InternalError::public)
    }
}

///
/// WasmStoreEndpoint
///
#[derive(Clone, Copy)]
struct WasmStoreEndpoint {
    method: &'static str,
}

impl WasmStoreEndpoint {
    const fn root_update(method: &'static str) -> Self {
        Self { method }
    }

    const fn structural_query(method: &'static str) -> Self {
        Self { method }
    }

    const fn requires_internal_proof(&self) -> bool {
        let _ = self;
        false
    }
}

// Borrowed chunk publish input for store-side chunk staging.
#[derive(CandidType)]
struct TemplateChunkInputRef<'a> {
    pub template_id: &'a TemplateId,
    pub version: &'a TemplateVersion,
    pub chunk_index: u32,
    pub bytes: &'a [u8],
}

#[cfg(test)]
mod tests {
    use super::WasmStoreInternalClient;
    use canic_core::control_plane_support::protocol;
    use std::collections::BTreeSet;

    #[test]
    fn typed_client_endpoint_table_matches_protocol_manifests() {
        let root_updates = WasmStoreInternalClient::ENDPOINTS
            .iter()
            .filter(|endpoint| {
                protocol::CANIC_WASM_STORE_ROOT_UPDATE_METHODS.contains(&endpoint.method)
            })
            .map(|endpoint| endpoint.method)
            .collect::<BTreeSet<_>>();
        let structural = WasmStoreInternalClient::ENDPOINTS
            .iter()
            .filter(|endpoint| {
                protocol::CANIC_WASM_STORE_STRUCTURAL_QUERY_METHODS.contains(&endpoint.method)
            })
            .map(|endpoint| endpoint.method)
            .collect::<BTreeSet<_>>();
        let all = WasmStoreInternalClient::ENDPOINTS
            .iter()
            .map(|endpoint| endpoint.method)
            .collect::<BTreeSet<_>>();

        assert_eq!(
            root_updates,
            protocol::CANIC_WASM_STORE_ROOT_UPDATE_METHODS
                .iter()
                .copied()
                .collect::<BTreeSet<_>>()
        );
        assert_eq!(
            structural,
            protocol::CANIC_WASM_STORE_STRUCTURAL_QUERY_METHODS
                .iter()
                .copied()
                .collect::<BTreeSet<_>>()
        );
        assert_eq!(
            all.len(),
            WasmStoreInternalClient::ENDPOINTS.len(),
            "typed wasm-store client endpoint methods must be unique"
        );
    }

    #[test]
    fn root_update_endpoint_table_does_not_require_internal_proofs() {
        for endpoint in WasmStoreInternalClient::ENDPOINTS
            .iter()
            .filter(|endpoint| {
                protocol::CANIC_WASM_STORE_ROOT_UPDATE_METHODS.contains(&endpoint.method)
            })
        {
            assert!(!endpoint.requires_internal_proof());
        }
    }
}
