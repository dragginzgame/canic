//! Canister Directory
//!
//! Purpose
//! - Directory is a read-model of installed canisters grouped by `CanisterType`.
//! - On root, the directory is not the source of truth and is generated from the
//!   `CanisterRegistry` on demand.
//! - On children, a local copy is stored to enable fast reads without cross-canister calls.
//!
//! Lifecycle
//! - Root generates a fresh view from the registry and cascades it after installs/updates.
//! - Children accept a full re-import of the directory view via the cascade endpoint.
//! - There are no partial mutations: the only write API is `import(view)`.
//!
//! Invariants
//! - Root directory view must equal `generate_from_registry()`.
//! - Child directory view should align with root’s generated view after cascade.
//!
use crate::{
    Error,
    cdk::structures::{BTreeMap, DefaultMemoryImpl, Memory, memory::VirtualMemory},
    icu_register_memory,
    memory::{CanisterEntry, MemoryError, SUBNET_DIRECTORY_MEMORY_ID, subnet::SubnetError},
    types::CanisterType,
};
use std::cell::RefCell;

//
// SUBNET_DIRECTORY
//

thread_local! {
    static SUBNET_DIRECTORY: RefCell<SubnetDirectoryCore<VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(SubnetDirectoryCore::new(BTreeMap::init(
            icu_register_memory!(SUBNET_DIRECTORY_MEMORY_ID),
        )));
}

///
/// SubnetDirectory
///

pub type SubnetDirectoryView = Vec<(CanisterType, CanisterEntry)>;

pub struct SubnetDirectory;

impl SubnetDirectory {
    #[must_use]
    pub fn get(ty: &CanisterType) -> Option<CanisterEntry> {
        SUBNET_DIRECTORY.with_borrow(|core| core.get(ty))
    }

    pub fn try_get(ty: &CanisterType) -> Result<CanisterEntry, Error> {
        Self::get(ty).ok_or_else(|| MemoryError::from(SubnetError::TypeNotFound(ty.clone())).into())
    }

    pub fn try_get_root() -> Result<CanisterEntry, Error> {
        Self::try_get(&CanisterType::ROOT)
    }

    /// Export current state
    pub(super) fn export() -> SubnetDirectoryView {
        SUBNET_DIRECTORY.with_borrow(SubnetDirectoryCore::export)
    }

    /// Import state (replace everything)
    pub fn import(view: SubnetDirectoryView) {
        SUBNET_DIRECTORY.with_borrow_mut(|core| core.import(view));
    }
}

///
/// SubnetDirectoryCore
///

pub struct SubnetDirectoryCore<M: Memory> {
    map: BTreeMap<CanisterType, CanisterEntry, M>,
}

impl<M: Memory> SubnetDirectoryCore<M> {
    pub const fn new(map: BTreeMap<CanisterType, CanisterEntry, M>) -> Self {
        Self { map }
    }

    pub fn get(&self, ty: &CanisterType) -> Option<CanisterEntry> {
        self.map.get(ty)
    }

    pub fn import(&mut self, entries: SubnetDirectoryView) {
        self.map.clear();
        for (ty, entry) in entries {
            self.map.insert(ty, entry);
        }
    }

    pub fn export(&self) -> SubnetDirectoryView {
        self.map.to_vec()
    }
}
