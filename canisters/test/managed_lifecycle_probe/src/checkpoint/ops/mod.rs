//! Lossless conversion between native boundary data and the unchanged checkpoint model.

use crate::checkpoint::protocol::Binding;

impl From<Binding> for canic::ids::ApplicationCheckpointBinding {
    fn from(value: Binding) -> Self {
        Self {
            authority: value.authority.into(),
            operation_id: value.operation_id,
            sequence: value.sequence,
            unit_digest: value.unit_digest,
        }
    }
}

impl From<canic::ids::ApplicationCheckpointBinding> for Binding {
    fn from(value: canic::ids::ApplicationCheckpointBinding) -> Self {
        Self {
            authority: value.authority.into(),
            operation_id: value.operation_id,
            sequence: value.sequence,
            unit_digest: value.unit_digest,
        }
    }
}
