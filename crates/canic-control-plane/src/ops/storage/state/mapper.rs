use crate::{
    dto::state::{SubnetStateResponse, WasmStoreStateResponse},
    storage::stable::state::subnet::{
        PublicationStoreStateRecord, SubnetStateRecord, WasmStoreRecord,
    },
};
use canic_template_types::dto::template::WasmStorePublicationStateResponse;

///
/// SubnetStateMapper
///

pub struct SubnetStateMapper;

impl SubnetStateMapper {
    // Map one stored wasm-store record into the DTO response shape.
    #[must_use]
    pub fn wasm_store_record_to_response(data: WasmStoreRecord) -> WasmStoreStateResponse {
        WasmStoreStateResponse {
            binding: data.binding,
            pid: data.pid,
            created_at: data.created_at,
        }
    }

    // Map the stored subnet-state snapshot into the public response shape.
    #[must_use]
    pub fn record_to_response(data: SubnetStateRecord) -> SubnetStateResponse {
        SubnetStateResponse {
            wasm_stores: data
                .wasm_stores
                .into_iter()
                .map(Self::wasm_store_record_to_response)
                .collect(),
        }
    }

    // Map the stored publication lifecycle record into the template response shape.
    #[must_use]
    pub fn publication_store_record_to_response(
        data: PublicationStoreStateRecord,
    ) -> WasmStorePublicationStateResponse {
        WasmStorePublicationStateResponse {
            active_binding: data.active_binding,
            detached_binding: data.detached_binding,
            retired_binding: data.retired_binding,
            generation: data.generation,
            changed_at: data.changed_at,
            retired_at: data.retired_at,
        }
    }
}
