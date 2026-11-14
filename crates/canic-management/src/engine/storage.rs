//! Versioned payload migration inside the original two checked allocations.
//! IC Storage directory capacities are fixed at creation; no backing rewrite is used.
use canic_management_protocol::{LIMITS, MAX_WASM, Status, upgrade};
use ic_storage::adapter::{Bound, CodecError, Storable};
use std::borrow::Cow;

const JOURNAL_TAG: &[u8] = b"CUJ2";
const ARTIFACT_TAG: &[u8] = b"CUA2";
pub(super) const JOURNAL_BYTES: usize = 8192;
pub(super) const ARTIFACT_BYTES: usize = 2 * MAX_WASM + 8;
const JOURNAL_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: JOURNAL_BYTES - 4,
    allocation: 2 * LIMITS.allocation,
    nodes: 2 * LIMITS.nodes,
    depth: LIMITS.depth + 1,
};

#[derive(Clone, Default)]
pub(super) struct Journal(pub Status, pub upgrade::Status);
impl Storable for Journal {
    const BOUND: Bound = Bound::Bounded {
        max_size: JOURNAL_BYTES as u32,
        is_fixed_size: false,
    };
    fn to_bytes(&self) -> Result<Cow<'_, [u8]>, CodecError> {
        let payload = ic_wire::native::encode(&(&self.0, &self.1), JOURNAL_LIMITS)
            .map_err(|_| CodecError::Invalid("upgrade journal"))?;
        let mut bytes = Vec::with_capacity(JOURNAL_TAG.len() + payload.len());
        bytes.extend_from_slice(JOURNAL_TAG);
        bytes.extend_from_slice(&payload);
        Ok(Cow::Owned(bytes))
    }
    fn from_bytes(bytes: Cow<'_, [u8]>) -> Result<Self, CodecError> {
        if let Some(payload) = bytes.strip_prefix(JOURNAL_TAG) {
            let (creation, upgrade) = ic_wire::native::decode(payload, JOURNAL_LIMITS)
                .map_err(|_| CodecError::Invalid("upgrade journal"))?;
            Ok(Self(creation, upgrade))
        } else {
            // The sole historical schema is the original bounded creation status.
            // Lifecycle validates both stores before committing the migration.
            ic_wire::native::decode(&bytes, LIMITS)
                .map(|creation| Self(creation, upgrade::Status::default()))
                .map_err(|_| CodecError::Invalid("creation journal migration"))
        }
    }
}

#[derive(Clone, Default)]
pub(super) struct Artifact(pub Vec<u8>, pub Vec<u8>);
impl Storable for Artifact {
    const BOUND: Bound = Bound::Bounded {
        max_size: ARTIFACT_BYTES as u32,
        is_fixed_size: false,
    };
    fn to_bytes(&self) -> Result<Cow<'_, [u8]>, CodecError> {
        if self.0.len() > MAX_WASM || self.1.len() > MAX_WASM {
            return Err(CodecError::Invalid("artifact bounds"));
        }
        let mut bytes = Vec::with_capacity(8 + self.0.len() + self.1.len());
        bytes.extend_from_slice(ARTIFACT_TAG);
        bytes.extend_from_slice(&(self.0.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&self.0);
        bytes.extend_from_slice(&self.1);
        Ok(Cow::Owned(bytes))
    }
    fn from_bytes(bytes: Cow<'_, [u8]>) -> Result<Self, CodecError> {
        if let Some(payload) = bytes.strip_prefix(ARTIFACT_TAG) {
            let length: [u8; 4] = payload
                .get(..4)
                .ok_or(CodecError::Invalid("artifact header"))?
                .try_into()
                .map_err(|_| CodecError::Invalid("artifact header"))?;
            let length = u32::from_le_bytes(length) as usize;
            let payload = &payload[4..];
            if length > MAX_WASM || length > payload.len() || payload.len() - length > MAX_WASM {
                return Err(CodecError::Invalid("artifact bounds"));
            }
            Ok(Self(payload[..length].to_vec(), payload[length..].to_vec()))
        } else if bytes.len() <= MAX_WASM
            && (bytes.is_empty() || bytes.starts_with(b"\0asm\x01\0\0\0"))
        {
            Ok(Self(bytes.into_owned(), vec![]))
        } else {
            Err(CodecError::Invalid("creation artifact migration"))
        }
    }
}
