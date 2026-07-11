use crate::{
    dto::{
        state::{SubnetStateResponse, WasmStoreStateResponse},
        template::WasmStorePublicationStateResponse,
    },
    storage::stable::state::subnet::{
        ControlPlaneSubnetStateData, PublicationStoreStateRecord, WasmStoreRecord,
    },
    view::state::{PublicationStoreStateView, WasmStoreGcView, WasmStoreView},
};

///
/// SubnetStateMapper
///

pub struct SubnetStateMapper;

impl SubnetStateMapper {
    // Project one stored wasm-store record into the internal read-only shape.
    #[must_use]
    pub fn wasm_store_record_to_view(record: WasmStoreRecord) -> WasmStoreView {
        WasmStoreView {
            binding: record.binding,
            pid: record.pid,
            created_at: record.created_at,
            gc: WasmStoreGcView {
                mode: record.gc.mode,
                changed_at: record.gc.changed_at,
                prepared_at: record.gc.prepared_at,
                started_at: record.gc.started_at,
                completed_at: record.gc.completed_at,
                runs_completed: record.gc.runs_completed,
            },
        }
    }

    // Project stored publication lifecycle state into the internal read-only shape.
    #[must_use]
    pub fn publication_store_record_to_view(
        record: PublicationStoreStateRecord,
    ) -> PublicationStoreStateView {
        PublicationStoreStateView {
            active_binding: record.active_binding,
            detached_binding: record.detached_binding,
            retired_binding: record.retired_binding,
            generation: record.generation,
            changed_at: record.changed_at,
            retired_at: record.retired_at,
        }
    }

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
    pub fn data_to_response(data: ControlPlaneSubnetStateData) -> SubnetStateResponse {
        SubnetStateResponse {
            wasm_stores: data
                .record
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

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ids::{WasmStoreBinding, WasmStoreGcMode},
        storage::stable::state::subnet::WasmStoreGcRecord,
    };
    use canic_core::cdk::types::Principal;

    #[test]
    fn records_project_to_exact_internal_views() {
        let active_binding = WasmStoreBinding::new("active");
        let detached_binding = WasmStoreBinding::new("detached");
        let retired_binding = WasmStoreBinding::new("retired");
        let publication =
            SubnetStateMapper::publication_store_record_to_view(PublicationStoreStateRecord {
                active_binding: Some(active_binding.clone()),
                detached_binding: Some(detached_binding.clone()),
                retired_binding: Some(retired_binding.clone()),
                generation: 7,
                changed_at: 11,
                retired_at: 13,
            });

        assert_eq!(
            publication,
            PublicationStoreStateView {
                active_binding: Some(active_binding),
                detached_binding: Some(detached_binding),
                retired_binding: Some(retired_binding),
                generation: 7,
                changed_at: 11,
                retired_at: 13,
            }
        );

        let binding = WasmStoreBinding::new("store");
        let pid = Principal::from_slice(&[1; 29]);
        let store = SubnetStateMapper::wasm_store_record_to_view(WasmStoreRecord {
            binding: binding.clone(),
            pid,
            created_at: 17,
            gc: WasmStoreGcRecord {
                mode: WasmStoreGcMode::Complete,
                changed_at: 19,
                prepared_at: Some(23),
                started_at: Some(29),
                completed_at: Some(31),
                runs_completed: 2,
            },
        });

        assert_eq!(
            store,
            WasmStoreView {
                binding,
                pid,
                created_at: 17,
                gc: WasmStoreGcView {
                    mode: WasmStoreGcMode::Complete,
                    changed_at: 19,
                    prepared_at: Some(23),
                    started_at: Some(29),
                    completed_at: Some(31),
                    runs_completed: 2,
                },
            }
        );
    }
}
