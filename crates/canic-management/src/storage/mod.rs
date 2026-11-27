//! Independent stable child records and bounded fleet metadata.

mod history;
pub(crate) use history::Paged;
#[cfg(test)]
mod tests;

use canic_management_protocol::{LIMITS, Status, children::ChildId, store_maintenance, upgrade};

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

thread_local! { static REVISION: std::cell::Cell<u64> = const { std::cell::Cell::new(0) }; }
pub(crate) fn revision() -> u64 {
    REVISION.get()
}

pub(super) fn complete<T>(entries: &BTreeMap<ChildId, T>) -> bool {
    entries
        .keys()
        .filter_map(|id| id.service_index())
        .enumerate()
        .all(|(expected, actual)| expected == actual)
}

pub(super) const JOURNAL_BYTES: usize = 65_536;
pub(super) const ARTIFACT_BYTES: usize = 1_048_576;
pub(super) const JOURNAL_KEY: &str = "manager.child_journal_pages.v1";
pub(super) const ARTIFACT_KEY: &str = "manager.child_artifact_pages.v1";
pub(super) const META_KEY: &str = "manager.metadata_pages.v1";
#[cfg(feature = "sharding")]
pub(super) const SHARDS_KEY: &str = "manager.placement_rows.v1";
#[cfg(feature = "sharding")]
pub(super) const WORKER_HISTORY_KEY: &str = "manager.worker_history.v1";
#[cfg(feature = "auth")]
pub(super) const AUTH_HISTORY_KEY: &str = "manager.auth_history.v1";
pub(super) const KEYS: &[&str] = &[
    #[cfg(feature = "role-issuer")]
    super::role_issuer::KEY,
    #[cfg(feature = "auth")]
    AUTH_HISTORY_KEY,
    META_KEY,
    JOURNAL_KEY,
    ARTIFACT_KEY,
    #[cfg(feature = "sharding")]
    SHARDS_KEY,
    #[cfg(feature = "sharding")]
    WORKER_HISTORY_KEY,
    crate::backup::KEY,
];
const JOURNAL_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: JOURNAL_BYTES - 4,
    // Wire charges visits to decoded Rust values in addition to encoded bytes.
    allocation: JOURNAL_BYTES * 1024,
    nodes: JOURNAL_BYTES * 8,
    depth: LIMITS.depth + 5,
};
const ARTIFACT_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: ARTIFACT_BYTES - 4,
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

#[derive(Clone, Default, Serialize, Deserialize)]
pub(super) struct Metadata {
    pub maintenance: Option<store_maintenance::Status>,
    pub history: Vec<store_maintenance::Status>,
    pub bootstrap: Option<canic_management_protocol::stored::RootBootstrap>,
}

pub(super) trait Record: Serialize + serde::de::DeserializeOwned {
    const TAG: &'static [u8; 4];
    const LIMITS: ic_wire::Limits;
    fn valid(&self) -> bool;

    fn encode(&self) -> Result<Vec<u8>, &'static str> {
        if !self.valid() {
            return Err("invalid management record");
        }
        let payload = ic_wire::encode(self, ic_wire::Encoding::Payload, Self::LIMITS)
            .map_err(|_| "management record budget")?;
        let mut bytes = Self::TAG.to_vec();
        bytes.extend(payload);
        Ok(bytes)
    }

    fn decode(bytes: &[u8]) -> Result<Self, &'static str> {
        let payload = bytes
            .strip_prefix(Self::TAG)
            .ok_or("unsupported management record format")?;
        let value: Self = ic_wire::decode(payload, ic_wire::Encoding::Payload, Self::LIMITS)
            .map_err(|_| "invalid management record encoding")?;
        if !value.valid() {
            return Err("invalid management record");
        }
        Ok(value)
    }
}

impl Record for Metadata {
    const TAG: &'static [u8; 4] = b"CMET";
    const LIMITS: ic_wire::Limits = JOURNAL_LIMITS;
    fn valid(&self) -> bool {
        self.bootstrap.is_none_or(|policy| policy.valid())
    }
}

impl Record for Journal {
    const TAG: &'static [u8; 4] = b"CJRW";
    const LIMITS: ic_wire::Limits = JOURNAL_LIMITS;
    fn valid(&self) -> bool {
        true
    }
}

impl Record for Artifact {
    const TAG: &'static [u8; 4] = b"CARW";
    const LIMITS: ic_wire::Limits = ARTIFACT_LIMITS;
    fn valid(&self) -> bool {
        match self {
            Self::Application(sources, lifecycle) => {
                !lifecycle.creation.as_slice().is_empty()
                    && lifecycle.upgrades.len() == sources.upgrades.len()
                    && lifecycle
                        .upgrades
                        .iter()
                        .all(|maintenance| maintenance.valid())
            }
            _ => true,
        }
    }
}

pub(super) struct ChildRows<T> {
    rows: BTreeMap<ChildId, T>,
    stable: ic_storage::adapter::StableBTreeMap<(u32, u64), Vec<u8>>,
}

impl<T: Paged> ChildRows<T> {
    pub fn rows(&self) -> &BTreeMap<ChildId, T> {
        &self.rows
    }

    pub fn open(
        runtime: &ic_storage::MemoryRuntime<ic_storage::DefaultMemoryImpl>,
        key: &str,
        fresh: bool,
    ) -> Self {
        let stable: ic_storage::adapter::StableBTreeMap<(u32, u64), Vec<u8>> =
            canic::storage::map(runtime, key, fresh);
        let mut rows: BTreeMap<ChildId, T> = BTreeMap::new();
        let mut expected = 0;
        for entry in stable.iter() {
            let &(slot, ordinal) = entry.key();
            let id = ChildId::from_raw(slot);
            let bytes = entry.value();
            if ordinal == 0 {
                assert_eq!(expected, 0, "missing upgrade records");
                let (count, head) = bytes.split_at(8);
                expected = u64::from_le_bytes(count.try_into().unwrap());
                rows.insert(
                    id,
                    T::decode(head)
                        .unwrap_or_else(|reason| ic_platform_runtime::system::trap(reason)),
                );
            } else {
                let row = rows.get_mut(&id).expect("retained child head");
                assert_eq!(ordinal, row.len() as u64 + 1, "upgrade record order");
                expected = expected.checked_sub(1).expect("extra upgrade record");
                row.append(&bytes).expect("retained upgrade record");
            }
        }
        assert_eq!(expected, 0, "missing final upgrade records");
        assert!(complete(&rows), "child identities");
        Self { rows, stable }
    }

    pub fn set(&mut self, slot: ChildId, value: T) -> Result<(), &'static str> {
        let old_len = self.rows.get(&slot).map_or(0, Paged::len);
        write_pages(&mut self.stable, slot.index() as u32, old_len, &value)?;
        self.rows.insert(slot, value);
        REVISION.set(revision().checked_add(1).expect("custody revision"));
        Ok(())
    }
}

pub(super) fn write_pages<T: Paged>(
    stable: &mut ic_storage::adapter::StableBTreeMap<(u32, u64), Vec<u8>>,
    slot: u32,
    old_len: usize,
    value: &T,
) -> Result<(), &'static str> {
    let n = value.len();
    let mut head = (n as u64).to_le_bytes().to_vec();
    head.extend(value.head().encode()?);
    // Completed predecessors are immutable. The latest delivery may settle in place.
    let start = if n == old_len {
        n.saturating_sub(1)
    } else {
        old_len.min(n)
    };
    let pages = (start..n)
        .map(|i| value.entry(i))
        .collect::<Result<Vec<_>, _>>()?;
    for (i, bytes) in (start..n).zip(pages) {
        stable.insert((slot, i as u64 + 1), bytes);
    }
    for i in n..old_len {
        stable.remove(&(slot, i as u64 + 1));
    }
    stable.insert((slot, 0), head);
    Ok(())
}
