//! Paired receivers differ only in their selected Blob participant.
use crate::database;

#[cfg(not(feature = "blob-receiver"))]
canic::mount_receiver_lifecycle! {
    parent_storage_key = "witness.parent.v1", authority = "witness",
    auth_read = "read",
}

#[cfg(feature = "blob-receiver")]
canic::mount_receiver_lifecycle! {
    parent_storage_key = "witness.parent.v1", authority = "witness",
    suppliers = crate::receiver::blob::GROUPS => crate::receiver::blob::attach,
    auth_read = "read",
}

#[canic::query]
fn receiver_binding() -> (Option<ic_db::Principal>, Option<String>) {
    (canic::topology::parent(), canic::topology::role())
}

#[cfg(feature = "blob-receiver")]
mod blob {
    use canic::{
        auth::install::Install,
        blob::supplier::{
            caffeine::ManifestLimits,
            local::{
                Limits,
                persistence::{STORAGE_AUTHORITY, STORAGE_KEY},
            },
            protocol::{Command, Installation},
        },
        role::RoleStorage,
        storage::StorageGroup,
    };

    pub(super) const GROUPS: &[StorageGroup<'static>] = &[StorageGroup {
        authority: STORAGE_AUTHORITY,
        prefix: "blob.",
        keys: &[STORAGE_KEY],
    }];

    pub(super) fn attach(storage: &RoleStorage, install: Option<&Install>) {
        let id = ic_platform_runtime::system::canister_self();
        let memory = storage
            .open_memory(&ic_storage::StableKey::parse(STORAGE_KEY).unwrap())
            .unwrap();
        let fresh = install.map(|input| Installation {
            application: id,
            manager: input.root,
            limits: Limits {
                manifest: ManifestLimits {
                    max_content_bytes: 100.try_into().unwrap(),
                    max_chunks: 1.try_into().unwrap(),
                    max_headers: 1.try_into().unwrap(),
                    max_header_bytes: 100.try_into().unwrap(),
                },
                uploads: 1.try_into().unwrap(),
                references: 1.try_into().unwrap(),
                logical_bytes: 100.try_into().unwrap(),
                exposure_bytes: 100.try_into().unwrap(),
            },
        });
        let mut service =
            canic::blob::open_service(memory, id, fresh).expect("open Blob in its granted lease");
        if install.is_some() {
            service
                .execute(ic_platform_runtime::system::caller(), Command::Drain(1), 0)
                .unwrap();
        }
        assert!(service.is_drained(), "retained Blob maintenance state");
    }
}
