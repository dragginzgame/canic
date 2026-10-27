use crate::{
    dto::template::{
        TemplateChunkRequest, TemplateChunkResponse, TemplateChunkSetInfoResponse,
        TemplateChunkSetPrepareInput, TemplateLookupRequest, TemplateManifestInput,
        WasmStoreCatalogEntryResponse, WasmStoreDeletionCycleReclamationRequest,
        WasmStoreDeletionCycleReclamationResponse, WasmStoreGcRequest, WasmStoreGcTarget,
        WasmStoreStatusResponse,
    },
    ids::{TemplateId, TemplateVersion},
};
use canic_contracts::{
    dto::{
        error::Error,
        fixture_provisioning::{
            FixtureDescriptor, FixtureGrant, FixtureGrantRequest, FixtureSourceStatus,
            FixtureStoreError,
        },
    },
    protocol,
};
use canic_core::{
    cdk::types::Principal,
    control_plane_support::{
        error::InternalError,
        ops::{cost_guard::CostGuardPermit, ic::call::CallOps},
    },
};

///
/// WasmStoreInternalClient
///

pub(in crate::workflow) struct WasmStoreInternalClient {
    store_pid: Principal,
}

impl WasmStoreInternalClient {
    const CHUNK: &str = protocol::CANIC_WASM_STORE_CHUNK;
    const PUBLISH_CHUNK: &str = protocol::CANIC_WASM_STORE_PUBLISH_CHUNK;
    #[cfg(test)]
    const ENDPOINTS: &[&str] = &[
        protocol::store_control::grant::METHOD,
        protocol::store_control::reclaim::METHOD,
        Self::CHUNK,
        Self::PUBLISH_CHUNK,
        protocol::store_catalog::fixture::METHOD,
        protocol::store_catalog::grant::METHOD,
        protocol::store_gc::METHOD,
        protocol::store_publication::fixture_prepare::METHOD,
        protocol::store_publication::inspect::METHOD,
        protocol::store_publication::prepare::METHOD,
        protocol::store_publication::stage::METHOD,
        protocol::store_inventory::catalog::METHOD,
        protocol::store_inventory::storage::METHOD,
    ];

    pub(in crate::workflow) const fn new(store_pid: Principal) -> Self {
        Self { store_pid }
    }

    pub(super) async fn catalog(
        &self,
    ) -> Result<Vec<WasmStoreCatalogEntryResponse>, InternalError> {
        let result = async {
            let bytes = ic_wire::native::encode_request(
                protocol::store_inventory::catalog::METHOD_ID,
                &(),
                protocol::store_inventory::REQUEST_LIMITS,
            )
            .map_err(|_| InternalError::invalid_input())?;
            let reply =
                CallOps::bounded_wait(self.store_pid, protocol::store_inventory::catalog::METHOD)
                    .with_raw_args(bytes)
                    .execute()
                    .await?
                    .into_bytes();
            let reply: Result<Vec<WasmStoreCatalogEntryResponse>, Error> =
                ic_wire::native::decode_response(
                    protocol::store_inventory::catalog::METHOD_ID,
                    &reply,
                    protocol::store_inventory::RESPONSE_LIMITS,
                )
                .map_err(|_| InternalError::invalid_input())?;
            reply.map_err(InternalError::observed_public)
        }
        .await;
        result.map_err(|error| {
            error.with_dependency_failure(
                self.store_pid,
                protocol::store_inventory::catalog::METHOD,
                "catalog",
            )
        })
    }

    pub(super) async fn info(
        &self,
        template_id: &TemplateId,
        version: &TemplateVersion,
    ) -> Result<TemplateChunkSetInfoResponse, InternalError> {
        let result = async {
            let bytes = ic_wire::native::encode_request(
                protocol::store_publication::inspect::METHOD_ID,
                &TemplateLookupRequest {
                    template_id: template_id.clone(),
                    version: version.clone(),
                },
                protocol::store_publication::REQUEST_LIMITS,
            )
            .map_err(|_| InternalError::invalid_input())?;
            let reply =
                CallOps::bounded_wait(self.store_pid, protocol::store_publication::inspect::METHOD)
                    .with_raw_args(bytes)
                    .execute()
                    .await?
                    .into_bytes();
            let reply: Result<TemplateChunkSetInfoResponse, Error> =
                ic_wire::native::decode_response(
                    protocol::store_publication::inspect::METHOD_ID,
                    &reply,
                    protocol::store_publication::RESPONSE_LIMITS,
                )
                .map_err(|_| InternalError::invalid_input())?;
            reply.map_err(InternalError::observed_public)
        }
        .await;
        result.map_err(|error| {
            error.with_dependency_failure(
                self.store_pid,
                protocol::store_publication::inspect::METHOD,
                "inspect_template",
            )
        })
    }

    pub(super) async fn status(&self) -> Result<WasmStoreStatusResponse, InternalError> {
        let result = async {
            let bytes = ic_wire::native::encode_request(
                protocol::store_inventory::storage::METHOD_ID,
                &(),
                protocol::store_inventory::REQUEST_LIMITS,
            )
            .map_err(|_| InternalError::invalid_input())?;
            let reply =
                CallOps::bounded_wait(self.store_pid, protocol::store_inventory::storage::METHOD)
                    .with_raw_args(bytes)
                    .execute()
                    .await?
                    .into_bytes();
            let reply: Result<WasmStoreStatusResponse, Error> = ic_wire::native::decode_response(
                protocol::store_inventory::storage::METHOD_ID,
                &reply,
                protocol::store_inventory::RESPONSE_LIMITS,
            )
            .map_err(|_| InternalError::invalid_input())?;
            reply.map_err(InternalError::observed_public)
        }
        .await;
        result.map_err(|error| {
            error.with_dependency_failure(
                self.store_pid,
                protocol::store_inventory::storage::METHOD,
                "storage_status",
            )
        })
    }

    pub(super) async fn prepare_chunk_set(
        &self,
        _publication_permit: &CostGuardPermit,
        request: TemplateChunkSetPrepareInput,
    ) -> Result<TemplateChunkSetInfoResponse, InternalError> {
        let result = async {
            let bytes = ic_wire::native::encode_request(
                protocol::store_publication::prepare::METHOD_ID,
                &request,
                protocol::store_publication::REQUEST_LIMITS,
            )
            .map_err(|_| InternalError::invalid_input())?;
            let reply =
                CallOps::bounded_wait(self.store_pid, protocol::store_publication::prepare::METHOD)
                    .with_raw_args(bytes)
                    .execute()
                    .await?
                    .into_bytes();
            let reply: Result<TemplateChunkSetInfoResponse, Error> =
                ic_wire::native::decode_response(
                    protocol::store_publication::prepare::METHOD_ID,
                    &reply,
                    protocol::store_publication::RESPONSE_LIMITS,
                )
                .map_err(|_| InternalError::invalid_input())?;
            reply.map_err(InternalError::observed_public)
        }
        .await;
        result.map_err(|error| {
            error.with_dependency_failure(
                self.store_pid,
                protocol::store_publication::prepare::METHOD,
                "prepare_chunk_set",
            )
        })
    }

    pub(super) async fn stage_manifest(
        &self,
        _publication_permit: &CostGuardPermit,
        request: TemplateManifestInput,
    ) -> Result<(), InternalError> {
        let result = async {
            let bytes = ic_wire::native::encode_request(
                protocol::store_publication::stage::METHOD_ID,
                &request,
                protocol::store_publication::REQUEST_LIMITS,
            )
            .map_err(|_| InternalError::invalid_input())?;
            let reply =
                CallOps::bounded_wait(self.store_pid, protocol::store_publication::stage::METHOD)
                    .with_raw_args(bytes)
                    .execute()
                    .await?
                    .into_bytes();
            let reply: Result<(), Error> = ic_wire::native::decode_response(
                protocol::store_publication::stage::METHOD_ID,
                &reply,
                protocol::store_publication::RESPONSE_LIMITS,
            )
            .map_err(|_| InternalError::invalid_input())?;
            reply.map_err(InternalError::observed_public)
        }
        .await;
        result.map_err(|error| {
            error.with_dependency_failure(
                self.store_pid,
                protocol::store_publication::stage::METHOD,
                "stage_manifest",
            )
        })
    }

    pub(super) async fn publish_chunk(
        &self,
        _publication_permit: &CostGuardPermit,
        template_id: &TemplateId,
        version: &TemplateVersion,
        chunk_index: u32,
        bytes: &[u8],
    ) -> Result<(), InternalError> {
        let result = async {
            let payload = ic_wire::native::encode_request(
                protocol::template_bytes::METHOD_ID,
                &canic_contracts::dto::template_bytes::PublishRequestRef {
                    preparation: None,
                    template_id,
                    version,
                    chunk_index,
                    bytes,
                },
                protocol::template_bytes::REQUEST_LIMITS,
            )
            .map_err(|_| InternalError::invalid_input())?;
            let bytes = CallOps::bounded_wait(self.store_pid, Self::PUBLISH_CHUNK)
                .with_raw_args(payload)
                .execute()
                .await?
                .into_bytes();
            let reply: Result<(), Error> = ic_wire::native::decode_response(
                protocol::template_bytes::METHOD_ID,
                &bytes,
                protocol::template_bytes::RESPONSE_LIMITS,
            )
            .map_err(|_| InternalError::invalid_input())?;
            reply.map_err(InternalError::observed_public)
        }
        .await;
        result.map_err(|error| {
            error.with_dependency_failure(self.store_pid, Self::PUBLISH_CHUNK, "publish_chunk")
        })
    }

    pub(super) async fn run_gc(
        &self,
        operation_id: [u8; 32],
        target: WasmStoreGcTarget,
    ) -> Result<(), InternalError> {
        let result = async {
            let bytes = ic_wire::native::encode_request(
                protocol::store_gc::METHOD_ID,
                &WasmStoreGcRequest {
                    operation_id,
                    target,
                },
                protocol::store_gc::REQUEST_LIMITS,
            )
            .map_err(|_| InternalError::invalid_input())?;
            let reply = CallOps::bounded_wait(self.store_pid, protocol::store_gc::METHOD)
                .with_raw_args(bytes)
                .execute()
                .await?
                .into_bytes();
            let reply: Result<canic_contracts::dto::role::OperationReceipt, Error> =
                ic_wire::native::decode_response(
                    protocol::store_gc::METHOD_ID,
                    &reply,
                    protocol::store_gc::RESPONSE_LIMITS,
                )
                .map_err(|_| InternalError::invalid_input())?;
            let receipt = reply.map_err(InternalError::observed_public)?;
            if receipt.operation_id != operation_id {
                return Err(InternalError::conflict());
            }
            Ok(())
        }
        .await;
        result.map_err(|error| {
            error.with_dependency_failure(self.store_pid, protocol::store_gc::METHOD, "run_gc")
        })
    }

    pub(super) async fn reclaim_deletion_cycles(
        &self,
        request: WasmStoreDeletionCycleReclamationRequest,
    ) -> Result<WasmStoreDeletionCycleReclamationResponse, InternalError> {
        let result = async {
            let bytes = ic_wire::native::encode_request(
                protocol::store_control::reclaim::METHOD_ID,
                &request,
                protocol::store_control::REQUEST_LIMITS,
            )
            .map_err(|_| InternalError::invalid_input())?;
            let bytes =
                CallOps::bounded_wait(self.store_pid, protocol::store_control::reclaim::METHOD)
                    .with_raw_args(bytes)
                    .execute()
                    .await?
                    .into_bytes();
            let reply: Result<canic_contracts::dto::store_control::ReclamationResponse, Error> =
                ic_wire::native::decode_response(
                    protocol::store_control::reclaim::METHOD_ID,
                    &bytes,
                    protocol::store_control::RESPONSE_LIMITS,
                )
                .map_err(|_| InternalError::invalid_input())?;
            Ok(reply.map_err(InternalError::observed_public)?.into())
        }
        .await;
        result.map_err(|error: InternalError| {
            error.with_dependency_failure(
                self.store_pid,
                protocol::store_control::reclaim::METHOD,
                "reclaim_deletion_cycles",
            )
        })
    }

    pub(super) async fn chunk(
        &self,
        template_id: &TemplateId,
        version: &TemplateVersion,
        chunk_index: u32,
    ) -> Result<Vec<u8>, InternalError> {
        let result = async {
            let bytes = ic_wire::native::encode_request(
                protocol::wasm_store_chunk::METHOD_ID,
                &TemplateChunkRequest {
                    template_id: template_id.clone(),
                    version: version.clone(),
                    chunk_index,
                },
                protocol::wasm_store_chunk::REQUEST_LIMITS,
            )
            .map_err(|_| InternalError::invalid_input())?;
            let reply = CallOps::bounded_wait(self.store_pid, Self::CHUNK)
                .with_raw_args(bytes)
                .execute()
                .await?
                .into_bytes();
            let response: Result<TemplateChunkResponse, Error> = ic_wire::native::decode_response(
                protocol::wasm_store_chunk::METHOD_ID,
                &reply,
                protocol::wasm_store_chunk::RESPONSE_LIMITS,
            )
            .map_err(|_| InternalError::invalid_input())?;
            response
                .map(|chunk| chunk.bytes)
                .map_err(InternalError::observed_public)
        }
        .await;
        result.map_err(|error| {
            error.with_dependency_failure(self.store_pid, Self::CHUNK, "read_chunk")
        })
    }

    pub(in crate::workflow) async fn prepare_fixture(
        &self,
        descriptor: FixtureDescriptor,
    ) -> Result<Result<FixtureSourceStatus, FixtureStoreError>, InternalError> {
        let result = async {
            let bytes = ic_wire::native::encode_request(
                protocol::store_publication::fixture_prepare::METHOD_ID,
                &descriptor,
                protocol::store_publication::REQUEST_LIMITS,
            )
            .map_err(|_| InternalError::invalid_input())?;
            let reply = CallOps::bounded_wait(
                self.store_pid,
                protocol::store_publication::fixture_prepare::METHOD,
            )
            .with_raw_args(bytes)
            .execute()
            .await?
            .into_bytes();
            let reply: Result<Result<FixtureSourceStatus, FixtureStoreError>, Error> =
                ic_wire::native::decode_response(
                    protocol::store_publication::fixture_prepare::METHOD_ID,
                    &reply,
                    protocol::store_publication::RESPONSE_LIMITS,
                )
                .map_err(|_| InternalError::invalid_input())?;
            reply.map_err(InternalError::observed_public)
        }
        .await;
        result.map_err(|error| {
            error.with_dependency_failure(
                self.store_pid,
                protocol::store_publication::fixture_prepare::METHOD,
                "prepare_fixture",
            )
        })
    }

    pub(in crate::workflow) async fn fixture_status(
        &self,
        content_id: [u8; 32],
    ) -> Result<Result<FixtureSourceStatus, FixtureStoreError>, InternalError> {
        let result = async {
            let bytes = ic_wire::native::encode_request(
                protocol::store_catalog::fixture::METHOD_ID,
                &content_id,
                protocol::store_catalog::REQUEST_LIMITS,
            )
            .map_err(|_| InternalError::invalid_input())?;
            let bytes =
                CallOps::bounded_wait(self.store_pid, protocol::store_catalog::fixture::METHOD)
                    .with_raw_args(bytes)
                    .execute()
                    .await?
                    .into_bytes();
            let reply: Result<Result<FixtureSourceStatus, FixtureStoreError>, Error> =
                ic_wire::native::decode_response(
                    protocol::store_catalog::fixture::METHOD_ID,
                    &bytes,
                    protocol::store_catalog::RESPONSE_LIMITS,
                )
                .map_err(|_| InternalError::invalid_input())?;
            reply.map_err(InternalError::observed_public)
        }
        .await;
        result.map_err(|error: InternalError| {
            error.with_dependency_failure(
                self.store_pid,
                protocol::store_catalog::fixture::METHOD,
                "fixture_status",
            )
        })
    }

    pub(in crate::workflow) async fn fixture_grant(
        &self,
        target: Principal,
    ) -> Result<Option<FixtureGrant>, InternalError> {
        let result = async {
            let bytes = ic_wire::native::encode_request(
                protocol::store_catalog::grant::METHOD_ID,
                &ic_platform::Principal::from(target),
                protocol::store_catalog::REQUEST_LIMITS,
            )
            .map_err(|_| InternalError::invalid_input())?;
            let bytes =
                CallOps::bounded_wait(self.store_pid, protocol::store_catalog::grant::METHOD)
                    .with_raw_args(bytes)
                    .execute()
                    .await?
                    .into_bytes();
            let reply: Result<
                Option<canic_contracts::dto::runtime_reporting::FixtureGrant>,
                Error,
            > = ic_wire::native::decode_response(
                protocol::store_catalog::grant::METHOD_ID,
                &bytes,
                protocol::store_catalog::RESPONSE_LIMITS,
            )
            .map_err(|_| InternalError::invalid_input())?;
            Ok(reply
                .map_err(InternalError::observed_public)?
                .map(Into::into))
        }
        .await;
        result.map_err(|error: InternalError| {
            error.with_dependency_failure(
                self.store_pid,
                protocol::store_catalog::grant::METHOD,
                "fixture_grant",
            )
        })
    }

    pub(in crate::workflow) async fn set_fixture_grant(
        &self,
        request: FixtureGrantRequest,
    ) -> Result<Result<FixtureGrant, FixtureStoreError>, InternalError> {
        let result = async {
            let bytes = ic_wire::native::encode_request(
                protocol::store_control::grant::METHOD_ID,
                &canic_contracts::dto::store_control::FixtureGrantRequest::from(request),
                protocol::store_control::REQUEST_LIMITS,
            )
            .map_err(|_| InternalError::invalid_input())?;
            let bytes =
                CallOps::bounded_wait(self.store_pid, protocol::store_control::grant::METHOD)
                    .with_raw_args(bytes)
                    .execute()
                    .await?
                    .into_bytes();
            let reply: Result<
                Result<canic_contracts::dto::runtime_reporting::FixtureGrant, FixtureStoreError>,
                Error,
            > = ic_wire::native::decode_response(
                protocol::store_control::grant::METHOD_ID,
                &bytes,
                protocol::store_control::RESPONSE_LIMITS,
            )
            .map_err(|_| InternalError::invalid_input())?;
            Ok(reply
                .map_err(InternalError::observed_public)?
                .map(Into::into))
        }
        .await;
        result.map_err(|error: InternalError| {
            error.with_dependency_failure(
                self.store_pid,
                protocol::store_control::grant::METHOD,
                "set_fixture_grant",
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::WasmStoreInternalClient;
    use std::collections::BTreeSet;

    #[test]
    fn typed_client_uses_only_store_command_catalog_and_byte_lanes() {
        let all = WasmStoreInternalClient::ENDPOINTS
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();

        assert_eq!(
            all,
            BTreeSet::from([
                "canic_wasm_store_set_fixture_grant",
                "canic_wasm_store_reclaim_deletion_cycles",
                "canic_wasm_store_chunk",
                "canic_wasm_store_publish_chunk",
                "canic_wasm_store_fixture_status",
                "canic_wasm_store_fixture_grant",
                "canic_wasm_store_run_gc",
                "canic_wasm_store_prepare_fixture",
                "canic_wasm_store_inspect_template",
                "canic_wasm_store_prepare_chunk_set",
                "canic_wasm_store_stage_manifest",
                "canic_wasm_store_inventory",
                "canic_wasm_store_storage",
            ])
        );
        assert_eq!(
            all.len(),
            WasmStoreInternalClient::ENDPOINTS.len(),
            "typed wasm-store client endpoint methods must be unique"
        );
    }
}
