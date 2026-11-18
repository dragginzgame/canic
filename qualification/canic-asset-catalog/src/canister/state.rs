//! One application Storage bootstrap, two independently owned durable records.
use super::super::protocol::{self, Asset, Draft, Entry, Error, Publication};
use canic::blob::RPC_LIMITS;
use canic::blob::supplier::{
    local::{
        ReferenceStatus,
        persistence::{STORAGE_AUTHORITY, STORAGE_KEY, Service},
    },
    protocol::{Command, Inspection, Installation, Observation},
};
use ic_platform_types::Principal;
use ic_storage::{
    AllocationGrants, DefaultMemoryImpl, GenericAllocationPolicy, Memory, MemoryAuthority,
    MemoryRequest, MemoryRuntime, SealedDeclarationSnapshot, StableKey, StorageConfig,
    adapter::StableCell,
};
use serde::{Deserialize, Serialize};

const CATALOG_KEY: &str = "catalog.entries.v1";
#[derive(Clone, Deserialize, Serialize)]
struct Catalog {
    version: u8,
    installation: Principal,
    users: [Principal; 2],
    entries: Vec<Entry>,
}
pub(super) struct State {
    pub blob: Service<DefaultMemoryImpl>,
    #[cfg(feature = "profile")]
    pub profiles: canic_profile_worker::state::State<DefaultMemoryImpl>,
    catalog: Catalog,
    cell: StableCell<Vec<u8>>,
}
impl State {
    pub fn open(input: Option<protocol::Installation>, installation: Principal) -> Self {
        let backing = DefaultMemoryImpl::default();
        assert_eq!(
            backing.size() == 0,
            input.is_some(),
            "fresh and existing installations are distinct"
        );
        let mut runtime = MemoryRuntime::new_with_config(
            backing,
            StorageConfig {
                allocations: if cfg!(feature = "profile") { 4 } else { 2 },
                extents: if cfg!(feature = "profile") { 6 } else { 4 },
                pages: 64,
                ..Default::default()
            },
        )
        .expect("one application backing");
        if input.is_none() {
            assert_eq!(
                runtime
                    .memory_allocations()
                    .expect("retained directory")
                    .len(),
                if cfg!(feature = "profile") { 4 } else { 2 },
                "composition membership requires an explicit migration"
            );
        }
        let requests = vec![
            MemoryRequest::new("catalog", CATALOG_KEY).unwrap(),
            MemoryRequest::new(STORAGE_AUTHORITY, STORAGE_KEY).unwrap(),
        ];
        let grants = vec![
            MemoryAuthority::new("catalog", "catalog.").unwrap(),
            MemoryAuthority::new(STORAGE_AUTHORITY, "blob.").unwrap(),
        ];
        #[cfg(feature = "profile")]
        let requests = requests
            .into_iter()
            .chain(canic_profile_worker::state::memory_requests())
            .collect();
        #[cfg(feature = "profile")]
        let grants = grants
            .into_iter()
            .chain([MemoryAuthority::new(
                canic_profile_worker::state::STORAGE_AUTHORITY,
                canic_profile_worker::state::STORAGE_PREFIX,
            )
            .unwrap()])
            .collect();
        runtime
            .bootstrap(
                &SealedDeclarationSnapshot::new(requests).unwrap(),
                &AllocationGrants::new(grants).unwrap(),
                &GenericAllocationPolicy,
            )
            .expect("combined memory admission");
        let catalog_memory = runtime
            .open_memory(&StableKey::parse(CATALOG_KEY).unwrap())
            .expect("catalog lease");
        let blob_memory = runtime
            .open_memory(&StableKey::parse(STORAGE_KEY).unwrap())
            .expect("Blob lease");
        let state = if let Some(input) = input {
            validate_users(input.users, installation);
            let catalog = Catalog {
                version: 1,
                installation,
                users: input.users,
                entries: vec![],
            };
            let bytes = ic_wire::encode(&catalog, ic_wire::Encoding::Payload, RPC_LIMITS)
                .expect("initial catalog");
            let cell = StableCell::create(catalog_memory, bytes);
            let blob = Service::create(
                blob_memory,
                installation,
                Installation {
                    application: installation,
                    manager: input.manager,
                    limits: input.limits,
                },
            )
            .expect("fresh Blob");
            Self {
                blob,
                #[cfg(feature = "profile")]
                profiles: canic_profile_worker::state::State::create(&runtime),
                cell,
                catalog,
            }
        } else {
            let blob = Service::open(blob_memory, installation)
                .expect("existing Blob; no initialization fallback");
            ic_storage::stable_cell_payload_length(&catalog_memory, RPC_LIMITS.bytes)
                .expect("bounded catalog");
            let cell = StableCell::<Vec<u8>>::load(catalog_memory).expect("existing catalog");
            let catalog: Catalog =
                ic_wire::decode(cell.get(), ic_wire::Encoding::Payload, RPC_LIMITS)
                    .expect("catalog record");
            assert!(blob.is_drained(), "reopen retains closed admission");
            Self {
                blob,
                #[cfg(feature = "profile")]
                profiles: canic_profile_worker::state::State::open(&runtime),
                cell,
                catalog,
            }
        };
        state.validate(installation);
        state
    }
    fn validate(&self, installation: Principal) {
        assert_eq!(self.catalog.version, 1, "catalog version");
        assert_eq!(
            self.catalog.installation, installation,
            "catalog installation"
        );
        validate_users(self.catalog.users, installation);
        assert!(self.catalog.entries.len() <= protocol::CAPACITY);
        for (index, entry) in self.catalog.entries.iter().enumerate() {
            assert!(valid_name(&entry.name));
            self.user(entry.request.uploader)
                .expect("retained catalog user");
            assert!(
                !self.catalog.entries[..index]
                    .iter()
                    .any(|old| collides(old, entry)),
                "catalog identity conflict"
            );
            let Observation::Upload(upload) = self
                .blob
                .inspect(installation, Inspection::Upload(entry.request.id))
                .expect("retained Blob authorization")
            else {
                panic!("upload reply")
            };
            assert_eq!(upload.request, entry.request);
            let reference = self
                .reference(entry.request.first_reference)
                .expect("retained reference");
            assert_eq!(reference.root, entry.request.root);
            assert!(
                match entry.publication {
                    Publication::Draft => matches!(
                        reference.status,
                        ReferenceStatus::Reserved | ReferenceStatus::Live
                    ),
                    Publication::Published => reference.status == ReferenceStatus::Live,
                    Publication::Released => matches!(
                        reference.status,
                        ReferenceStatus::Released | ReferenceStatus::Retired
                    ),
                },
                "catalog/reference agreement"
            );
        }
    }
    pub(super) fn user(&self, caller: Principal) -> Result<(), Error> {
        if self.catalog.users.contains(&caller) {
            Ok(())
        } else {
            Err(Error::Denied)
        }
    }
    fn owned(&self, caller: Principal, reference: std::num::NonZeroU128) -> Result<usize, Error> {
        self.user(caller)?;
        let index = self
            .catalog
            .entries
            .iter()
            .position(|entry| entry.request.first_reference == reference)
            .ok_or(Error::Missing)?;
        if self.catalog.entries[index].request.uploader != caller {
            return Err(Error::Denied);
        }
        Ok(index)
    }
    fn persist(&mut self, fault: u8) {
        // Failure after Blob mutation must trap, never return an ordinary Result error.
        let limits = if fault == 1 {
            ic_wire::Limits {
                bytes: 0,
                ..RPC_LIMITS
            }
        } else {
            RPC_LIMITS
        };
        let bytes = ic_wire::encode(&self.catalog, ic_wire::Encoding::Payload, limits)
            .expect("catalog persistence encoding");
        self.cell.set(bytes);
    }
    pub fn authorize(
        &mut self,
        caller: Principal,
        draft: Draft,
        now: u64,
        fault: u8,
    ) -> Result<Entry, Error> {
        self.user(caller)?;
        if !valid_name(&draft.name) {
            return Err(Error::Invalid);
        }
        let entry = Entry {
            name: draft.name,
            request: canic::blob::supplier::local::UploadRequest {
                id: draft.id,
                uploader: caller,
                root: draft.root,
                bytes: draft.bytes,
                first_reference: draft.reference,
                expires_at: draft.expires_at,
            },
            publication: Publication::Draft,
        };
        let existing = self
            .catalog
            .entries
            .iter()
            .position(|old| collides(old, &entry));
        if let Some(index) = existing {
            let old = &self.catalog.entries[index];
            if old.name != entry.name || old.request != entry.request {
                return Err(Error::Conflict);
            }
        } else if self.catalog.entries.len() == protocol::CAPACITY {
            return Err(Error::Capacity);
        }
        // All catalog validation precedes the trusted application-authority path.
        self.blob
            .execute(
                self.catalog.installation,
                Command::Authorize {
                    request: entry.request,
                    chunks: draft.chunks,
                    headers: draft.headers,
                },
                now,
            )
            .map_err(Error::Blob)?;
        if let Some(index) = existing {
            return Ok(self.catalog.entries[index].clone());
        }
        self.catalog.entries.push(entry.clone());
        self.persist(fault);
        Ok(entry)
    }
    pub fn publish(
        &mut self,
        caller: Principal,
        reference: std::num::NonZeroU128,
        fault: u8,
    ) -> Result<Entry, Error> {
        let index = self.owned(caller, reference)?;
        if self.catalog.entries[index].publication == Publication::Released {
            return Err(Error::Released);
        }
        if self.blob.is_drained() {
            return Err(Error::Blob(
                canic::blob::supplier::local::Error::Maintenance,
            ));
        }
        if self.reference(reference)?.status != ReferenceStatus::Live {
            return Err(Error::NotLive);
        }
        self.catalog.entries[index].publication = Publication::Published;
        self.persist(fault);
        Ok(self.catalog.entries[index].clone())
    }
    pub fn remove(
        &mut self,
        caller: Principal,
        reference: std::num::NonZeroU128,
        now: u64,
        fault: u8,
    ) -> Result<Entry, Error> {
        let index = self.owned(caller, reference)?;
        self.blob
            .execute(self.catalog.installation, Command::Release(reference), now)
            .map_err(Error::Blob)?;
        self.catalog.entries[index].publication = Publication::Released;
        self.persist(fault);
        Ok(self.catalog.entries[index].clone())
    }
    fn reference(
        &self,
        reference: std::num::NonZeroU128,
    ) -> Result<canic::blob::supplier::local::ReferenceView, Error> {
        match self
            .blob
            .inspect(self.catalog.installation, Inspection::Reference(reference))
            .map_err(Error::Blob)?
        {
            Observation::Reference(value) => Ok(value),
            _ => unreachable!("typed supplier reply"),
        }
    }
    pub fn inspect(
        &self,
        caller: Principal,
        reference: std::num::NonZeroU128,
    ) -> Result<Asset, Error> {
        let entry = self.catalog.entries[self.owned(caller, reference)?].clone();
        let reference = self.reference(reference)?;
        let Observation::Object(object) = self
            .blob
            .inspect(
                self.catalog.installation,
                Inspection::Object(entry.request.root),
            )
            .map_err(Error::Blob)?
        else {
            unreachable!("typed supplier reply")
        };
        Ok(Asset {
            entry,
            reference,
            object,
        })
    }
}
fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 32
        && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
}
fn collides(a: &Entry, b: &Entry) -> bool {
    a.name == b.name
        || a.request.id == b.request.id
        || a.request.first_reference == b.request.first_reference
}
fn validate_users(users: [Principal; 2], installation: Principal) {
    assert_ne!(users[0], users[1]);
    assert!(
        users.iter().all(|user| *user != Principal::anonymous()
            && *user != Principal::management_canister()
            && *user != installation),
        "explicit real users"
    );
}
