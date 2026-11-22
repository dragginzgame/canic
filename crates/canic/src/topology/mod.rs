//! Explicit topology for a newly created shard. The pool approval owns these bytes.
#[cfg(feature = "method")]
pub mod attestation;
pub mod directory;
#[cfg(feature = "role-attestation")]
mod service;
pub mod worker;
#[cfg(feature = "role-attestation")]
pub use service::{attach_role_calls, call_service, call_service_by_role};
#[cfg(feature = "method")]
pub mod dormancy;

use ic_platform_types::Principal;
use serde::{Deserialize, Serialize};

#[cfg(feature = "storage")]
pub use actor::{attach_parent, parent, role};

/// In a worker grant, request the exact allocation generation as the inner install payload.
pub const WORKER_GENERATION: &[u8] = b"canic.worker_generation.v1";

const INSTALL_ID: u64 = ic_wire::method_id("canic.shard_install.v2");
const FRAME: ic_wire::Encoding = ic_wire::Encoding::Frame {
    method: INSTALL_ID,
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
    /// Role selected by the application definition.
    pub role: String,
    /// Inner application arguments, e.g. a nonce.
    pub application: Vec<u8>,
}
impl Install {
    fn validate(&self) -> Result<(), &'static str> {
        directory::check_name(&self.role)?;
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
        if crate::guard::require_sender(self.root).is_err() {
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

    /// Inspect application bytes for the direct topology request frame.
    /// # Errors
    /// Rejects an invalid payload under the recognized header. Other application
    /// protocols remain opaque to this decoder.
    pub fn decode_if_present(bytes: &[u8]) -> Result<Option<Self>, &'static str> {
        if bytes
            .first_chunk()
            .is_some_and(|header| ic_wire::validate_request_header(INSTALL_ID, header).is_ok())
        {
            Self::decode(bytes).map(Some)
        } else {
            Ok(None)
        }
    }

    /// Resolve the approved topology in a direct or Auth-wrapped child install.
    /// # Errors
    /// Refuses a malformed recognized frame or disagreeing outer/inner Root.
    pub fn approved(bytes: &[u8]) -> Result<Option<Self>, &'static str> {
        #[cfg(feature = "auth")]
        if crate::auth::install::Install::is_frame(bytes) {
            let receiver = crate::auth::install::Install::decode(bytes)
                .map_err(|_| "malformed receiver installation")?;
            let binding = Self::decode_if_present(&receiver.application)?;
            if let Some(binding) = &binding {
                binding.check_root(receiver.root)?;
            }
            return Ok(binding);
        }
        Self::decode_if_present(bytes)
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
    use ic_storage::{MemoryBacking, MemoryRuntime};
    use std::cell::OnceCell;

    #[derive(serde::Serialize, serde::Deserialize)]
    struct Identity {
        parent: Principal,
        role: String,
    }

    thread_local! { static IDENTITY: OnceCell<Identity> = const {OnceCell::new()}; }

    /// Create or reopen the receiver's parent and role in its named allocation.
    /// # Panics
    /// Traps on an invalid install, duplicate attachment or invalid retained identity.
    pub fn attach_parent<M: MemoryBacking>(
        host: &MemoryRuntime<M>,
        key: &str,
        install: Option<&Install>,
    ) {
        let initial = install.map(|input| {
            let identity = Identity {
                parent: input.installing_parent(),
                role: input.role.clone(),
            };
            ic_wire::encode(&identity, ic_wire::Encoding::Payload, super::LIMITS)
                .expect("parent and role")
        });
        let cell = crate::storage::cell(host, key, 128, initial);
        let retained: Identity =
            ic_wire::decode(cell.get(), ic_wire::Encoding::Payload, super::LIMITS)
                .expect("retained parent and role");
        super::directory::check_name(&retained.role).expect("invalid retained role");
        assert_eq!(
            retained.parent.as_slice().last(),
            Some(&1),
            "invalid retained parent"
        );
        assert_ne!(
            retained.parent,
            api::canister_self(),
            "parent must differ from shard"
        );
        IDENTITY.with(|cell| assert!(cell.set(retained).is_ok(), "parent already attached"));
    }

    /// Parent recovered from this actor's own admitted storage.
    #[must_use]
    pub fn parent() -> Option<Principal> {
        IDENTITY.with(|cell| cell.get().map(|id| id.parent))
    }

    /// Role recovered from this actor's own admitted storage.
    #[must_use]
    pub fn role() -> Option<String> {
        IDENTITY.with(|cell| cell.get().map(|id| id.role.clone()))
    }
}
