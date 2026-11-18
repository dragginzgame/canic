//! Installed Canic adapter + real Blob Service + application-owned catalog.
use canic::blob::RPC_LIMITS;
use canic::blob::{
    self,
    supplier::{
        caffeine::ManifestLimits,
        local::Limits,
        local::{Error as BlobError, ReferenceStatus, UploadState},
        protocol::{Command, Inspection, Installation, ManagementView, Observation, Outcome},
    },
};
use canic_asset_catalog::protocol as app;
use ic_testkit::pocket_ic::{CanisterId, nonblocking::PocketIc};
use serde::{Serialize, de::DeserializeOwned};
use std::{num::NonZeroU128, path::PathBuf};
mod blob_fleet_tests;
mod blob_profile_tests;
mod support;

fn id(n: u128) -> NonZeroU128 {
    n.try_into().unwrap()
}
fn actor(n: u8) -> CanisterId {
    CanisterId::self_authenticating([n])
}
fn directory() -> PathBuf {
    PathBuf::from(std::env::var_os("CANIC_BLOB_ARTIFACT_DIRECTORY").expect("make blob-test"))
}
fn initialization() -> app::Installation {
    app::Installation {
        users: [
            actor(1).as_slice().try_into().unwrap(),
            actor(2).as_slice().try_into().unwrap(),
        ],
        manager: actor(8).as_slice().try_into().unwrap(),
        limits: Limits {
            manifest: ManifestLimits {
                max_content_bytes: 100.try_into().unwrap(),
                max_chunks: 1.try_into().unwrap(),
                max_headers: 1.try_into().unwrap(),
                max_header_bytes: 100.try_into().unwrap(),
            },
            uploads: 64.try_into().unwrap(),
            references: 64.try_into().unwrap(),
            logical_bytes: 1000.try_into().unwrap(),
            exposure_bytes: 1000.try_into().unwrap(),
        },
    }
}
fn frame<T: Serialize>(method: u64, input: &T) -> Vec<u8> {
    ic_wire::encode(
        input,
        ic_wire::Encoding::Frame {
            method,
            direction: ic_wire::Direction::Request,
        },
        RPC_LIMITS,
    )
    .unwrap()
}
struct Client<'a> {
    pic: &'a PocketIc,
    canister: CanisterId,
}
impl Client<'_> {
    async fn update<T: Serialize, R: DeserializeOwned>(
        &self,
        caller: CanisterId,
        method: &str,
        method_id: u64,
        input: &T,
    ) -> R {
        let bytes = self
            .pic
            .update_call(self.canister, caller, method, frame(method_id, input))
            .await
            .unwrap();
        ic_wire::decode(
            &bytes,
            ic_wire::Encoding::Frame {
                method: method_id,
                direction: ic_wire::Direction::Response,
            },
            RPC_LIMITS,
        )
        .unwrap()
    }
    async fn query<T: Serialize, R: DeserializeOwned>(
        &self,
        caller: CanisterId,
        method: &str,
        method_id: u64,
        input: &T,
    ) -> R {
        let bytes = self
            .pic
            .query_call(self.canister, caller, method, frame(method_id, input))
            .await
            .unwrap();
        ic_wire::decode(
            &bytes,
            ic_wire::Encoding::Frame {
                method: method_id,
                direction: ic_wire::Direction::Response,
            },
            RPC_LIMITS,
        )
        .unwrap()
    }
    async fn command(&self, caller: CanisterId, input: Command) -> Result<Outcome, BlobError> {
        self.update(caller, "blob_update", blob::UPDATE, &input)
            .await
    }
    async fn authorize(
        &self,
        caller: CanisterId,
        draft: &app::Draft,
    ) -> Result<app::Entry, app::Error> {
        self.update(caller, "catalog_authorize", app::AUTHORIZE, draft)
            .await
    }
    async fn asset(&self, caller: CanisterId, n: u128) -> Result<app::Asset, app::Error> {
        self.query(caller, "catalog_inspect", app::INSPECT, &id(n))
            .await
    }
    async fn remove(&self, caller: CanisterId, n: u128) -> Result<app::Entry, app::Error> {
        self.update(caller, "catalog_remove", app::REMOVE, &id(n))
            .await
    }
    async fn publish(&self, caller: CanisterId, n: u128) -> Result<app::Entry, app::Error> {
        self.update(caller, "catalog_publish", app::PUBLISH, &id(n))
            .await
    }
    async fn management(&self) -> ManagementView {
        self.update::<_, Result<ManagementView, BlobError>>(
            actor(8),
            "blob_management",
            blob::MANAGEMENT,
            &(),
        )
        .await
        .unwrap()
    }
    async fn draft(&self, n: u128) -> app::Draft {
        app::Draft {
            id: id(n),
            reference: id(n),
            name: format!("asset-{n}"),
            root: [42; 32],
            bytes: 3,
            expires_at: self.pic.get_time().await.as_nanos_since_unix_epoch() + 600_000_000_000,
            chunks: vec![[42; 32]],
            headers: vec![],
        }
    }
    async fn fault(&self, point: u8) {
        self.update::<_, ()>(actor(9), "catalog_fault", app::FAULT, &point)
            .await;
    }
    async fn inspect_upload(&self, caller: CanisterId, n: u128) -> Result<Observation, BlobError> {
        self.query(
            caller,
            "blob_query",
            blob::QUERY,
            &Inspection::Upload(id(n)),
        )
        .await
    }
    async fn upgrade(&self, artifact: &str, args: Vec<u8>) -> Result<(), String> {
        canic_qualification::upgrade_stopped(
            self.pic,
            self.canister,
            std::fs::read(directory().join(artifact)).unwrap(),
            args,
            Some(actor(9)),
        )
        .await
        .map_err(|e| e.to_string())
    }
    async fn ready(&self) -> bool {
        let state: canic::readiness::State = self
            .update(actor(8), "readiness", canic::readiness::METHOD, &())
            .await;
        state == canic::readiness::State::Ready
    }
}
async fn install(pic: &PocketIc) -> Client<'_> {
    install_bytes(
        pic,
        std::fs::read(directory().join("original.wasm")).unwrap(),
    )
    .await
}
async fn install_bytes(pic: &PocketIc, wasm: Vec<u8>) -> Client<'_> {
    let canister = pic
        .create_canister_with_settings(Some(actor(9)), None)
        .await;
    pic.install_canister(
        canister,
        wasm,
        frame(app::INIT, &initialization()),
        Some(actor(9)),
    )
    .await;
    Client { pic, canister }
}

#[tokio::test(flavor = "current_thread")]
async fn registry_blob_refuses_incompatible_populated_predecessor() {
    let (_, binary) = support::inputs();
    let (mut server, owner) = support::instance(&binary).await;
    let pic = owner.client();
    let client = install_bytes(
        pic,
        include_bytes!("../fixtures/blob-catalog-01006.wasm").to_vec(),
    )
    .await;
    first(&client).await;
    client
        .authorize(actor(2), &client.draft(2).await)
        .await
        .unwrap();
    client.publish(actor(2), 2).await.unwrap();
    client.command(actor(8), Command::Drain(1)).await.unwrap();
    client.remove(actor(1), 1).await.unwrap();
    let retained = client.management().await;
    let bob = client.asset(actor(2), 2).await.unwrap();
    let previous = pic.get_stable_memory(client.canister).await;
    client
        .upgrade("candidate.wasm", frame(app::UPGRADE, &()))
        .await
        .expect_err("incompatible directory must refuse upgrade");
    assert_eq!(pic.get_stable_memory(client.canister).await, previous);
    let backing = ic_storage::VectorMemory::new(std::cell::RefCell::new(previous));
    assert!(matches!(
        ic_storage::MemoryRuntime::new(backing),
        Err(ic_storage::RuntimeConstructionError::Directory(
            ic_storage::DirectoryError::Invalid("root marker or reserved bytes")
        ))
    ));
    assert_eq!(client.management().await, retained);
    assert_eq!(client.asset(actor(2), 2).await.unwrap(), bob);
    assert_eq!(
        client.authorize(actor(1), &client.draft(3).await).await,
        Err(app::Error::Blob(BlobError::Maintenance))
    );
    client
        .command(actor(8), Command::Activate(1))
        .await
        .unwrap();
    assert_eq!(client.asset(actor(2), 2).await.unwrap(), bob);
    assert_eq!(client.publish(actor(1), 1).await, Err(app::Error::Released));
    client
        .authorize(actor(1), &client.draft(3).await)
        .await
        .unwrap();
    assert_eq!(
        owner.close(std::time::Duration::from_secs(10)).await,
        ic_testkit::server::Deletion::Attempted
    );
    let stopped = server.stop().unwrap();
    assert!(stopped.process.cleanup.is_empty(), "{stopped:?}");
}
async fn first(client: &Client<'_>) -> app::Draft {
    let draft = client.draft(1).await;
    client.authorize(actor(1), &draft).await.unwrap();
    client
        .command(actor(1), Command::Register(draft.root))
        .await
        .unwrap();
    client.publish(actor(1), 1).await.unwrap();
    draft
}

#[tokio::test(flavor = "current_thread")]
async fn captured_users_share_one_object_without_application_authority_leaking() {
    let (_, binary) = support::inputs();
    let (mut server, owner) = support::instance(&binary).await;
    let pic = owner.client();
    let client = install(pic).await;
    let first = first(&client).await;
    let second = client.draft(2).await;
    let before = pic.get_stable_memory(client.canister).await;
    // Controller and manager are neither of the two application users.
    for outsider in [actor(9), actor(8), actor(3), CanisterId::anonymous()] {
        assert_eq!(
            client.authorize(outsider, &second).await,
            Err(app::Error::Denied)
        );
    }
    assert_eq!(client.asset(actor(2), 1).await, Err(app::Error::Denied));
    assert_eq!(client.publish(actor(2), 1).await, Err(app::Error::Denied));
    assert_eq!(client.remove(actor(2), 1).await, Err(app::Error::Denied));
    assert_eq!(
        client.inspect_upload(actor(2), 1).await,
        Err(BlobError::Denied)
    );
    assert_eq!(
        client
            .command(actor(2), Command::Register(first.root))
            .await,
        Err(BlobError::Denied)
    );
    // Supplier DTO fields describe an intended upload, never the endpoint's caller.
    let forged = Command::Authorize {
        request: canic::blob::supplier::local::UploadRequest {
            id: second.id,
            first_reference: second.reference,
            uploader: client.canister.as_slice().try_into().unwrap(),
            root: second.root,
            bytes: second.bytes,
            expires_at: second.expires_at,
        },
        chunks: second.chunks.clone(),
        headers: second.headers.clone(),
    };
    assert_eq!(
        client.command(actor(1), forged).await,
        Err(BlobError::Denied)
    );
    assert_eq!(
        client
            .command(actor(1), Command::Retain(first.root, id(2)))
            .await,
        Err(BlobError::Denied)
    );
    assert_eq!(
        client.command(actor(1), Command::Release(id(1))).await,
        Err(BlobError::Denied)
    );
    assert_eq!(
        client.command(actor(9), Command::Drain(1)).await,
        Err(BlobError::Denied)
    );
    let denied: Result<Observation, BlobError> = client
        .query(actor(1), "blob_query", blob::QUERY, &Inspection::Usage)
        .await;
    assert_eq!(denied, Err(BlobError::Denied));
    assert_eq!(pic.get_stable_memory(client.canister).await, before);

    client.authorize(actor(2), &second).await.unwrap();
    client.publish(actor(2), 2).await.unwrap();
    let one = client.asset(actor(1), 1).await.unwrap();
    let two = client.asset(actor(2), 2).await.unwrap();
    assert_eq!(one.reference.root, two.reference.root);
    assert_ne!(
        one.entry.request.first_reference,
        two.entry.request.first_reference
    );
    assert_eq!(one.object.live_references, 2);
    assert_eq!(two.object.original_upload.request.id, first.id);
    assert!(
        matches!(client.inspect_upload(actor(2), 2).await, Ok(Observation::Upload(value)) if value.state == UploadState::Attached)
    );
    client.remove(actor(1), 1).await.unwrap();
    assert_eq!(
        client.asset(actor(1), 1).await.unwrap().reference.status,
        ReferenceStatus::Released
    );
    assert_eq!(
        client.asset(actor(2), 2).await.unwrap().reference.status,
        ReferenceStatus::Live
    );
    assert_eq!(
        client
            .asset(actor(2), 2)
            .await
            .unwrap()
            .object
            .live_references,
        1
    );
    let released = pic.get_stable_memory(client.canister).await;
    assert_eq!(
        client
            .authorize(actor(1), &first)
            .await
            .unwrap()
            .publication,
        app::Publication::Released
    );
    assert_eq!(client.publish(actor(1), 1).await, Err(app::Error::Released));
    client.remove(actor(1), 1).await.unwrap();
    assert_eq!(pic.get_stable_memory(client.canister).await, released);
    assert_eq!(
        owner.close(std::time::Duration::from_secs(10)).await,
        ic_testkit::server::Deletion::Attempted
    );
    let stopped = server.stop().unwrap();
    assert!(stopped.process.cleanup.is_empty(), "{stopped:?}");
}

#[tokio::test(flavor = "current_thread")]
async fn catalog_validation_and_post_mutation_traps_preserve_both_owners() {
    let (_, binary) = support::inputs();
    let (mut server, owner) = support::instance(&binary).await;
    let pic = owner.client();
    let client = install(pic).await;
    catalog_validation_and_rollback(&client).await;
    assert_eq!(
        owner.close(std::time::Duration::from_secs(10)).await,
        ic_testkit::server::Deletion::Attempted
    );
    let stopped = server.stop().unwrap();
    assert!(stopped.process.cleanup.is_empty(), "{stopped:?}");
}

// Exercise the same catalog contract with and without another durable participant.
async fn catalog_validation_and_rollback(client: &Client<'_>) {
    let pic = client.pic;
    first(client).await;
    let second = client.draft(2).await;
    let before = pic.get_stable_memory(client.canister).await;
    for point in [1, 2] {
        client.fault(point).await;
        assert!(
            pic.update_call(
                client.canister,
                actor(2),
                "catalog_authorize",
                frame(app::AUTHORIZE, &second)
            )
            .await
            .is_err()
        );
        assert_eq!(pic.get_stable_memory(client.canister).await, before);
        assert_eq!(
            client.inspect_upload(actor(2), 2).await,
            Err(BlobError::UnknownUpload)
        );
        assert_eq!(client.asset(actor(2), 2).await, Err(app::Error::Missing));
        client.fault(0).await;
    }
    client.authorize(actor(2), &second).await.unwrap();
    client.publish(actor(2), 2).await.unwrap();
    let shared = pic.get_stable_memory(client.canister).await;
    for point in [1, 2] {
        client.fault(point).await;
        assert!(
            pic.update_call(
                client.canister,
                actor(1),
                "catalog_remove",
                frame(app::REMOVE, &id(1))
            )
            .await
            .is_err()
        );
        assert_eq!(pic.get_stable_memory(client.canister).await, shared);
        assert_eq!(
            client.asset(actor(1), 1).await.unwrap().entry.publication,
            app::Publication::Published
        );
        assert_eq!(
            client
                .asset(actor(2), 2)
                .await
                .unwrap()
                .object
                .live_references,
            2
        );
        client.fault(0).await;
    }
    let mut conflict = client.draft(3).await;
    conflict.name = second.name.clone();
    assert_eq!(
        client.authorize(actor(1), &conflict).await,
        Err(app::Error::Conflict)
    );
    conflict.name = String::new();
    assert_eq!(
        client.authorize(actor(1), &conflict).await,
        Err(app::Error::Invalid)
    );
    assert_eq!(pic.get_stable_memory(client.canister).await, shared);
    for n in [3, 4] {
        client
            .authorize(actor(1), &client.draft(n).await)
            .await
            .unwrap();
    }
    let full = pic.get_stable_memory(client.canister).await;
    assert_eq!(
        client.authorize(actor(1), &client.draft(5).await).await,
        Err(app::Error::Capacity)
    );
    assert_eq!(
        client.inspect_upload(actor(1), 5).await,
        Err(BlobError::UnknownUpload)
    );
    assert_eq!(pic.get_stable_memory(client.canister).await, full);
    client.remove(actor(1), 1).await.unwrap(); // Capacity cannot prevent cleanup.
    assert_eq!(
        client.authorize(actor(1), &client.draft(5).await).await,
        Err(app::Error::Capacity)
    );
}

#[tokio::test(flavor = "current_thread")]
async fn malformed_frames_at_actual_mounts_leave_all_state_unchanged() {
    let (_, binary) = support::inputs();
    let (mut server, owner) = support::instance(&binary).await;
    let pic = owner.client();
    let client = install(pic).await;
    let draft = first(&client).await;
    let before = pic.get_stable_memory(client.canister).await;
    let methods = [
        (
            "blob_update",
            blob::UPDATE,
            frame(blob::UPDATE, &Command::Release(id(1))),
            false,
        ),
        (
            "blob_query",
            blob::QUERY,
            frame(blob::QUERY, &Inspection::Upload(id(1))),
            true,
        ),
        (
            "blob_management",
            blob::MANAGEMENT,
            frame(blob::MANAGEMENT, &()),
            false,
        ),
        (
            "catalog_authorize",
            app::AUTHORIZE,
            frame(app::AUTHORIZE, &draft),
            false,
        ),
        (
            "catalog_publish",
            app::PUBLISH,
            frame(app::PUBLISH, &id(1)),
            false,
        ),
        (
            "catalog_remove",
            app::REMOVE,
            frame(app::REMOVE, &id(1)),
            false,
        ),
        (
            "catalog_inspect",
            app::INSPECT,
            frame(app::INSPECT, &id(1)),
            true,
        ),
    ];
    for (name, method, valid, query) in methods {
        let mut trailing = valid.clone();
        trailing.push(0);
        for bytes in [
            vec![],
            frame(method + 1, &()),
            ic_wire::encode(
                &(),
                ic_wire::Encoding::Frame {
                    method,
                    direction: ic_wire::Direction::Response,
                },
                RPC_LIMITS,
            )
            .unwrap(),
            vec![0; RPC_LIMITS.bytes + 1],
            trailing,
            valid[..valid.len() - 1].to_vec(),
        ] {
            let result = if query {
                pic.query_call(client.canister, actor(1), name, bytes).await
            } else {
                pic.update_call(client.canister, actor(1), name, bytes)
                    .await
            };
            assert!(result.is_err(), "{name}");
            assert_eq!(
                pic.get_stable_memory(client.canister).await,
                before,
                "{name}"
            );
        }
    }
    assert_eq!(
        owner.close(std::time::Duration::from_secs(10)).await,
        ic_testkit::server::Deletion::Attempted
    );
    let stopped = server.stop().unwrap();
    assert!(stopped.process.cleanup.is_empty(), "{stopped:?}");
}

#[tokio::test(flavor = "current_thread")]
async fn drain_reopen_and_exact_activation_preserve_catalog_and_blob() {
    let (_, binary) = support::inputs();
    let (mut server, owner) = support::instance(&binary).await;
    let pic = owner.client();
    let client = install(pic).await;
    first(&client).await;
    client
        .authorize(actor(2), &client.draft(2).await)
        .await
        .unwrap();
    client.publish(actor(2), 2).await.unwrap();
    assert!(client.ready().await);
    let original = pic.get_stable_memory(client.canister).await;
    assert!(
        client
            .upgrade("candidate.wasm", frame(app::UPGRADE, &()))
            .await
            .is_err(),
        "undrained upgrade must trap"
    );
    assert_eq!(pic.get_stable_memory(client.canister).await, original);
    client.command(actor(8), Command::Drain(1)).await.unwrap();
    assert!(!client.ready().await);
    assert_eq!(
        client.authorize(actor(1), &client.draft(3).await).await,
        Err(app::Error::Blob(BlobError::Maintenance))
    );
    client.remove(actor(1), 1).await.unwrap(); // Cleanup remains available while drained.
    let closed = pic.get_stable_memory(client.canister).await;
    let inspection = client.management().await;
    assert_eq!(
        inspection.installation.as_slice(),
        client.canister.as_slice()
    );
    assert_eq!(
        inspection.configuration,
        Installation {
            application: client.canister.as_slice().try_into().unwrap(),
            manager: initialization().manager,
            limits: initialization().limits
        }
    );
    let bob = client.asset(actor(2), 2).await.unwrap();
    // Canic's current empty upgrade arguments are incompatible; they cannot activate Blob.
    assert!(client.upgrade("candidate.wasm", vec![]).await.is_err());
    assert_eq!(pic.get_stable_memory(client.canister).await, closed);
    client
        .upgrade("candidate.wasm", frame(app::UPGRADE, &()))
        .await
        .unwrap();
    assert_eq!(pic.get_stable_memory(client.canister).await, closed);
    assert_eq!(client.management().await, inspection);
    assert_eq!(client.asset(actor(2), 2).await.unwrap(), bob);
    assert!(
        !client.ready().await,
        "code/readiness checks cannot implicitly activate"
    );
    assert_eq!(
        client.command(actor(9), Command::Activate(1)).await,
        Err(BlobError::Denied)
    );
    client
        .command(actor(8), Command::Activate(1))
        .await
        .unwrap();
    assert!(client.ready().await);
    client.command(actor(8), Command::Drain(1)).await.unwrap();
    assert!(
        client.ready().await,
        "historical drain replay is not a new attempt"
    );
    client.command(actor(8), Command::Drain(2)).await.unwrap();
    assert!(!client.ready().await);
    client
        .upgrade("original.wasm", frame(app::UPGRADE, &()))
        .await
        .unwrap();
    let second = pic.get_stable_memory(client.canister).await;
    assert_eq!(
        client.command(actor(8), Command::Activate(1)).await,
        Err(BlobError::MaintenanceAttempt)
    );
    assert!(!client.ready().await);
    assert_eq!(pic.get_stable_memory(client.canister).await, second);
    client
        .command(actor(8), Command::Activate(2))
        .await
        .unwrap();
    assert!(client.ready().await);
    assert_eq!(client.asset(actor(2), 2).await.unwrap(), bob);
    assert_eq!(
        owner.close(std::time::Duration::from_secs(10)).await,
        ic_testkit::server::Deletion::Attempted
    );
    let stopped = server.stop().unwrap();
    assert!(stopped.process.cleanup.is_empty(), "{stopped:?}");
}

#[tokio::test(flavor = "current_thread")]
async fn reopen_failure_never_initializes_missing_invalid_or_foreign_state() {
    let (_, binary) = support::inputs();
    let (mut server, owner) = support::instance(&binary).await;
    let pic = owner.client();
    let source = install(pic).await;
    first(&source).await;
    source.command(actor(8), Command::Drain(1)).await.unwrap();
    let retained = pic.get_stable_memory(source.canister).await;
    for bytes in [vec![], vec![7; 65536], retained.clone()] {
        let recipient = install(pic).await;
        recipient
            .command(actor(8), Command::Drain(1))
            .await
            .unwrap();
        pic.set_stable_memory(
            recipient.canister,
            bytes.clone(),
            ic_testkit::pocket_ic::common::rest::BlobCompression::NoCompression,
        )
        .await;
        assert!(
            recipient
                .upgrade("candidate.wasm", frame(app::UPGRADE, &()))
                .await
                .is_err()
        );
        assert_eq!(pic.get_stable_memory(recipient.canister).await, bytes);
        assert_eq!(pic.get_stable_memory(source.canister).await, retained);
    }
    assert_eq!(
        owner.close(std::time::Duration::from_secs(10)).await,
        ic_testkit::server::Deletion::Attempted
    );
    let stopped = server.stop().unwrap();
    assert!(stopped.process.cleanup.is_empty(), "{stopped:?}");
}
