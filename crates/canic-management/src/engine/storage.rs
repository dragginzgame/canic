//! Versioned payload migration inside the original two checked allocations.
//! IC Storage directory capacities are fixed at creation; no backing rewrite is used.
use canic_management_protocol::{LIMITS, MAX_WASM, Status, upgrade};
use ic_storage::adapter::{Bound, CodecError, Storable};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

const JOURNAL_TAG: &[u8] = b"CUJ3";
const ARTIFACT_TAG: &[u8] = b"CUA2";
pub(super) const JOURNAL_BYTES: usize = 16_384;
pub(super) const ARTIFACT_BYTES: usize = 2 * MAX_WASM + 24;
const JOURNAL_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: JOURNAL_BYTES - 4,
    allocation: 2 * (1 + upgrade::MAX_UPGRADES) * LIMITS.allocation,
    nodes: 2 * (1 + upgrade::MAX_UPGRADES) * LIMITS.nodes,
    depth: LIMITS.depth + 4,
};

#[derive(Clone, Default, Deserialize, Serialize)]
pub(super) struct Journal(pub Status, pub upgrade::Status, pub Vec<upgrade::Status>);
impl Journal {
    fn decode(bytes: &[u8]) -> Result<Self, CodecError> {
        if let Some(payload) = bytes.strip_prefix(JOURNAL_TAG) {
            let (creation, upgrade, history) = ic_wire::native::decode::<(
                Status,
                upgrade::Status,
                Vec<upgrade::Status>,
            )>(payload, JOURNAL_LIMITS)
            .map_err(|_| CodecError::Invalid("upgrade journal"))?;
            if history.len() >= upgrade::MAX_UPGRADES {
                return Err(CodecError::Invalid("upgrade history bound"));
            }
            Ok(Self(creation, upgrade, history))
        } else if let Some(payload) = bytes.strip_prefix(b"CUJ2") {
            let (creation, upgrade) = ic_wire::native::decode(payload, JOURNAL_LIMITS)
                .map_err(|_| CodecError::Invalid("single upgrade journal migration"))?;
            Ok(Self(creation, upgrade, vec![]))
        } else {
            // The sole historical schema is the original bounded creation status.
            // Lifecycle validates both stores before committing the migration.
            ic_wire::native::decode(bytes, LIMITS)
                .map(|creation| Self(creation, upgrade::Status::default(), vec![]))
                .map_err(|_| CodecError::Invalid("creation journal migration"))
        }
    }
}

#[derive(Clone, Default)]
pub(super) struct Inline(pub Vec<u8>, pub Vec<u8>);
impl Storable for Inline {
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

// Inline payloads keep their original codec. Store custody fits the same lease and
// never retains application bytes. Old executables refuse the new per-child tag.
#[derive(Clone)]
pub(super) enum Artifact {
    Inline(Inline),
    Stored(canic_management_protocol::stored::Sources),
}
impl Default for Artifact {
    fn default() -> Self {
        Self::Inline(Inline::default())
    }
}
impl Artifact {
    pub(super) fn sources(&self) -> Option<&canic_management_protocol::stored::Sources> {
        match self {
            Self::Inline(_) => None,
            Self::Stored(value) => Some(value),
        }
    }
    pub(super) fn wasm(&self, upgrade: bool) -> &[u8] {
        match self {
            Self::Inline(value) => {
                if upgrade {
                    &value.1
                } else {
                    &value.0
                }
            }
            Self::Stored(_) => panic!("Store bytes never belong to Root"),
        }
    }
    pub(super) fn hash(&self, upgrade: bool) -> Option<[u8; 32]> {
        match self {
            Self::Inline(_) => {
                let bytes = self.wasm(upgrade);
                (!bytes.is_empty()).then(|| canic_management_protocol::digest(bytes))
            }
            Self::Stored(value) => if upgrade {
                value.upgrades.last()
            } else {
                Some(&value.creation)
            }
            .map(|source| source.reference.manifest.hash),
        }
    }
}
impl Storable for Artifact {
    const BOUND: Bound = Inline::BOUND;
    fn to_bytes(&self) -> Result<Cow<'_, [u8]>, CodecError> {
        match self {
            Self::Inline(value) => value.to_bytes(),
            Self::Stored(value) => {
                if value.upgrades.len() > upgrade::MAX_UPGRADES {
                    return Err(CodecError::Invalid("Store history bound"));
                }
                let mut bytes = b"CUS1".to_vec();
                bytes.extend(
                    ic_wire::native::encode(value, canic_management_protocol::stored::LIMITS)
                        .map_err(|_| CodecError::Invalid("Store custody"))?,
                );
                Ok(Cow::Owned(bytes))
            }
        }
    }
    fn from_bytes(bytes: Cow<'_, [u8]>) -> Result<Self, CodecError> {
        if let Some(payload) = bytes.strip_prefix(b"CUS1") {
            let value: canic_management_protocol::stored::Sources =
                ic_wire::native::decode(payload, canic_management_protocol::stored::LIMITS)
                    .map_err(|_| CodecError::Invalid("Store custody"))?;
            if value.upgrades.len() > upgrade::MAX_UPGRADES {
                return Err(CodecError::Invalid("Store history bound"));
            }
            Ok(Self::Stored(value))
        } else {
            Inline::from_bytes(bytes).map(Self::Inline)
        }
    }
}

// Both children share the original checked leases; each retains separate payloads.
#[derive(Clone, Default)]
pub(super) struct Journals(
    pub [Journal; 2],
    pub Option<canic_management_protocol::store_maintenance::Status>,
);
impl Storable for Journals {
    const BOUND: Bound = Bound::Bounded {
        max_size: JOURNAL_BYTES as u32,
        is_fixed_size: false,
    };
    fn to_bytes(&self) -> Result<Cow<'_, [u8]>, CodecError> {
        if self
            .0
            .iter()
            .any(|journal| journal.2.len() >= upgrade::MAX_UPGRADES)
        {
            return Err(CodecError::Invalid("upgrade history bound"));
        }
        let payload = ic_wire::native::encode(&(&self.0, &self.1), JOURNAL_LIMITS)
            .map_err(|_| CodecError::Invalid("child journals"))?;
        let mut bytes = b"CUJ5".to_vec();
        bytes.extend_from_slice(&payload);
        Ok(Cow::Owned(bytes))
    }
    fn from_bytes(bytes: Cow<'_, [u8]>) -> Result<Self, CodecError> {
        if let Some(payload) = bytes.strip_prefix(b"CUJ5") {
            let (journals, maintenance) = ic_wire::native::decode(payload, JOURNAL_LIMITS)
                .map_err(|_| CodecError::Invalid("Store maintenance journal"))?;
            Ok(Self(journals, maintenance))
        } else if let Some(payload) = bytes.strip_prefix(b"CUJ4") {
            let journals: [Journal; 2] = ic_wire::native::decode(payload, JOURNAL_LIMITS)
                .map_err(|_| CodecError::Invalid("child journals"))?;
            if journals
                .iter()
                .any(|journal| journal.2.len() >= upgrade::MAX_UPGRADES)
            {
                return Err(CodecError::Invalid("upgrade history bound"));
            }
            Ok(Self(journals, None))
        } else {
            Ok(Self([Journal::decode(&bytes)?, Journal::default()], None))
        }
    }
}
#[derive(Clone, Default)]
pub(super) struct Artifacts(pub [Artifact; 2]);
impl Artifacts {
    pub(super) fn fits(&self) -> bool {
        let inline_bytes: usize = self
            .0
            .iter()
            .map(|artifact| match artifact {
                Artifact::Inline(value) => value.0.len() + value.1.len(),
                Artifact::Stored(_) => 0,
            })
            .sum();
        inline_bytes <= canic_management_protocol::children::MAX_ARTIFACT_BYTES
            && self
                .0
                .iter()
                .try_fold(8_usize, |size, artifact| {
                    artifact
                        .to_bytes()
                        .ok()
                        .and_then(|bytes| size.checked_add(bytes.len()))
                })
                .is_some_and(|size| size <= ARTIFACT_BYTES)
    }
}
impl Storable for Artifacts {
    const BOUND: Bound = Artifact::BOUND;
    fn to_bytes(&self) -> Result<Cow<'_, [u8]>, CodecError> {
        if !self.fits() {
            return Err(CodecError::Invalid("aggregate artifact budget"));
        }
        let first = self.0[0].to_bytes()?;
        let second = self.0[1].to_bytes()?;
        let mut bytes = b"CUA3".to_vec();
        bytes.extend_from_slice(&(first.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&first);
        bytes.extend_from_slice(&second);
        Ok(Cow::Owned(bytes))
    }
    fn from_bytes(bytes: Cow<'_, [u8]>) -> Result<Self, CodecError> {
        if let Some(payload) = bytes.strip_prefix(b"CUA3") {
            let length: [u8; 4] = payload
                .get(..4)
                .ok_or(CodecError::Invalid("child artifact header"))?
                .try_into()
                .map_err(|_| CodecError::Invalid("child artifact header"))?;
            let length = u32::from_le_bytes(length) as usize;
            let payload = &payload[4..];
            if length > payload.len() {
                return Err(CodecError::Invalid("child artifact bounds"));
            }
            Ok(Self([
                Artifact::from_bytes(Cow::Borrowed(&payload[..length]))?,
                Artifact::from_bytes(Cow::Borrowed(&payload[length..]))?,
            ]))
        } else {
            Ok(Self([Artifact::from_bytes(bytes)?, Artifact::default()]))
        }
    }
}
