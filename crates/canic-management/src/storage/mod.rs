//! Versioned payloads in two bounded Storage records.
//! Open accepts only the current journal and Store delivery tags.

#[cfg(test)]
mod tests;

use canic_management_protocol::{LIMITS, Status, children::ChildId, store_maintenance, upgrade};

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// Persist identities explicitly. The protocol remains the admission ceiling;
// accepting a new record shape never creates capacity or drops unknown entries.
type Records<T> = Vec<(u32, T)>;

// Require unique, ordered identities before collecting into the runtime map.
fn identified<T>(entries: Records<T>) -> Result<BTreeMap<ChildId, T>, &'static str> {
    let mut map = BTreeMap::new();
    for (id, value) in entries {
        let slot = ChildId::placement(id as usize)
            .or_else(|| ChildId::new(id as usize))
            .ok_or("child identity bound")?;
        if map.last_key_value().is_some_and(|(last, _)| *last >= slot) {
            return Err("child identity order");
        }
        map.insert(slot, value);
    }
    if !complete(&map) {
        return Err("child identities");
    }
    Ok(map)
}

fn complete<T>(entries: &BTreeMap<ChildId, T>) -> bool {
    entries
        .keys()
        .filter_map(|id| id.service_index())
        .enumerate()
        .all(|(expected, actual)| expected == actual)
        && ChildId::HISTORICAL_SERVICES
            .iter()
            .all(|id| entries.contains_key(id))
        && entries.contains_key(&ChildId::PROJECT[0]) == entries.contains_key(&ChildId::PROJECT[1])
}

const JOURNAL_TAG: &[u8; 4] = b"CMJ1";
const ARTIFACT_TAG: &[u8; 4] = b"CMS1";
pub(super) const JOURNAL_BYTES: usize = 65_536;
pub(super) const ARTIFACT_BYTES: usize = 1_048_576;
pub(super) const JOURNAL_KEY: &str = "manager.journal.v2";
pub(super) const ARTIFACT_KEY: &str = "manager.artifacts.v1";
#[cfg(feature = "sharding")]
pub(super) const SHARDS_KEY: &str = "manager.shards.v2";
#[cfg(feature = "sharding")]
pub(super) const WORKER_HISTORY_KEY: &str = "manager.worker_history.v1";
#[cfg(feature = "auth")]
pub(super) const AUTH_HISTORY_KEY: &str = "manager.auth_history.v1";
pub(super) const KEYS: &[&str] = &[
    #[cfg(feature = "role-issuer")]
    super::role_issuer::KEY,
    #[cfg(feature = "auth")]
    AUTH_HISTORY_KEY,
    JOURNAL_KEY,
    ARTIFACT_KEY,
    #[cfg(feature = "sharding")]
    SHARDS_KEY,
    #[cfg(feature = "sharding")]
    WORKER_HISTORY_KEY,
    "manager.snapshot.v1",
];
const JOURNAL_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: JOURNAL_BYTES - 4,
    // Wire charges visits to decoded Rust values in addition to encoded bytes.
    allocation: JOURNAL_BYTES * 1024,
    nodes: JOURNAL_BYTES * 8,
    depth: LIMITS.depth + 5,
};
const ARTIFACT_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: ARTIFACT_BYTES - ARTIFACT_TAG.len(),
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

#[derive(Clone, Default, Deserialize, Serialize)]
pub(super) enum Artifact {
    #[default]
    Empty,
    Stored(canic_management_protocol::stored::Sources),
    Application(
        canic_management_protocol::stored::Sources,
        canic_management_protocol::application::Lifecycle,
    ),
}
impl Artifact {
    pub(super) fn sources(&self) -> Option<&canic_management_protocol::stored::Sources> {
        match self {
            Self::Empty => None,
            Self::Stored(value) | Self::Application(value, _) => Some(value),
        }
    }

    pub(super) fn hash(&self, upgrade: bool) -> Option<[u8; 32]> {
        match self {
            Self::Empty => None,
            Self::Stored(value) | Self::Application(value, _) => if upgrade {
                value.upgrades.last()
            } else {
                Some(&value.creation)
            }
            .map(|source| source.reference.manifest.hash),
        }
    }
}

// All children share the original checked leases; each retains separate payloads.
#[derive(Clone)]
pub(super) struct Journals(
    pub BTreeMap<ChildId, Journal>,
    pub Option<store_maintenance::Status>,
    pub Vec<store_maintenance::Status>,
    pub Option<canic_management_protocol::stored::RootBootstrap>,
);
impl Default for Journals {
    fn default() -> Self {
        Self(
            ChildId::HISTORICAL_SERVICES
                .into_iter()
                .map(|id| (id, Journal::default()))
                .collect(),
            None,
            vec![],
            None,
        )
    }
}
impl Journals {
    fn validate(&self) -> Result<(), &'static str> {
        if self.3.is_some_and(|policy| !policy.valid()) || !complete(&self.0) {
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
    pub(super) fn to_bytes(&self) -> Result<Vec<u8>, &'static str> {
        self.validate()?;
        let entries: Vec<_> = self
            .0
            .iter()
            .map(|(slot, value)| (slot.index() as u32, value))
            .collect();
        let payload = ic_wire::encode(
            &(entries, &self.1, &self.2, self.3),
            ic_wire::Encoding::Payload,
            JOURNAL_LIMITS,
        )
        .map_err(|_| "child journals")?;
        let mut bytes = JOURNAL_TAG.to_vec();
        bytes.extend_from_slice(&payload);
        Ok(bytes)
    }

    pub(super) fn from_bytes(bytes: &[u8]) -> Result<Self, &'static str> {
        let payload = bytes
            .strip_prefix(JOURNAL_TAG)
            .ok_or("unsupported Root journal format")?;
        let (entries, maintenance, history, bootstrap): (Records<Journal>, _, _, _) =
            ic_wire::decode(payload, ic_wire::Encoding::Payload, JOURNAL_LIMITS)
                .map_err(|_| "identified child journals")?;
        let value = Self(identified(entries)?, maintenance, history, bootstrap);
        value.validate()?;
        Ok(value)
    }
}
#[derive(Clone)]
pub(super) struct Artifacts(pub BTreeMap<ChildId, Artifact>);
impl Default for Artifacts {
    fn default() -> Self {
        Self(
            ChildId::HISTORICAL_SERVICES
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

    fn entries(&self) -> Result<Vec<(u32, &Artifact)>, &'static str> {
        if !complete(&self.0)
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
            .map(|(slot, artifact)| (slot.index() as u32, artifact))
            .collect())
    }
}
impl Artifacts {
    pub(super) fn to_bytes(&self) -> Result<Vec<u8>, &'static str> {
        let mut bytes = ARTIFACT_TAG.to_vec();
        bytes.extend(
            ic_wire::encode(
                &self.entries()?,
                ic_wire::Encoding::Payload,
                ARTIFACT_LIMITS,
            )
            .map_err(|_| "identified child artifacts")?,
        );
        Ok(bytes)
    }

    pub(super) fn from_bytes(bytes: &[u8]) -> Result<Self, &'static str> {
        let payload = bytes
            .strip_prefix(ARTIFACT_TAG)
            .ok_or("unsupported Root artifact format")?;
        let entries: Records<Artifact> =
            ic_wire::decode(payload, ic_wire::Encoding::Payload, ARTIFACT_LIMITS)
                .map_err(|_| "identified child artifacts")?;
        let value = Self(identified(entries)?);
        if !value.fits() {
            return Err("aggregate artifact budget");
        }
        Ok(value)
    }
}
