//! Explicit topology for a newly created shard. The pool approval owns these bytes.
pub mod directory;

use ic_platform_types::Principal;
use serde::{Deserialize, Serialize};

#[cfg(feature = "storage")]
pub use actor::{attach_parent, parent, require_parent};

/// Check the actual caller against a locally retained topology authority.
/// # Errors
/// Refuses an invalid authority or a call from another principal.
pub fn require_sender(expected: Principal) -> Result<(), &'static str> {
    if expected.as_slice().last() == Some(&1) && expected == ic_platform_runtime::system::caller() {
        Ok(())
    } else {
        Err("unexpected topology sender")
    }
}

const FRAME: ic_wire::Encoding = ic_wire::Encoding::Frame {
    method: ic_wire::method_id("canic.shard_install.v1"),
    direction: ic_wire::Direction::Request,
};
const LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 4096,
    allocation: 65536,
    nodes: 8192,
    depth: 8,
};

/// Passive install frame, independent of receiver enrollment.
#[derive(Debug, Serialize, Deserialize)]
pub struct Install {
    /// Installing management owner.
    pub root: Principal,
    /// Application's approved immediate parent.
    pub parent: Principal,
    /// Inner application arguments, e.g. a nonce.
    pub application: Vec<u8>,
}
impl Install {
    fn validate(&self) -> Result<(), &'static str> {
        if self.root == self.parent
            || self.root.as_slice().last() != Some(&1)
            || self.parent.as_slice().last() != Some(&1)
        {
            return Err("expected distinct Root and parent canisters");
        }
        Ok(())
    }

    #[cfg(feature = "storage")]
    fn installing_parent(&self) -> Principal {
        self.validate().expect("shard topology");
        if require_sender(self.root).is_err() {
            ic_platform_runtime::system::trap("wrong shard installer");
        }
        assert_ne!(
            self.parent,
            ic_platform_runtime::system::canister_self(),
            "parent must differ from shard"
        );
        self.parent
    }

    /// Encode a bounded, structurally valid install frame.
    /// # Errors
    /// Refuses invalid bindings or excessive application bytes.
    pub fn encode(&self) -> Result<Vec<u8>, &'static str> {
        self.validate()?;
        ic_wire::encode(self, FRAME, LIMITS).map_err(|_| "shard install frame")
    }

    /// Decode and check the context required by an opted-in shard.
    /// # Errors
    /// Refuses absent or malformed bindings.
    pub fn decode(bytes: &[u8]) -> Result<Self, &'static str> {
        let input: Self =
            ic_wire::decode(bytes, FRAME, LIMITS).map_err(|_| "shard install frame")?;
        input.validate()?;
        Ok(input)
    }

    /// Check the enclosing receiver policy when the application selects Auth.
    /// # Errors
    /// Refuses disagreement between the two approved Root bindings.
    pub fn check_root(&self, root: Principal) -> Result<(), &'static str> {
        (root == self.root)
            .then_some(())
            .ok_or("conflicting installation Root")
    }
}
#[cfg(feature = "storage")]
mod actor {
    use super::{Install, Principal};
    use ic_platform_runtime::system as api;
    use ic_storage::{
        Memory, MemoryRuntime, StableKey,
        adapter::{StableCell, storable::Blob},
    };
    use std::cell::Cell;

    thread_local! { static PARENT: Cell<Option<Principal>> = const {Cell::new(None)}; }

    /// Create the bounded parent cell on install or strictly reopen its predecessor format.
    /// # Panics
    /// Traps on wrong installer, invalid or self parent, duplicate attachment or invalid storage.
    pub fn attach_parent<M: Memory>(host: &MemoryRuntime<M>, key: &str, install: Option<&Install>) {
        assert!(PARENT.get().is_none(), "parent already attached");
        let memory = host
            .open_memory(&StableKey::parse(key).expect("parent key"))
            .expect("parent lease");
        let cell = match install {
            Some(input) => {
                let parent = input.installing_parent();
                assert_eq!(memory.size(), 0, "occupied parent allocation");
                StableCell::create(
                    memory,
                    Blob::<29>::try_from(parent.as_slice()).expect("parent bytes"),
                )
            }
            None => {
                ic_storage::stable_cell_payload_length(&memory, 29)
                    .expect("bounded retained parent");
                StableCell::<Blob<29>, _>::load(memory).expect("retained parent")
            }
        };
        let bytes = cell.get().as_ref();
        assert_eq!(bytes.last(), Some(&1), "invalid retained parent");
        let parent = Principal::try_from_slice(bytes).expect("retained parent principal");
        assert_ne!(
            parent,
            api::canister_self(),
            "parent must differ from shard"
        );
        PARENT.set(Some(parent));
    }

    /// Parent recovered from this actor's own admitted storage.
    #[must_use]
    pub fn parent() -> Option<Principal> {
        PARENT.get()
    }

    /// Pre-decode guard for a parent-only endpoint.
    /// # Errors
    /// Refuses a missing binding or a caller other than this actor's parent.
    pub fn require_parent() -> Result<(), &'static str> {
        (parent() == Some(api::caller()))
            .then_some(())
            .ok_or("unauthorized hub")
    }
}
