//! Lossless checkpoint boundary conversion and bounded application-record storage.

use crate::checkpoint::{BusinessRecord, protocol::Binding};
use canic::__internal::core::cdk::structures::storable::{Bound, Storable};
use std::borrow::Cow;

const RECORD_HEADER: [u8; 4] = *b"CKW\x01";
const RECORD_MAX_BYTES: u32 = 65_536;
pub(super) const RECORD_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: RECORD_MAX_BYTES as usize,
    allocation: 2_097_152,
    nodes: 4096,
    depth: 32,
};

/// Encode the current application checkpoint and counter as one admitted value.
pub(super) fn encode(record: &BusinessRecord) -> Result<Vec<u8>, ic_wire::Error> {
    ic_wire::native::encode(&(RECORD_HEADER, record), RECORD_LIMITS)
}

/// Refuse foreign, incomplete or oversized records before restoring business state.
pub(super) fn decode(bytes: &[u8]) -> Result<BusinessRecord, ic_wire::Error> {
    if !bytes.starts_with(&RECORD_HEADER) {
        return Err(ic_wire::Error::Identity);
    }
    ic_wire::native::decode::<([u8; 4], BusinessRecord)>(bytes, RECORD_LIMITS)
        .map(|(_, record)| record)
}

impl Storable for BusinessRecord {
    const BOUND: Bound = Bound::Bounded {
        max_size: RECORD_MAX_BYTES,
        is_fixed_size: false,
    };

    fn to_bytes(&self) -> Result<Cow<'_, [u8]>, ic_storage::storable::CodecError> {
        encode(self)
            .map(Cow::Owned)
            .map_err(|_| ic_storage::storable::CodecError::Invalid("business record encode"))
    }

    fn from_bytes(bytes: Cow<'_, [u8]>) -> Result<Self, ic_storage::storable::CodecError> {
        decode(&bytes)
            .map_err(|_| ic_storage::storable::CodecError::Invalid("business record decode"))
    }
}

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
