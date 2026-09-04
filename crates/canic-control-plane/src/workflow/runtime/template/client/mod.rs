use crate::{
    dto::template::{
        StoreCommand, StoreCommandResponse, StoreStatusRequest, StoreStatusResponse,
        TemplateChunkRequest, TemplateChunkResponse, TemplateChunkSetInfoResponse,
        TemplateChunkSetPrepareInput, TemplateLookupRequest, TemplateManifestInput,
        WasmStoreCatalogEntryResponse, WasmStoreDeletionCycleReclamationRequest,
        WasmStoreDeletionCycleReclamationResponse, WasmStoreStatusResponse,
    },
    ids::{TemplateId, TemplateVersion},
};
use candid::{CandidType, utils::ArgumentEncoder};
use canic_core::cdk::types::Principal;
use canic_core::{
    control_plane_support::{
        error::InternalError,
        ops::{cost_guard::CostGuardPermit, ic::call::CallOps},
    },
    dto::{error::Error, role::OperationStatusRequest},
    protocol,
};

///
/// WasmStoreInternalClient
///
pub(in crate::workflow::runtime::template) struct WasmStoreInternalClient {
    store_pid: Principal,
}

impl WasmStoreInternalClient {
    const COMMAND: &str = protocol::CANIC_WASM_STORE_COMMAND;
    const CHUNK: &str = "canic_wasm_store_chunk";
    const PUBLISH_CHUNK: &str = "canic_wasm_store_publish_chunk";
    const STATUS: &str = protocol::CANIC_WASM_STORE_STATUS;
    #[cfg(test)]
    const ENDPOINTS: &[&str] = &[
        Self::COMMAND,
        Self::CHUNK,
        Self::PUBLISH_CHUNK,
        Self::STATUS,
    ];

    pub(super) const fn new(store_pid: Principal) -> Self {
        Self { store_pid }
    }

    pub(super) async fn catalog(
        &self,
    ) -> Result<Vec<WasmStoreCatalogEntryResponse>, InternalError> {
        match self.status_request(StoreStatusRequest::Catalog).await? {
            StoreStatusResponse::Catalog(catalog) => Ok(catalog),
            _ => Err(InternalError::conflict()),
        }
    }

    pub(super) async fn info(
        &self,
        template_id: &TemplateId,
        version: &TemplateVersion,
    ) -> Result<TemplateChunkSetInfoResponse, InternalError> {
        match self
            .command(StoreCommand::InspectTemplate(TemplateLookupRequest {
                template_id: template_id.clone(),
                version: version.clone(),
            }))
            .await?
        {
            StoreCommandResponse::InspectTemplate(info) => Ok(info),
            _ => Err(InternalError::conflict()),
        }
    }

    pub(super) async fn status(&self) -> Result<WasmStoreStatusResponse, InternalError> {
        match self.status_request(StoreStatusRequest::Storage).await? {
            StoreStatusResponse::Storage(status) => Ok(status),
            _ => Err(InternalError::conflict()),
        }
    }

    pub(super) async fn prepare_chunk_set(
        &self,
        _publication_permit: &CostGuardPermit,
        request: TemplateChunkSetPrepareInput,
    ) -> Result<TemplateChunkSetInfoResponse, InternalError> {
        match self.command(StoreCommand::PrepareChunkSet(request)).await? {
            StoreCommandResponse::PrepareChunkSet(info) => Ok(info),
            _ => Err(InternalError::conflict()),
        }
    }

    pub(super) async fn stage_manifest(
        &self,
        _publication_permit: &CostGuardPermit,
        request: TemplateManifestInput,
    ) -> Result<(), InternalError> {
        match self.command(StoreCommand::StageManifest(request)).await? {
            StoreCommandResponse::StageManifest => Ok(()),
            _ => Err(InternalError::conflict()),
        }
    }

    pub(super) async fn publish_chunk(
        &self,
        _publication_permit: &CostGuardPermit,
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

    pub(super) async fn run_gc(&self, operation_id: [u8; 32]) -> Result<(), InternalError> {
        match self
            .command(StoreCommand::RunGc(OperationStatusRequest { operation_id }))
            .await?
        {
            StoreCommandResponse::OperationAccepted(receipt)
                if receipt.operation_id == operation_id =>
            {
                Ok(())
            }
            _ => Err(InternalError::conflict()),
        }
    }

    pub(super) async fn reclaim_deletion_cycles(
        &self,
        request: WasmStoreDeletionCycleReclamationRequest,
    ) -> Result<WasmStoreDeletionCycleReclamationResponse, InternalError> {
        match self
            .command(StoreCommand::ReclaimDeletionCycles(request))
            .await?
        {
            StoreCommandResponse::ReclaimDeletionCycles(response) => Ok(response),
            _ => Err(InternalError::conflict()),
        }
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
                (TemplateChunkRequest {
                    template_id: template_id.clone(),
                    version: version.clone(),
                    chunk_index,
                },),
            )
            .await?;

        Ok(response.bytes)
    }

    async fn command(&self, command: StoreCommand) -> Result<StoreCommandResponse, InternalError> {
        let call = CallOps::bounded_wait(self.store_pid, Self::COMMAND)
            .with_arg(command)?
            .execute()
            .await?;
        let result: Result<StoreCommandResponse, Error> = call.candid()?;
        result.map_err(InternalError::observed_public)
    }

    async fn status_request(
        &self,
        request: StoreStatusRequest,
    ) -> Result<StoreStatusResponse, InternalError> {
        let call = CallOps::bounded_wait(self.store_pid, Self::STATUS)
            .with_arg(request)?
            .execute()
            .await?;
        let result: Result<StoreStatusResponse, Error> = call.candid()?;
        result.map_err(InternalError::observed_public)
    }

    async fn call_result<T, A>(&self, method: &'static str, arg: A) -> Result<T, InternalError>
    where
        T: CandidType + serde::de::DeserializeOwned,
        A: ArgumentEncoder,
    {
        let call = CallOps::bounded_wait(self.store_pid, method)
            .with_args(arg)
            .map_err(|_err| InternalError::public(canic_core::diagnostics::codes::STATE_INVALID))?
            .execute()
            .await
            .map_err(|_err| {
                InternalError::public(canic_core::diagnostics::codes::STATE_UNAVAILABLE)
            })?;
        let call_res: Result<T, Error> = call
            .candid::<Result<T, Error>>()
            .map_err(|_err| InternalError::public(canic_core::diagnostics::codes::STATE_INVALID))?;

        call_res.map_err(InternalError::observed_public)
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
    use std::collections::BTreeSet;

    #[test]
    fn typed_client_uses_only_store_command_status_and_byte_lanes() {
        let all = WasmStoreInternalClient::ENDPOINTS
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();

        assert_eq!(
            all,
            BTreeSet::from([
                "canic_wasm_store_command",
                "canic_wasm_store_chunk",
                "canic_wasm_store_publish_chunk",
                "canic_wasm_store_status",
            ])
        );
        assert_eq!(
            all.len(),
            WasmStoreInternalClient::ENDPOINTS.len(),
            "typed wasm-store client endpoint methods must be unique"
        );
    }
}
