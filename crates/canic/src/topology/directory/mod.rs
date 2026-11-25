//! Local view of the published services in one managed fleet.

use ic_platform_types::Principal;
use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests;

pub(super) fn check_name(name: &str) -> Result<(), &'static str> {
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
    /// Principal explicitly authorized to administer this hub’s routes.
    pub hub_operator: Option<Principal>,
    /// Operator authorized to administer direct-child payments on this actor.
    pub funding_operator: Option<Principal>,
    /// Stable service name.
    pub name: String,
    /// Role used in local lookup.
    pub role: String,
    /// Opaque inner arguments for the application.
    pub application: Vec<u8>,
}

impl ServiceInstall {
    const FRAME: ic_wire::Encoding = ic_wire::Encoding::Frame {
        method: ic_wire::method_id("canic.managed_service_install.v1"),
        direction: ic_wire::Direction::Request,
    };

    /// Read a named service from its direct install or from a DB receiver policy.
    /// # Errors
    /// Refuses malformed bindings and an enclosing Auth policy naming a different Root.
    pub fn approved(bytes: &[u8], root: Principal) -> Result<Self, &'static str> {
        #[cfg(feature = "auth")]
        if crate::auth::install::Install::is_frame(bytes) {
            let receiver = crate::auth::install::Install::decode(bytes)
                .map_err(|_| "malformed receiver installation")?;
            if receiver.root != root {
                return Err("conflicting service Root");
            }
            return Self::decode(&receiver.application);
        }
        let _ = root;
        Self::decode(bytes)
    }

    fn validate(&self) -> Result<(), &'static str> {
        check_name(&self.name)?;
        check_name(&self.role)?;
        if [self.hub_operator, self.funding_operator]
            .into_iter()
            .flatten()
            .any(|p| p == Principal::anonymous() || p == Principal::management_canister())
        {
            return Err("invalid hub operator");
        }
        Ok(())
    }

    /// Encode the service declaration into an existing bounded install approval.
    /// # Errors
    /// Refuses invalid names or excessive arguments.
    pub fn encode(&self) -> Result<Vec<u8>, &'static str> {
        self.validate()?;
        ic_wire::encode(self, Self::FRAME, super::LIMITS).map_err(|_| "service install frame")
    }

    /// Decode and validate this service's method-bound install arguments.
    /// # Errors
    /// Refuses an unrelated frame, invalid names or excessive arguments.
    pub fn decode(bytes: &[u8]) -> Result<Self, &'static str> {
        let service: Self = ic_wire::decode(bytes, Self::FRAME, super::LIMITS)
            .map_err(|_| "service install frame")?;
        service.validate()?;
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

    /// Resolve an exact published service name in this local revision.
    #[must_use]
    pub fn by_name(&self, name: &str) -> Option<Principal> {
        self.services
            .iter()
            .find(|service| service.name == name)
            .map(|service| service.canister)
    }

    /// Resolve a single service in this revision without selecting among siblings.
    /// # Errors
    /// Refuses a missing role or a role shared by multiple services.
    pub fn unique_by_role(&self, role: &str) -> Result<Principal, &'static str> {
        let mut matches = self.by_role(role);
        let principal = matches.next().ok_or("service role missing")?;
        if matches.next().is_some() {
            return Err("service role ambiguous");
        }
        Ok(principal)
    }

    /// Resolve every exact role match from this revision in service-name order.
    pub fn by_role<'a>(&'a self, role: &'a str) -> impl Iterator<Item = Principal> + 'a {
        self.services
            .iter()
            .filter(move |service| service.role == role)
            .map(|service| service.canister)
    }
}

/// Failure to accept or acknowledge a local topology update
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "inspection", derive(ic_wire::typescript::Wire))]
pub enum DirectoryError {
    /// Unexpected topology sender.
    Unauthorized,
    /// Invalid topology delivery.
    InvalidDelivery,
    /// Installation generation changed.
    GenerationChanged,
    /// Directory revision conflicts with retained state.
    RevisionConflict,
    /// Child installation was withdrawn.
    Withdrawn,
    /// Directory storage budget exhausted.
    Capacity,
    /// Directory has not been opened.
    Unavailable,
    /// Child directory delivery unconfirmed.
    ChildUnconfirmed,
}

impl std::fmt::Display for DirectoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Unauthorized => "Unexpected topology sender",
            Self::InvalidDelivery => "Invalid topology delivery",
            Self::GenerationChanged => "Installation generation changed",
            Self::RevisionConflict => "Directory revision conflicts with retained state",
            Self::Withdrawn => "Child installation was withdrawn",
            Self::Capacity => "Directory storage budget exhausted",
            Self::Unavailable => "Directory has not been opened",
            Self::ChildUnconfirmed => "Child directory delivery unconfirmed",
        })
    }
}

impl std::error::Error for DirectoryError {}

/// One addressed installation on the remaining path, in parent-to-child order.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Hop {
    /// Approved shared-shard pool, absent for other placement kinds.
    pub shard_pool: Option<String>,
    /// Installed canister.
    pub canister: Principal,
    /// Approved role, including for unnamed children.
    pub role: String,
    /// Root's immutable creation operation.
    pub generation: [u8; 32],
}

/// Parent-authenticated directory delivery. The first hop is its intended receiver.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Delivery {
    /// Shared services at the selected revision.
    pub directory: Directory,
    /// Exact branch remaining for this delivery.
    pub path: Vec<Hop>,
}

/// Root's outcome when replaying a retained directory revision to a named service.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "inspection", derive(ic_wire::typescript::Wire))]
pub enum RedeliveryError {
    /// Root has no ready named services.
    NoReadyServices,
    /// Retained services failed directory validation.
    InvalidDirectory,
    /// Root's directory changed before or during delivery.
    RevisionChanged,
    /// The requested service is absent from this revision.
    UnknownService,
    /// At least one path lacked a successful acknowledgement.
    DeliveryUnconfirmed,
}

/// Deliver a named service's original directory revision again.
#[cfg(feature = "method")]
pub struct RedeliverDirectoryRevision;

#[cfg(feature = "method")]
impl crate::method::Contract for RedeliverDirectoryRevision {
    const NAME: &'static str = "redeliver_directory_revision";
    const ID: u64 = ic_wire::method_id("canic.topology.redeliver_directory_revision.v1");
    type Input = (String, u64);
    type Output = Result<u64, RedeliveryError>;
}

/// Directional Wire contract for a full service directory delivery.
#[cfg(feature = "method")]
pub struct ReceiveDirectory;

#[cfg(feature = "method")]
impl crate::method::Contract for ReceiveDirectory {
    const NAME: &'static str = "receive_directory";
    const ID: u64 = ic_wire::method_id("canic.topology.receive_directory.v1");
    const REQUEST: ic_wire::Limits = DIRECTORY_LIMITS;
    const REPLY: ic_wire::Limits = crate::method::LIMITS;
    type Input = Delivery;
    type Output = Result<(), DirectoryError>;
}

/// Withdraw one exact child installation before its principal can be recycled.
#[cfg(feature = "method")]
pub struct WithdrawChild;

#[cfg(feature = "method")]
impl crate::method::Contract for WithdrawChild {
    const NAME: &'static str = "withdraw_child";
    const ID: u64 = ic_wire::method_id("canic.withdraw_child.v1");
    const REQUEST: ic_wire::Limits = DIRECTORY_LIMITS;
    type Input = (Principal, [u8; 32]);
    type Output = Result<(), DirectoryError>;
}

/// Mount delivery into the actor’s retained service directory. Install or reopen the
/// index during the actor's lifecycle before receiving the first publication.
#[cfg(all(feature = "storage", feature = "method"))]
#[macro_export]
macro_rules! mount_service_directory {
    () => {
        #[$crate::update(contract = $crate::topology::directory::ReceiveDirectory)]
        async fn receive_directory(
            update: $crate::topology::directory::Delivery,
        ) -> Result<(), $crate::topology::directory::DirectoryError> {
            use $crate::topology::directory::retained::{self, DIRECTORY};
            retained::receive(&DIRECTORY, update).await
        }

        #[$crate::update(contract = $crate::topology::directory::WithdrawChild)]
        fn withdraw_child(
            child: <$crate::topology::directory::WithdrawChild as $crate::method::Contract>::Input,
        ) -> Result<(), $crate::topology::directory::DirectoryError> {
            $crate::topology::directory::retained::withdraw(child)
        }
    };
}

/// Bounded directory frames and retained records.
pub const DIRECTORY_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 10 * 1024,
    allocation: 262_144,
    nodes: 4096,
    depth: 8,
};

/// Recipient-local persistence under the application's existing Storage bootstrap.
#[cfg(feature = "storage")]
pub mod retained {
    use super::DirectoryError;
    use super::{DIRECTORY_LIMITS, Delivery, Directory, Hop, Principal, check_name};
    use ic_storage::{MemoryBacking, MemoryRuntime, adapter::StableCell};

    #[derive(Clone, serde::Deserialize, serde::Serialize)]
    struct Child {
        hop: Hop,
        acknowledged: Option<u64>,
    }

    #[derive(Clone, serde::Deserialize, serde::Serialize)]
    struct Record {
        publisher: Principal,
        role: String,
        generation: Option<[u8; 32]>,
        children: Vec<Child>,
        current: Option<Directory>,
        withdrawn: Vec<(Principal, [u8; 32])>,
    }

    /// A recipient's retained publisher and service directory.
    pub struct Index<M: MemoryBacking> {
        cell: StableCell<Vec<u8>, M>,
        record: Record,
    }

    // Keep old index bytes distinguishable even when neither layout contains children.
    const INDEX_FRAME: ic_wire::Encoding = ic_wire::Encoding::Frame {
        method: ic_wire::method_id("canic.topology.directory_index.v1"),
        direction: ic_wire::Direction::Response,
    };

    impl<M: MemoryBacking> Index<M> {
        /// Create a direct recipient during installation by its fleet Root.
        /// # Panics
        /// Traps for an installer without a canister identity or an occupied allocation.
        #[must_use]
        pub fn install(runtime: &MemoryRuntime<M>, key: &str, role: &str) -> Self {
            Self::create(runtime, key, ic_platform_runtime::system::caller(), role)
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
            Self::create(runtime, key, install.installing_parent(), &install.role)
        }

        pub(super) fn create(
            runtime: &MemoryRuntime<M>,
            key: &str,
            publisher: Principal,
            role: &str,
        ) -> Self {
            assert_eq!(
                publisher.as_slice().last(),
                Some(&1),
                "directory installer must be a canister"
            );
            check_name(role).expect("invalid directory role");
            let record = Record {
                publisher,
                role: role.to_owned(),
                generation: None,
                children: Vec::new(),
                current: None,
                withdrawn: Vec::new(),
            };
            let bytes = ic_wire::encode(&record, INDEX_FRAME, DIRECTORY_LIMITS)
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
            let record: Record = ic_wire::decode(cell.get(), INDEX_FRAME, DIRECTORY_LIMITS)
                .expect("retained service directory");
            assert_eq!(
                record.publisher.as_slice().last(),
                Some(&1),
                "invalid directory publisher"
            );
            check_name(&record.role).expect("invalid retained directory role");
            assert_eq!(
                record.generation.is_some(),
                record.current.is_some(),
                "invalid directory generation"
            );
            assert!(record.children.len() <= 16, "invalid retained child count");
            let mut seen = std::collections::BTreeSet::new();
            for child in &record.children {
                check_name(&child.hop.role).expect("invalid retained child role");
                assert!(
                    child
                        .hop
                        .shard_pool
                        .as_ref()
                        .is_none_or(|pool| !pool.is_empty() && pool.len() <= 64)
                        && child.hop.canister.as_slice().last() == Some(&1)
                        && child.hop.canister != record.publisher
                        && seen.insert(child.hop.canister)
                        && record.current.as_ref().is_some_and(|d| child
                            .acknowledged
                            .is_none_or(|r| r > 0 && r <= d.revision)),
                    "invalid retained directory child"
                );
            }
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

        /// Role retained independently of service directory membership.
        #[must_use]
        pub fn role(&self) -> &str {
            &self.record.role
        }

        /// Last confirmed delivery for a known direct child. An absent revision is pending.
        #[must_use]
        pub fn child_delivery(&self, child: Principal) -> Option<(&Hop, Option<u64>)> {
            self.record
                .children
                .iter()
                .find(|entry| entry.hop.canister == child)
                .map(|entry| (&entry.hop, entry.acknowledged))
        }

        /// Direct children of this role that confirmed at least one directory delivery.
        /// Includes unnamed children even while a newer revision awaits delivery.
        pub fn acknowledged_children<'a>(
            &'a self,
            role: &'a str,
        ) -> impl Iterator<Item = Principal> + 'a {
            self.record.children.iter().filter_map(move |child| {
                (child.hop.role == role && child.acknowledged.is_some())
                    .then_some(child.hop.canister)
            })
        }

        /// Inspect the pool's acknowledged direct children in the last committed directory.
        pub fn acknowledged_shards<'a>(
            &'a self,
            pool: &'a str,
        ) -> impl Iterator<Item = Principal> + 'a {
            self.record.children.iter().filter_map(move |child| {
                (child.acknowledged.is_some() && child.hop.shard_pool.as_deref() == Some(pool))
                    .then_some(child.hop.canister)
            })
        }

        /// Validate the IC caller and branch against this receiver, then commit its local projection.
        /// # Errors
        /// Refuses an invalid publisher, target, role, path or superseded directory.
        pub fn accept(&mut self, update: &Delivery) -> Result<(), DirectoryError> {
            let receiver = ic_platform_runtime::system::canister_self();
            crate::guard::require_sender(self.publisher())
                .map_err(|_| DirectoryError::Unauthorized)?;
            let first = update.path.first().ok_or(DirectoryError::InvalidDelivery)?;
            if first.canister != receiver || first.role != self.role() {
                return Err(DirectoryError::InvalidDelivery);
            }
            let mut seen = std::collections::BTreeSet::from([self.publisher()]);
            for hop in &update.path {
                check_name(&hop.role).map_err(|_| DirectoryError::InvalidDelivery)?;
                if hop
                    .shard_pool
                    .as_ref()
                    .is_some_and(|pool| pool.is_empty() || pool.len() > 64)
                {
                    return Err(DirectoryError::InvalidDelivery);
                }
                if update.directory.services.iter().any(|row| {
                    row.canister == hop.canister
                        && (row.role != hop.role || hop.shard_pool.is_some())
                }) {
                    return Err(DirectoryError::InvalidDelivery);
                }
                if hop.canister.as_slice().last() != Some(&1) || !seen.insert(hop.canister) {
                    return Err(DirectoryError::InvalidDelivery);
                }
            }
            self.apply(&update.directory, first.generation, update.path.get(1))
        }

        #[cfg(feature = "method")]
        pub(super) fn acknowledge(
            &mut self,
            directory: &Directory,
            child: &Hop,
        ) -> Result<(), DirectoryError> {
            let rev = directory.revision;
            if self.current() != Some(directory) {
                return Err(DirectoryError::RevisionConflict);
            }
            let mut record = self.record.clone();
            let entry = record
                .children
                .iter_mut()
                .find(|entry| &entry.hop == child)
                .ok_or(DirectoryError::GenerationChanged)?;
            if entry.acknowledged != Some(rev) {
                entry.acknowledged = Some(rev);
                self.save(record)?;
            }
            Ok(())
        }

        pub(super) fn withdraw(
            &mut self,
            target: (Principal, [u8; 32]),
        ) -> Result<(), DirectoryError> {
            if self.record.withdrawn.contains(&target) {
                return Ok(());
            }
            let mut record = self.record.clone();
            record
                .children
                .retain(|entry| (entry.hop.canister, entry.hop.generation) != target);
            record.withdrawn.push(target);
            self.save(record)
        }

        fn save(&mut self, record: Record) -> Result<(), DirectoryError> {
            let bytes = ic_wire::encode(&record, INDEX_FRAME, DIRECTORY_LIMITS)
                .map_err(|_| DirectoryError::Capacity)?;
            self.cell.set(bytes);
            self.record = record;
            Ok(())
        }

        pub(super) fn apply(
            &mut self,
            next: &Directory,
            generation: [u8; 32],
            child: Option<&Hop>,
        ) -> Result<(), DirectoryError> {
            next.validate()
                .map_err(|_| DirectoryError::InvalidDelivery)?;
            if let Some(current) = self.current()
                && current != next
                && current.revision >= next.revision
            {
                return Err(DirectoryError::RevisionConflict);
            }
            if self
                .record
                .generation
                .is_some_and(|current| current != generation)
            {
                return Err(DirectoryError::GenerationChanged);
            }
            if let Some(current) = self.current()
                && current.revision < next.revision
                && !current
                    .services
                    .iter()
                    .all(|service| next.services.contains(service))
            {
                return Err(DirectoryError::RevisionConflict);
            }
            let mut record = self.record.clone();
            record.generation = Some(generation);
            if let Some(child) = child {
                if record
                    .withdrawn
                    .contains(&(child.canister, child.generation))
                {
                    return Err(DirectoryError::Withdrawn);
                }
                if let Some((stored, _)) = self.child_delivery(child.canister) {
                    if stored != child {
                        return Err(DirectoryError::GenerationChanged);
                    }
                } else {
                    if record.children.len() >= 16 {
                        return Err(DirectoryError::Capacity);
                    }
                    record.children.push(Child {
                        hop: child.clone(),
                        acknowledged: None,
                    });
                }
            }
            if self.current() == Some(next) && self.record.children.len() == record.children.len() {
                return Ok(());
            }
            record.current = Some(next.clone());
            self.save(record)
        }
    }

    thread_local! {
        /// This actor's service index. Install or reopen it during application lifecycle.
        pub static DIRECTORY: std::cell::RefCell<Option<Index<ic_storage::DefaultMemoryImpl>>> =
            const { std::cell::RefCell::new(None) };
    }

    /// Durably refuse future deliveries for the exact installation.
    /// # Errors
    /// Requires the retained publisher and sufficient index space.
    pub fn withdraw(target: (Principal, [u8; 32])) -> Result<(), DirectoryError> {
        DIRECTORY.with_borrow_mut(|cell| {
            let index = cell.as_mut().ok_or(DirectoryError::Unavailable)?;
            crate::guard::require_sender(index.publisher())
                .map_err(|_| DirectoryError::Unauthorized)?;
            index.withdraw(target)
        })
    }

    /// Apply the directory locally and forward only the remaining admitted branch.
    /// # Errors
    /// Refuses a foreign publisher, mismatched receiver, invalid branch or unconfirmed hop.
    /// # Panics
    /// Requires an installed canister with an open index.
    #[cfg(feature = "method")]
    pub async fn receive<M: MemoryBacking + 'static>(
        index: &'static std::thread::LocalKey<std::cell::RefCell<Option<Index<M>>>>,
        mut update: Delivery,
    ) -> Result<(), DirectoryError> {
        index.with_borrow_mut(|slot| slot.as_mut().expect("open service index").accept(&update))?;
        update.path.remove(0);
        let Some(child) = update.path.first().cloned() else {
            return Ok(());
        };
        crate::method::call::<super::ReceiveDirectory>(child.canister, 5, &update)
            .await
            .map_err(|_| DirectoryError::ChildUnconfirmed)??;
        index.with_borrow_mut(|slot| {
            let index = slot.as_mut().expect("open service index");
            index.acknowledge(&update.directory, &child)
        })
    }
}

/// Approved stateful service arguments inside the named service declaration.
/// Database identity and verifier trust have independent owners and lengths.
#[derive(Debug, Serialize, Deserialize)]
pub struct DbService {
    /// Application selected incarnation nonce.
    pub nonce: [u8; 16],
    /// Public half of the installing Root's role signer.
    pub issuer_key: [u8; 32],
}

impl DbService {
    const FRAME: ic_wire::Encoding = ic_wire::Encoding::Frame {
        method: ic_wire::method_id("canic.db_role_service.v1"),
        direction: ic_wire::Direction::Request,
    };
    /// Read DB bindings when an Auth receiver declares a named service.
    /// # Errors
    /// Refuses missing or malformed bindings and a different installing Root.
    pub fn from_receiver(bytes: &[u8], root: Principal) -> Result<Option<Self>, &'static str> {
        #[cfg(feature = "auth")]
        if crate::auth::install::Install::is_frame(bytes) {
            let receiver = crate::auth::install::Install::decode(bytes)
                .map_err(|_| "malformed receiver installation")?;
            if receiver.application.first_chunk().is_some_and(|header| {
                ic_wire::validate_request_header(
                    ic_wire::method_id("canic.managed_service_install.v1"),
                    header,
                )
                .is_ok()
            }) {
                if receiver.root != root {
                    return Err("conflicting service Root");
                }
                let service = ServiceInstall::decode(&receiver.application)?;
                return Self::decode(&service.application).map(Some);
            }
        }
        let _ = (bytes, root);
        Ok(None)
    }
    /// Encode the independent bindings.
    /// # Errors
    /// Refuses a zero nonce or a framing failure.
    pub fn encode(&self) -> Result<Vec<u8>, &'static str> {
        if self.nonce == [0; 16] {
            return Err("zero installation nonce");
        }
        ic_wire::encode(self, Self::FRAME, super::LIMITS).map_err(|_| "DB service frame")
    }
    /// Open the exact approved frame.
    /// # Errors
    /// Refuses a malformed frame or zero nonce.
    pub fn decode(bytes: &[u8]) -> Result<Self, &'static str> {
        let value: Self =
            ic_wire::decode(bytes, Self::FRAME, super::LIMITS).map_err(|_| "DB service frame")?;
        if value.nonce == [0; 16] {
            return Err("zero installation nonce");
        }
        Ok(value)
    }
}
