//! Versioned payloads in two bounded Storage records.
//! Logical predecessor codecs do not import an older Storage backing format.

#[cfg(test)]
mod tests;

use canic_management_protocol::{
    LIMITS, MAX_WASM, Status, children::ChildId, store_maintenance, upgrade,
};

use ic_wire::{Bytes, BytesRef, List};
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, collections::BTreeMap};

// Persist identities explicitly. The protocol remains the admission ceiling;
// accepting a new record shape never creates capacity or drops unknown entries.
type Records<T> = List<(u8, T), { ChildId::SLOT_COUNT }>;

// Older formats require every historical slot. Expanded records retain sorted
// identities explicitly and need no placeholder for an unallocated child.
fn identified<T>(entries: Records<T>) -> Result<BTreeMap<ChildId, T>, &'static str> {
    let entries = entries.into_vec();
    let mut map = BTreeMap::new();
    for (id, value) in entries {
        let slot = ChildId::placement(usize::from(id)).ok_or("child identity bound")?;
        if map.last_key_value().is_some_and(|(last, _)| *last >= slot) {
            return Err("child identity order");
        }
        map.insert(slot, value);
    }
    if !historical_or_complete(&map) {
        return Err("child identities");
    }
    Ok(map)
}

fn expanded<T>(entries: &BTreeMap<ChildId, T>) -> bool {
    entries
        .last_key_value()
        .is_some_and(|(id, _)| id.index() >= 4)
}

fn complete<T>(entries: &BTreeMap<ChildId, T>) -> bool {
    ChildId::ALL.iter().all(|id| entries.contains_key(id))
        && entries.contains_key(&ChildId::PROJECT[0]) == entries.contains_key(&ChildId::PROJECT[1])
}

fn historical_or_complete<T>(entries: &BTreeMap<ChildId, T>) -> bool {
    complete(entries)
        || entries
            .keys()
            .copied()
            .eq([ChildId::FIRST, ChildId::SECOND])
}

const JOURNAL_TAG: &[u8; 4] = b"CUJE";
const ARTIFACT_TAG: &[u8] = b"CUA2";
pub(super) const JOURNAL_BYTES: usize = 65_536;
// Each extra empty record encodes its slot, inline variant and two empty byte vectors.
const SLOT_FRAMING: usize = 4 * (ChildId::SLOT_COUNT - 2);
pub(super) const ARTIFACT_BYTES: usize = 2 * MAX_WASM + 24 + SLOT_FRAMING;
pub(super) const JOURNAL_KEY: &str = "manager.journal.v2";
pub(super) const ARTIFACT_KEY: &str = "manager.artifacts.v1";
#[cfg(feature = "sharding")]
pub(super) const SHARDS_KEY: &str = "manager.shards.v2";
#[cfg(feature = "sharding")]
pub(super) const CLAIMS_KEY: &str = "manager.shard_claims.v2";
#[cfg(feature = "sharding")]
pub(super) const WORKER_HISTORY_KEY: &str = "manager.worker_history.v1";
pub(super) const KEYS: &[&str] = &[
    #[cfg(feature = "role-issuer")]
    super::role_issuer::KEY,
    JOURNAL_KEY,
    ARTIFACT_KEY,
    #[cfg(feature = "sharding")]
    SHARDS_KEY,
    #[cfg(feature = "sharding")]
    CLAIMS_KEY,
    #[cfg(feature = "sharding")]
    WORKER_HISTORY_KEY,
    #[cfg(feature = "backup")]
    "manager.snapshot.v1",
];
const JOURNAL_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: JOURNAL_BYTES - 4,
    allocation: ChildId::ALL.len() * (1 + upgrade::MAX_UPGRADES) * LIMITS.allocation
        + store_maintenance::MAX_UPGRADES * store_maintenance::LIMITS.allocation,
    nodes: ChildId::ALL.len() * (1 + upgrade::MAX_UPGRADES) * LIMITS.nodes
        + store_maintenance::MAX_UPGRADES * store_maintenance::LIMITS.nodes,
    depth: LIMITS.depth + 5,
};
const ARTIFACT_LIMITS: ic_wire::Limits = ic_wire::Limits {
    // Freeze the historical payload ceiling (24-byte Cell header plus 4-byte tag).
    // New Storage framing does not expand admitted inline artifact authority.
    bytes: 2 * MAX_WASM - 28,
    allocation: ARTIFACT_BYTES * 9 + canic_management_protocol::stored::LIMITS.allocation,
    nodes: canic_management_protocol::stored::LIMITS.nodes + 64,
    depth: canic_management_protocol::stored::LIMITS.depth + 3,
};

#[derive(Clone, Default, Deserialize, Serialize)]
pub(super) struct Journal(pub Status, pub upgrade::Status, pub Vec<upgrade::Status>);
#[cfg(feature = "role-issuer")]
impl Journal {
    pub(super) fn ready_for_role(&self) -> bool {
        self.0.phase == canic_management_protocol::Phase::Ready
            && self.0.failure.is_none()
            && self.1.failure.is_none()
            && matches!(self.1.phase, upgrade::Phase::Empty | upgrade::Phase::Ready)
    }
}

#[derive(Clone, Default)]
pub(super) struct Inline(pub Vec<u8>, pub Vec<u8>);
impl Serialize for Inline {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let creation = BytesRef::<MAX_WASM>::new(&self.0).map_err(serde::ser::Error::custom)?;
        let upgrade = BytesRef::<MAX_WASM>::new(&self.1).map_err(serde::ser::Error::custom)?;
        (creation, upgrade).serialize(serializer)
    }
}
impl<'de> Deserialize<'de> for Inline {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let (creation, upgrade) = <(Bytes<MAX_WASM>, Bytes<MAX_WASM>)>::deserialize(deserializer)?;
        Ok(Self(creation.into_vec(), upgrade.into_vec()))
    }
}
impl Inline {
    fn decode(bytes: Cow<'_, [u8]>) -> Result<Self, &'static str> {
        if let Some(payload) = bytes.strip_prefix(ARTIFACT_TAG) {
            let length: [u8; 4] = payload
                .get(..4)
                .ok_or("artifact header")?
                .try_into()
                .map_err(|_| "artifact header")?;
            let length = u32::from_le_bytes(length) as usize;
            let payload = &payload[4..];
            if length > MAX_WASM || length > payload.len() || payload.len() - length > MAX_WASM {
                return Err("artifact bounds");
            }
            Ok(Self(payload[..length].to_vec(), payload[length..].to_vec()))
        } else if bytes.len() <= MAX_WASM
            && (bytes.is_empty() || bytes.starts_with(b"\0asm\x01\0\0\0"))
        {
            Ok(Self(bytes.into_owned(), vec![]))
        } else {
            Err("creation artifact migration")
        }
    }
}

// New collections encode payloads directly; nested historical tags are read-only.
// This avoids adding framing bytes to a predecessor that filled its checked lease.
#[derive(Clone, Deserialize, Serialize)]
pub(super) enum Artifact {
    Inline(Inline),
    Stored(canic_management_protocol::stored::Sources),
    Application(
        canic_management_protocol::stored::Sources,
        canic_management_protocol::application::Lifecycle,
    ),
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
            Self::Stored(value) | Self::Application(value, _) => Some(value),
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
            Self::Stored(_) | Self::Application(..) => panic!("Store bytes never belong to Root"),
        }
    }

    pub(super) fn hash(&self, upgrade: bool) -> Option<[u8; 32]> {
        match self {
            Self::Inline(_) => {
                let bytes = self.wasm(upgrade);
                (!bytes.is_empty()).then(|| canic_management_protocol::digest(bytes))
            }
            Self::Stored(value) | Self::Application(value, _) => if upgrade {
                value.upgrades.last()
            } else {
                Some(&value.creation)
            }
            .map(|source| source.reference.manifest.hash),
        }
    }

    fn decode(bytes: Cow<'_, [u8]>) -> Result<Self, &'static str> {
        if let Some(payload) = bytes.strip_prefix(b"CUS1") {
            let value: canic_management_protocol::stored::Sources = ic_wire::decode(
                payload,
                ic_wire::Encoding::Payload,
                canic_management_protocol::stored::LIMITS,
            )
            .map_err(|_| "Store custody")?;
            if value.upgrades.len() > upgrade::MAX_UPGRADES {
                return Err("Store history bound");
            }
            Ok(Self::Stored(value))
        } else {
            Inline::decode(bytes).map(Self::Inline)
        }
    }
}

// Only this codec maps historical array positions to runtime child identities.
fn bind<T>([first, second]: [T; 2]) -> BTreeMap<ChildId, T> {
    BTreeMap::from([(ChildId::FIRST, first), (ChildId::SECOND, second)])
}

// All children share the original checked leases; each retains separate payloads.
#[derive(Clone)]
pub(super) struct Journals(
    pub BTreeMap<ChildId, Journal>,
    pub Option<store_maintenance::Status>,
    pub Vec<store_maintenance::Status>,
);
impl Default for Journals {
    fn default() -> Self {
        Self(
            ChildId::ALL
                .into_iter()
                .map(|id| (id, Journal::default()))
                .collect(),
            None,
            vec![],
        )
    }
}
impl Journals {
    fn validate(&self) -> Result<(), &'static str> {
        if !complete(&self.0) {
            return Err("child journal identities");
        }
        if self
            .0
            .values()
            .any(|journal| journal.2.len() >= upgrade::MAX_UPGRADES)
        {
            return Err("upgrade history bound");
        }
        if self.2.len() >= store_maintenance::MAX_UPGRADES {
            return Err("Store maintenance history bound");
        }
        Ok(())
    }
}
impl Journals {
    pub(super) fn to_bytes(&self) -> Result<Cow<'_, [u8]>, &'static str> {
        self.validate()?;
        let entries: Vec<_> = self
            .0
            .iter()
            .map(|(slot, value)| (slot.index() as u8, value))
            .collect();
        let payload = ic_wire::encode(
            &(entries, &self.1, &self.2),
            ic_wire::Encoding::Payload,
            JOURNAL_LIMITS,
        )
        .map_err(|_| "child journals")?;
        let mut bytes = JOURNAL_TAG.to_vec();
        bytes.extend_from_slice(&payload);
        Ok(Cow::Owned(bytes))
    }

    pub(super) fn from_bytes(bytes: &[u8]) -> Result<Self, &'static str> {
        let payload = bytes
            .strip_prefix(JOURNAL_TAG)
            .ok_or("unsupported Root journal format")?;
        let (entries, maintenance, history): (Records<Journal>, _, _) =
            ic_wire::decode(payload, ic_wire::Encoding::Payload, JOURNAL_LIMITS)
                .map_err(|_| "identified child journals")?;
        let value = Self(identified(entries)?, maintenance, history);
        value.validate()?;
        Ok(value)
    }
}
#[derive(Clone)]
pub(super) struct Artifacts(pub BTreeMap<ChildId, Artifact>);
impl Default for Artifacts {
    fn default() -> Self {
        Self(
            ChildId::ALL
                .into_iter()
                .map(|id| (id, Artifact::default()))
                .collect(),
        )
    }
}
impl Artifacts {
    fn limits(expanded_layout: bool) -> ic_wire::Limits {
        if expanded_layout {
            ic_wire::Limits {
                bytes: ARTIFACT_LIMITS.bytes + SLOT_FRAMING,
                ..ARTIFACT_LIMITS
            }
        } else {
            ARTIFACT_LIMITS
        }
    }

    pub(super) fn fits(&self) -> bool {
        self.entries().is_ok_and(|entries| {
            ic_wire::encoded_len(
                &entries,
                ic_wire::Encoding::Payload,
                Self::limits(expanded(&self.0)),
            )
            .is_ok()
        })
    }

    fn entries(&self) -> Result<Vec<(u8, &Artifact)>, &'static str> {
        let inline_bytes: usize = self
            .0
            .values()
            .map(|artifact| match artifact {
                Artifact::Inline(value) => value.0.len() + value.1.len(),
                Artifact::Stored(_) | Artifact::Application(..) => 0,
            })
            .sum();
        if !historical_or_complete(&self.0)
            || inline_bytes > canic_management_protocol::children::MAX_ARTIFACT_BYTES
            || self
                .0
                .values()
                .filter_map(Artifact::sources)
                .any(|source| source.upgrades.len() > upgrade::MAX_UPGRADES)
            || self.0.values().any(|artifact| match artifact {
                Artifact::Application(sources, lifecycle) => {
                    lifecycle.creation.as_slice().is_empty()
                        || lifecycle.upgrades.len() != sources.upgrades.len()
                        || lifecycle.upgrades.iter().any(|m| !m.valid())
                }
                _ => false,
            })
        {
            return Err("aggregate artifact budget");
        }
        Ok(self
            .0
            .iter()
            .map(|(slot, artifact)| (slot.index() as u8, artifact))
            .collect())
    }
}
impl Artifacts {
    pub(super) fn to_bytes(&self) -> Result<Cow<'_, [u8]>, &'static str> {
        let mut bytes = if expanded(&self.0) {
            b"CUA7"
        } else if self.0.len() > 2 {
            b"CUA6"
        } else if self
            .0
            .values()
            .any(|a| matches!(a, Artifact::Application(..)))
        {
            b"CUA5"
        } else {
            b"CUA4"
        }
        .to_vec();
        bytes.extend(
            ic_wire::encode(
                &self.entries()?,
                ic_wire::Encoding::Payload,
                Self::limits(expanded(&self.0)),
            )
            .map_err(|_| "identified child artifacts")?,
        );
        Ok(Cow::Owned(bytes))
    }

    pub(super) fn from_bytes(bytes: Cow<'_, [u8]>) -> Result<Self, &'static str> {
        let historical = bytes.starts_with(b"CUA4");
        let value =
            if let Some(payload) = bytes
                .strip_prefix(b"CUA7")
                .or_else(|| bytes.strip_prefix(b"CUA6"))
                .or_else(|| bytes.strip_prefix(b"CUA5"))
                .or_else(|| bytes.strip_prefix(b"CUA4"))
            {
                let limits = Self::limits(bytes.starts_with(b"CUA7"));
                let entries: Records<Artifact> =
                    ic_wire::decode(payload, ic_wire::Encoding::Payload, limits)
                        .map_err(|_| "identified child artifacts")?;
                let expected = (!bytes.starts_with(b"CUA7"))
                    .then_some(if bytes.starts_with(b"CUA6") { 4 } else { 2 });
                let entries = identified(entries)?;
                if expected.is_some_and(|count| entries.len() != count)
                    || expanded(&entries) != expected.is_none()
                {
                    return Err("child identities");
                }
                Ok::<_, &'static str>(Self(entries))
            } else if let Some(payload) = bytes.strip_prefix(b"CUA3") {
                let length: [u8; 4] = payload
                    .get(..4)
                    .ok_or("child artifact header")?
                    .try_into()
                    .map_err(|_| "child artifact header")?;
                let length = u32::from_le_bytes(length) as usize;
                let payload = &payload[4..];
                if length > payload.len() {
                    return Err("child artifact bounds");
                }
                Ok(Self(bind([
                    Artifact::decode(Cow::Borrowed(&payload[..length]))?,
                    Artifact::decode(Cow::Borrowed(&payload[length..]))?,
                ])))
            } else {
                Ok(Self(bind([Artifact::decode(bytes)?, Artifact::default()])))
            }?;
        if historical
            && value
                .0
                .values()
                .any(|a| matches!(a, Artifact::Application(..)))
        {
            return Err("application arguments require CUA5");
        }
        if !value.fits() {
            return Err("aggregate artifact budget");
        }
        Ok(value)
    }
}
