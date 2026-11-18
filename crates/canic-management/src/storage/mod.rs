//! Versioned payloads in two bounded Storage records.
//! Logical predecessor codecs do not import an older Storage backing format.
use canic_management_protocol::{
    LIMITS, MAX_WASM, Status, children::ChildId, store_maintenance, upgrade,
};

use ic_wire::{Bytes, BytesRef, List};
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, collections::BTreeMap};

#[cfg(test)]
mod tests;

// Persist identities explicitly. The protocol remains the admission ceiling;
// accepting a new record shape never creates capacity or drops unknown entries.
type Records<T> = List<(u8, T), { ChildId::ALL.len() }>;

fn identified<T>(entries: Records<T>) -> Result<BTreeMap<ChildId, T>, &'static str> {
    let entries = entries.into_vec();
    if entries.len() != ChildId::ALL.len() {
        return Err("child identities");
    }
    entries
        .into_iter()
        .enumerate()
        .map(|(index, (id, value))| {
            let slot = ChildId::new(usize::from(id))
                .filter(|slot| slot.index() == index)
                .ok_or("child identity order")?;
            Ok((slot, value))
        })
        .collect()
}

fn complete<T>(entries: &BTreeMap<ChildId, T>) -> bool {
    entries.keys().copied().eq(ChildId::ALL)
}

const JOURNAL_TAG: &[u8] = b"CUJ3";
const ARTIFACT_TAG: &[u8] = b"CUA2";
pub(super) const JOURNAL_BYTES: usize = 16_384;
pub(super) const ARTIFACT_BYTES: usize = 2 * MAX_WASM + 24;
pub(super) const JOURNAL_KEY: &str = "manager.journal.v2";
pub(super) const ARTIFACT_KEY: &str = "manager.artifacts.v1";
#[cfg(feature = "sharding")]
pub(super) const SHARDS_KEY: &str = "manager.shards.v1";
pub(super) const KEYS: &[&str] = &[
    JOURNAL_KEY,
    ARTIFACT_KEY,
    #[cfg(feature = "sharding")]
    SHARDS_KEY,
    #[cfg(feature = "backup")]
    "manager.snapshot.v1",
];
const JOURNAL_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: JOURNAL_BYTES - 4,
    allocation: 2 * (1 + upgrade::MAX_UPGRADES) * LIMITS.allocation
        + store_maintenance::MAX_UPGRADES * store_maintenance::LIMITS.allocation,
    nodes: 2 * (1 + upgrade::MAX_UPGRADES) * LIMITS.nodes
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
impl Journal {
    fn decode(bytes: &[u8]) -> Result<Self, &'static str> {
        if let Some(payload) = bytes.strip_prefix(JOURNAL_TAG) {
            let (creation, upgrade, history) =
                ic_wire::decode::<(Status, upgrade::Status, Vec<upgrade::Status>)>(
                    payload,
                    ic_wire::Encoding::Payload,
                    JOURNAL_LIMITS,
                )
                .map_err(|_| "upgrade journal")?;
            if history.len() >= upgrade::MAX_UPGRADES {
                return Err("upgrade history bound");
            }
            Ok(Self(creation, upgrade, history))
        } else if let Some(payload) = bytes.strip_prefix(b"CUJ2") {
            let (creation, upgrade) =
                ic_wire::decode(payload, ic_wire::Encoding::Payload, JOURNAL_LIMITS)
                    .map_err(|_| "single upgrade journal migration")?;
            Ok(Self(creation, upgrade, vec![]))
        } else {
            // The sole historical schema is the original bounded creation status.
            // Lifecycle validates both stores before committing the migration.
            ic_wire::decode(bytes, ic_wire::Encoding::Payload, LIMITS)
                .map(|creation| Self(creation, upgrade::Status::default(), vec![]))
                .map_err(|_| "creation journal migration")
        }
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
        let mut bytes = b"CUJ9".to_vec();
        bytes.extend_from_slice(&payload);
        Ok(Cow::Owned(bytes))
    }
    pub(super) fn from_bytes(bytes: &[u8]) -> Result<Self, &'static str> {
        let value = if let Some(payload) = bytes
            .strip_prefix(b"CUJ9")
            .or_else(|| bytes.strip_prefix(b"CUJ8"))
        {
            let (entries, maintenance, history): (Records<Journal>, _, _) =
                ic_wire::decode(payload, ic_wire::Encoding::Payload, JOURNAL_LIMITS)
                    .map_err(|_| "identified child journals")?;
            Ok(Self(identified(entries)?, maintenance, history))
        } else if let Some(payload) = bytes.strip_prefix(b"CUJ7") {
            let (journals, maintenance, history) =
                ic_wire::decode(payload, ic_wire::Encoding::Payload, JOURNAL_LIMITS)
                    .map_err(|_| "Store maintenance history")?;
            Ok(Self(bind(journals), maintenance, history))
        } else if let Some(payload) = bytes.strip_prefix(b"CUJ6") {
            let (journals, maintenance) =
                ic_wire::decode(payload, ic_wire::Encoding::Payload, JOURNAL_LIMITS)
                    .map_err(|_| "Store maintenance journal")?;
            Ok(Self(bind(journals), maintenance, vec![]))
        } else if let Some(payload) = bytes.strip_prefix(b"CUJ5") {
            // Wire records have exact arity: decode the historical shape explicitly.
            #[derive(Deserialize)]
            struct Previous {
                approval: store_maintenance::Approval,
                operator: String,
                phase: store_maintenance::Phase,
                stop_request: Option<[u8; 32]>,
                failure: Option<canic_management_protocol::Failure>,
            }
            let (journals, maintenance): ([Journal; 2], Option<Previous>) =
                ic_wire::decode(payload, ic_wire::Encoding::Payload, JOURNAL_LIMITS)
                    .map_err(|_| "previous Store maintenance journal")?;
            Ok(Self(
                bind(journals),
                maintenance.map(|old| store_maintenance::Status {
                    approval: old.approval,
                    operator: old.operator,
                    phase: old.phase,
                    stop_request: old.stop_request,
                    failure: old.failure,
                    preservation: store_maintenance::Scan::Legacy(None),
                    installation: None,
                }),
                vec![],
            ))
        } else if let Some(payload) = bytes.strip_prefix(b"CUJ4") {
            let journals: [Journal; 2] =
                ic_wire::decode(payload, ic_wire::Encoding::Payload, JOURNAL_LIMITS)
                    .map_err(|_| "child journals")?;
            Ok(Self(bind(journals), None, vec![]))
        } else {
            Ok(Self(
                bind([Journal::decode(bytes)?, Journal::default()]),
                None,
                vec![],
            ))
        }?;
        if !bytes.starts_with(b"CUJ9")
            && value.1.iter().chain(value.2.iter()).any(|record| {
                matches!(record.preservation, store_maintenance::Scan::Entries(_))
                    || record.installation.as_ref().is_some_and(|installation| {
                        matches!(
                            installation.verification,
                            store_maintenance::Scan::Entries(_)
                        )
                    })
            })
        {
            return Err("versioned scan in historical journal");
        }
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
    pub(super) fn fits(&self) -> bool {
        self.entries().is_ok_and(|entries| {
            ic_wire::encoded_len(&entries, ic_wire::Encoding::Payload, ARTIFACT_LIMITS).is_ok()
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
        if !complete(&self.0)
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
        let mut bytes = if self
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
                ARTIFACT_LIMITS,
            )
            .map_err(|_| "identified child artifacts")?,
        );
        Ok(Cow::Owned(bytes))
    }
    pub(super) fn from_bytes(bytes: Cow<'_, [u8]>) -> Result<Self, &'static str> {
        let historical = bytes.starts_with(b"CUA4");
        let value = if let Some(payload) = bytes
            .strip_prefix(b"CUA5")
            .or_else(|| bytes.strip_prefix(b"CUA4"))
        {
            let entries: Records<Artifact> =
                ic_wire::decode(payload, ic_wire::Encoding::Payload, ARTIFACT_LIMITS)
                    .map_err(|_| "identified child artifacts")?;
            Ok(Self(identified(entries)?))
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
