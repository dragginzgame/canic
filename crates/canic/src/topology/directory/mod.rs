//! Local view of the published services in one managed fleet.

use ic_platform_types::Principal;
use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests;

fn check_name(name: &str) -> Result<(), &'static str> {
    if name.is_empty()
        || name.len() > 64
        || !name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    {
        return Err("invalid directory name");
    }
    Ok(())
}

/// Opt-in install arguments for a named service. The controller's ordinary application
/// approval retains these exact bytes; the principal is learned from Root's install result.
#[derive(Debug, Serialize, Deserialize)]
pub struct ServiceInstall {
    /// Stable service name.
    pub name: String,
    /// Role used in local lookup.
    pub role: String,
    /// Opaque inner arguments for the application.
    pub application: Vec<u8>,
}

impl ServiceInstall {
    const FRAME: ic_wire::Encoding = ic_wire::Encoding::Frame {
        method: ic_wire::method_id("canic.service_install.v1"),
        direction: ic_wire::Direction::Request,
    };

    /// Encode the service declaration into an existing bounded install approval.
    /// # Errors
    /// Refuses invalid names or excessive arguments.
    pub fn encode(&self) -> Result<Vec<u8>, &'static str> {
        check_name(&self.name)?;
        check_name(&self.role)?;
        ic_wire::encode(self, Self::FRAME, super::LIMITS).map_err(|_| "service install frame")
    }

    /// Decode and validate this service's method-bound install arguments.
    /// # Errors
    /// Refuses an unrelated frame, invalid names or excessive arguments.
    pub fn decode(bytes: &[u8]) -> Result<Self, &'static str> {
        let service: Self = ic_wire::decode(bytes, Self::FRAME, super::LIMITS)
            .map_err(|_| "service install frame")?;
        check_name(&service.name)?;
        check_name(&service.role)?;
        Ok(service)
    }
}

/// A named service admitted for fleet-wide discovery.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Service {
    /// Stable service name within the fleet.
    pub name: String,
    /// Several services may share a role.
    pub role: String,
    /// Current deployed identity.
    pub canister: Principal,
}

/// Complete service directory for one revision. Child allocation tables stay with their owners.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Directory {
    /// Monotonic publication revision, starting at one.
    pub revision: u64,
    /// Ordered by service name, independently of role or principal.
    pub services: Vec<Service>,
}

impl Directory {
    /// Check service identity, bounds and canonical row ordering before publication or receipt.
    /// # Errors
    /// Refuses invalid names, canister identities, ordering or revision.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.revision == 0 || self.services.len() > 32 {
            return Err("invalid directory revision or service count");
        }
        let mut principals = std::collections::BTreeSet::new();
        let mut preceding = None;
        for service in &self.services {
            check_name(&service.name)?;
            check_name(&service.role)?;
            if preceding.is_some_and(|name| name >= service.name.as_str())
                || service.canister.as_slice().last() != Some(&1)
                || !principals.insert(service.canister)
            {
                return Err("invalid or duplicate service identity or order");
            }
            preceding = Some(service.name.as_str());
        }
        Ok(())
    }

    /// Resolve every exact role match from this revision in service-name order.
    pub fn by_role<'a>(&'a self, role: &'a str) -> impl Iterator<Item = Principal> + 'a {
        self.services
            .iter()
            .filter(move |service| service.role == role)
            .map(|service| service.canister)
    }
}

/// Directional Wire contract for a full service directory delivery.
#[cfg(feature = "method")]
pub struct ReceiveDirectory;

#[cfg(feature = "method")]
impl crate::method::Contract for ReceiveDirectory {
    const NAME: &'static str = "receive_directory";
    const ID: u64 = ic_wire::method_id(Self::NAME);
    const REQUEST: ic_wire::Limits = DIRECTORY_LIMITS;
    const REPLY: ic_wire::Limits = crate::method::LIMITS;
    type Input = Directory;
    type Output = ();
}

/// Bounded directory frames and retained records.
pub const DIRECTORY_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 8192,
    allocation: 262_144,
    nodes: 4096,
    depth: 8,
};

/// Recipient-local persistence under the application's existing Storage bootstrap.
#[cfg(feature = "storage")]
pub mod retained {
    use super::{DIRECTORY_LIMITS, Directory, Principal};
    use ic_storage::{MemoryBacking, MemoryRuntime, adapter::StableCell};

    #[derive(serde::Deserialize, serde::Serialize)]
    struct Record {
        publisher: Principal,
        current: Option<Directory>,
    }

    /// A recipient's retained publisher and service directory.
    pub struct Index<M: MemoryBacking> {
        cell: StableCell<Vec<u8>, M>,
        record: Record,
    }

    impl<M: MemoryBacking> Index<M> {
        /// Create a direct recipient during installation by its fleet Root.
        /// # Panics
        /// Traps for an installer without a canister identity or an occupied allocation.
        #[must_use]
        pub fn install(runtime: &MemoryRuntime<M>, key: &str) -> Self {
            Self::create(runtime, key, ic_platform_runtime::system::caller())
        }

        /// Create a descendant index from its Root-approved parent binding.
        /// # Panics
        /// Traps if the installer differs from the supplied Root, the parent is invalid,
        /// or the allocation is already occupied.
        #[must_use]
        pub fn install_child(
            runtime: &MemoryRuntime<M>,
            key: &str,
            install: &crate::topology::Install,
        ) -> Self {
            Self::create(runtime, key, install.installing_parent())
        }

        pub(super) fn create(runtime: &MemoryRuntime<M>, key: &str, publisher: Principal) -> Self {
            assert_eq!(
                publisher.as_slice().last(),
                Some(&1),
                "directory installer must be a canister"
            );
            let record = Record {
                publisher,
                current: None,
            };
            let bytes = ic_wire::encode(&record, ic_wire::Encoding::Payload, DIRECTORY_LIMITS)
                .expect("encode initial directory");
            let cell = crate::storage::cell(runtime, key, DIRECTORY_LIMITS.bytes, Some(bytes));
            Self { cell, record }
        }

        /// Reopen the directory and the authority retained at install.
        /// # Panics
        /// Traps on missing storage, malformed state or invalid retained authority.
        #[must_use]
        pub fn open(runtime: &MemoryRuntime<M>, key: &str) -> Self {
            let cell = crate::storage::cell(runtime, key, DIRECTORY_LIMITS.bytes, None);
            let record: Record =
                ic_wire::decode(cell.get(), ic_wire::Encoding::Payload, DIRECTORY_LIMITS)
                    .expect("retained service directory");
            assert_eq!(
                record.publisher.as_slice().last(),
                Some(&1),
                "invalid directory publisher"
            );
            if let Some(directory) = &record.current {
                directory
                    .validate()
                    .expect("invalid retained service directory");
            }
            Self { cell, record }
        }

        /// Immediate publisher retained at installation.
        #[must_use]
        pub const fn publisher(&self) -> Principal {
            self.record.publisher
        }

        /// Last committed revision, available without an inter-canister call.
        #[must_use]
        pub const fn current(&self) -> Option<&Directory> {
            self.record.current.as_ref()
        }

        /// Accept a full directory from the expected publisher on this IC call.
        /// # Errors
        /// Refuses a foreign sender, malformed directory, conflict or regression.
        pub fn receive(&mut self, directory: Directory) -> Result<(), &'static str> {
            crate::topology::require_sender(self.record.publisher)?;
            self.commit(directory)
        }

        pub(super) fn commit(&mut self, next: Directory) -> Result<(), &'static str> {
            next.validate()?;
            if let Some(current) = self.current() {
                if current == &next {
                    return Ok(());
                }
                if current.revision >= next.revision {
                    return Err("stale or conflicting service directory");
                }
            }
            let record = Record {
                publisher: self.record.publisher,
                current: Some(next),
            };
            let bytes = ic_wire::encode(&record, ic_wire::Encoding::Payload, DIRECTORY_LIMITS)
                .map_err(|_| "service directory exceeds its storage budget")?;
            self.cell.set(bytes);
            self.record = record;
            Ok(())
        }
    }
}
