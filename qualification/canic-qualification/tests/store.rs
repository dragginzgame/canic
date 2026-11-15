//! Installed Store upload custody, raw chunk integrity and independent maintenance.
use canic_wasm_store_protocol as store;
use ic_testkit::pocket_ic::{CanisterId, nonblocking::PocketIc};
use serde::{Serialize, de::DeserializeOwned};
mod store_root_tests;
mod support;

struct Client<'a> {
    pic: &'a PocketIc,
    id: CanisterId,
    operator: CanisterId,
}
impl Client<'_> {
    async fn update<T: Serialize, R: DeserializeOwned>(
        &self,
        method: &str,
        id: u64,
        input: &T,
        limits: ic_wire::Limits,
    ) -> R {
        let reply = self
            .pic
            .update_call(
                self.id,
                self.operator,
                method,
                ic_wire::encode(
                    input,
                    ic_wire::Encoding::Frame {
                        method: id,
                        direction: ic_wire::Direction::Request,
                    },
                    limits,
                )
                .unwrap(),
            )
            .await
            .unwrap();
        ic_wire::decode(
            &reply,
            ic_wire::Encoding::Frame {
                method: id,
                direction: ic_wire::Direction::Response,
            },
            limits,
        )
        .unwrap()
    }
    async fn info(&self) -> store::Info {
        let reply = self
            .pic
            .query_call(
                self.id,
                self.operator,
                "info",
                ic_wire::encode(
                    &(),
                    ic_wire::Encoding::Frame {
                        method: store::INFO,
                        direction: ic_wire::Direction::Request,
                    },
                    store::LIMITS,
                )
                .unwrap(),
            )
            .await
            .unwrap();
        ic_wire::decode(
            &reply,
            ic_wire::Encoding::Frame {
                method: store::INFO,
                direction: ic_wire::Direction::Response,
            },
            store::LIMITS,
        )
        .unwrap()
    }
    async fn status(&self, release: &store::Release) -> store::Status {
        self.update::<_, Result<store::Status, store::Error>>(
            "observe",
            store::STATUS,
            release,
            store::LIMITS,
        )
        .await
        .unwrap()
    }
}

#[tokio::test(flavor = "current_thread")]
async fn versioned_upload_resumes_across_two_executables_and_reuses_bytes() {
    let (original, binary) = support::inputs();
    let candidate =
        std::fs::read(std::env::var_os("CANIC_STORE_CANDIDATE").expect("run make store-test"))
            .unwrap();
    assert_ne!(original, candidate);
    let owner = support::instance(&binary).await;
    let pic = owner.instance();
    let operator = CanisterId::self_authenticating(b"store qualification controller");
    let id = pic
        .create_canister_with_settings(Some(operator), None)
        .await;
    pic.install_canister(id, original, vec![], Some(operator))
        .await;
    let client = Client { pic, id, operator };
    // More than the old combined Root budget. Integrity publication is not Wasm validation.
    let mut bytes = vec![0; store::CHUNK_BYTES * store::MAX_CHUNKS];
    for (index, chunk) in bytes.chunks_mut(store::CHUNK_BYTES).enumerate() {
        chunk.fill(index as u8);
    }
    bytes[..8].copy_from_slice(b"\0asm\x01\0\0\0");
    let manifest = store::Manifest {
        release: store::Release {
            name: "app".into(),
            version: "1.0.0".into(),
        },
        hash: store::digest(&bytes),
        bytes: bytes.len() as u64,
        chunks: bytes
            .chunks(store::CHUNK_BYTES)
            .map(store::digest)
            .collect(),
    };
    assert!(
        pic.update_call(
            id,
            CanisterId::anonymous(),
            "prepare",
            ic_wire::encode(
                &manifest,
                ic_wire::Encoding::Frame {
                    method: store::PREPARE,
                    direction: ic_wire::Direction::Request
                },
                store::LIMITS
            )
            .unwrap()
        )
        .await
        .is_err()
    );
    assert!(
        pic.update_call(id, operator, "put", vec![0; store::CHUNK_LIMITS.bytes + 1])
            .await
            .is_err()
    );
    let initial = client.info().await;
    assert_eq!(initial.releases, 0);
    let prepared: Result<store::Status, store::Error> = client
        .update("prepare", store::PREPARE, &manifest, store::LIMITS)
        .await;
    assert!(!prepared.unwrap().published);
    let first = store::Put {
        release: manifest.release.clone(),
        index: 0,
        bytes: bytes[..store::CHUNK_BYTES].to_vec(),
    };
    let _: Result<store::Status, store::Error> = client
        .update("put", store::PUT, &first, store::CHUNK_LIMITS)
        .await;
    let partial = client.status(&manifest.release).await;
    assert!(!partial.missing.contains(&0));
    let blocked: Result<store::Status, store::Error> = client
        .update("publish", store::PUBLISH, &manifest.release, store::LIMITS)
        .await;
    assert_eq!(blocked, Err(store::Error::Incomplete));
    pic.stop_canister(id, Some(operator)).await.unwrap();
    pic.upgrade_canister(id, candidate, vec![], Some(operator))
        .await
        .unwrap();
    pic.start_canister(id, Some(operator)).await.unwrap();
    assert!(client.info().await.maintenance_candidate);
    assert_eq!(client.status(&manifest.release).await, partial);
    // Resume only reported missing indices, at the complete admitted artifact bound.
    for index in partial.missing {
        let start = index as usize * store::CHUNK_BYTES;
        let end = (start + store::CHUNK_BYTES).min(bytes.len());
        let input = store::Put {
            release: manifest.release.clone(),
            index,
            bytes: bytes[start..end].to_vec(),
        };
        let result: Result<store::Status, store::Error> = client
            .update("put", store::PUT, &input, store::CHUNK_LIMITS)
            .await;
        result.unwrap();
    }
    let published: Result<store::Status, store::Error> = client
        .update("publish", store::PUBLISH, &manifest.release, store::LIMITS)
        .await;
    let published = published.unwrap();
    assert!(published.published);
    let repeated: Result<store::Status, store::Error> = client
        .update("publish", store::PUBLISH, &manifest.release, store::LIMITS)
        .await;
    assert_eq!(repeated.unwrap(), published);
    let usage = client.info().await;
    assert_eq!(usage.chunks, store::MAX_CHUNKS as u64);
    let second = store::Manifest {
        release: store::Release {
            version: "2.0.0".into(),
            ..manifest.release.clone()
        },
        ..manifest.clone()
    };
    let reused: Result<store::Status, store::Error> = client
        .update("prepare", store::PREPARE, &second, store::LIMITS)
        .await;
    assert!(reused.unwrap().published);
    assert_eq!(client.info().await.chunks, usage.chunks);
    assert_eq!(client.info().await.reserved_bytes, usage.reserved_bytes);
    let mut conflict = manifest.clone();
    conflict.hash[0] ^= 1;
    let result: Result<store::Status, store::Error> = client
        .update("prepare", store::PREPARE, &conflict, store::LIMITS)
        .await;
    assert_eq!(result, Err(store::Error::Conflict));
    let too_small = store::Quota {
        reserved_bytes: 1,
        releases: 1,
    };
    let result: Result<store::Info, store::Error> = client
        .update("configure", store::CONFIGURE, &too_small, store::LIMITS)
        .await;
    assert_eq!(result, Err(store::Error::Capacity));
    let changed_quota = store::Quota {
        reserved_bytes: usage.reserved_bytes * 2,
        releases: 10,
    };
    let result: Result<store::Info, store::Error> = client
        .update("configure", store::CONFIGURE, &changed_quota, store::LIMITS)
        .await;
    assert_eq!(result.unwrap().quota, changed_quota);
    // Replicated reads recover exact bytes without relying on uncertified query results.
    let mut recovered = Vec::new();
    for index in 0..manifest.chunks.len() {
        let input = store::Read {
            release: manifest.release.clone(),
            index: index as u32,
        };
        let reply = pic
            .update_call(
                id,
                operator,
                "read_chunk",
                ic_wire::encode(
                    &input,
                    ic_wire::Encoding::Frame {
                        method: store::READ,
                        direction: ic_wire::Direction::Request,
                    },
                    store::LIMITS,
                )
                .unwrap(),
            )
            .await
            .unwrap();
        let chunk: Result<store::Chunk, store::Error> = ic_wire::decode(
            &reply,
            ic_wire::Encoding::Frame {
                method: store::READ,
                direction: ic_wire::Direction::Response,
            },
            store::CHUNK_LIMITS,
        )
        .unwrap();
        recovered.extend(chunk.unwrap().0);
    }
    assert_eq!(recovered, bytes);
    let reply = pic
        .query_call(
            id,
            operator,
            "list",
            ic_wire::encode(
                &Option::<store::Release>::None,
                ic_wire::Encoding::Frame {
                    method: store::LIST,
                    direction: ic_wire::Direction::Request,
                },
                store::LIMITS,
            )
            .unwrap(),
        )
        .await
        .unwrap();
    let listed: Result<Vec<store::Status>, store::Error> = ic_wire::decode(
        &reply,
        ic_wire::Encoding::Frame {
            method: store::LIST,
            direction: ic_wire::Direction::Response,
        },
        store::LIMITS,
    )
    .unwrap();
    assert_eq!(listed.unwrap().len(), 2);
    println!(
        "Store: {} payload bytes, {} unique chunks, two retained versions; upload resumed after changed executable",
        bytes.len(),
        usage.chunks
    );
    owner.close().await;
}

impl Client<'_> {
    async fn publish_bytes(&self, version: &str, bytes: &[u8]) -> store::Manifest {
        let manifest = store::Manifest {
            release: store::Release {
                name: "greeting".into(),
                version: version.into(),
            },
            hash: store::digest(bytes),
            bytes: bytes.len() as u64,
            chunks: bytes
                .chunks(store::CHUNK_BYTES)
                .map(store::digest)
                .collect(),
        };
        self.update::<_, Result<store::Status, store::Error>>(
            "prepare",
            store::PREPARE,
            &manifest,
            store::LIMITS,
        )
        .await
        .unwrap();
        for (index, bytes) in bytes.chunks(store::CHUNK_BYTES).enumerate() {
            self.update::<_, Result<store::Status, store::Error>>(
                "put",
                store::PUT,
                &store::Put {
                    release: manifest.release.clone(),
                    index: index as u32,
                    bytes: bytes.to_vec(),
                },
                store::CHUNK_LIMITS,
            )
            .await
            .unwrap();
        }
        self.update::<_, Result<store::Status, store::Error>>(
            "publish",
            store::PUBLISH,
            &manifest.release,
            store::LIMITS,
        )
        .await
        .unwrap();
        manifest
    }
    async fn pin(&self, pin: &store::Pin) -> store::PinReceipt {
        self.update::<_, Result<store::PinReceipt, store::Error>>(
            "pin",
            store::PIN,
            pin,
            store::LIMITS,
        )
        .await
        .unwrap()
    }
    async fn cache(&self, pin: &store::Pin) -> store::CacheReady {
        self.update::<_, Result<store::CacheReady, store::CacheError>>(
            "prepare_cache",
            store::PREPARE_CACHE,
            &store::PrepareCache {
                pin: pin.clone(),
                expires_at_ns: self.pic.get_time().await.as_nanos_since_unix_epoch()
                    + 300_000_000_000,
            },
            store::LIMITS,
        )
        .await
        .unwrap()
    }
}

#[tokio::test(flavor = "current_thread")]
async fn pins_survive_maintenance_and_cache_reconstruction_supports_install_and_upgrade() {
    let (original, binary) = support::inputs();
    let directory = std::path::PathBuf::from(
        std::env::var_os("CANIC_STORE_APP_DIRECTORY").expect("run make store-test"),
    );
    let candidate = std::fs::read(directory.join("candidate.wasm")).unwrap();
    let app = std::fs::read(directory.join("app.wasm")).unwrap();
    let app_v2 = std::fs::read(directory.join("app-v2.wasm")).unwrap();
    let owner = support::instance(&binary).await;
    let pic = owner.instance();
    let operator = CanisterId::self_authenticating(b"Store delivery controller");
    let id = pic
        .create_canister_with_settings(Some(operator), None)
        .await;
    pic.install_canister(id, original, vec![], Some(operator))
        .await;
    let client = Client { pic, id, operator };
    let manifest = client.publish_bytes("1", &app).await;
    let pin = store::Pin {
        operation: [1; 32],
        manifest,
    };
    let request = store::PrepareCache {
        pin: pin.clone(),
        expires_at_ns: pic.get_time().await.as_nanos_since_unix_epoch() + 300_000_000_000,
    };
    let absent: Result<store::CacheReady, store::CacheError> = client
        .update(
            "prepare_cache",
            store::PREPARE_CACHE,
            &request,
            store::LIMITS,
        )
        .await;
    assert_eq!(absent, Err(store::CacheError::Pin(store::Error::NotFound)));
    let receipt = client.pin(&pin).await;
    assert_eq!(client.pin(&pin).await, receipt);
    let mut changed = pin.clone();
    changed.manifest.hash[0] ^= 1;
    let refused: Result<store::PinReceipt, store::Error> = client
        .update("pin", store::PIN, &changed, store::LIMITS)
        .await;
    assert_eq!(refused, Err(store::Error::Conflict));
    assert!(
        pic.update_call(
            id,
            CanisterId::anonymous(),
            "prepare_cache",
            ic_wire::encode(
                &request,
                ic_wire::Encoding::Frame {
                    method: store::PREPARE_CACHE,
                    direction: ic_wire::Direction::Request
                },
                store::LIMITS
            )
            .unwrap()
        )
        .await
        .is_err()
    );
    let expired = store::PrepareCache {
        expires_at_ns: 0,
        ..request
    };
    let refused: Result<store::CacheReady, store::CacheError> = client
        .update(
            "prepare_cache",
            store::PREPARE_CACHE,
            &expired,
            store::LIMITS,
        )
        .await;
    assert_eq!(refused, Err(store::CacheError::Expired));
    assert!(
        pic.stored_chunks(id, Some(operator))
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        client.cache(&pin).await.uploaded as usize,
        pin.manifest.chunks.len()
    );
    assert_eq!(client.cache(&pin).await.uploaded, 0);
    // Discarding a successful reply does not require another upload on replay.
    let before = pic.stored_chunks(id, Some(operator)).await.unwrap();
    client.cache(&pin).await;
    assert_eq!(pic.stored_chunks(id, Some(operator)).await.unwrap(), before);
    // The Store's stable pins/payloads are independent of its management cache.
    pic.clear_chunk_store(id, Some(operator)).await.unwrap();
    pic.stop_canister(id, Some(operator)).await.unwrap();
    pic.upgrade_canister(id, candidate, vec![], Some(operator))
        .await
        .unwrap();
    pic.start_canister(id, Some(operator)).await.unwrap();
    let observed: Result<store::PinReceipt, store::Error> = client
        .update("observe_pin", store::OBSERVE_PIN, &pin, store::LIMITS)
        .await;
    assert_eq!(observed.unwrap(), receipt);
    assert_eq!(
        client.cache(&pin).await.uploaded as usize,
        pin.manifest.chunks.len()
    );

    // Provider-boundary proof: one controller owns target and Store. This does not
    // impersonate a running Root or claim its durable approval path is integrated.
    let target = pic
        .create_canister_with_settings(Some(operator), None)
        .await;
    pic.install_chunked_canister(
        target,
        Some(operator),
        ic_testkit::pocket_ic::CanisterInstallMode::Install,
        id,
        pin.manifest
            .chunks
            .iter()
            .map(|hash| hash.to_vec())
            .collect(),
        pin.manifest.hash.to_vec(),
        vec![],
    )
    .await
    .unwrap();
    let installed = pic.canister_status(target, Some(operator)).await.unwrap();
    assert_eq!(installed.module_hash, Some(pin.manifest.hash.to_vec()));
    let second = store::Pin {
        operation: [2; 32],
        manifest: client.publish_bytes("2", &app_v2).await,
    };
    client.pin(&second).await;
    client.cache(&second).await;
    pic.stop_canister(target, Some(operator)).await.unwrap();
    pic.install_chunked_canister(
        target,
        Some(operator),
        ic_testkit::pocket_ic::CanisterInstallMode::Upgrade(None),
        id,
        second
            .manifest
            .chunks
            .iter()
            .map(|hash| hash.to_vec())
            .collect(),
        second.manifest.hash.to_vec(),
        vec![],
    )
    .await
    .unwrap();
    pic.start_canister(target, Some(operator)).await.unwrap();
    let upgraded = pic.canister_status(target, Some(operator)).await.unwrap();
    assert_eq!(upgraded.module_hash, Some(second.manifest.hash.to_vec()));
    assert!(client.status(&pin.manifest.release).await.published);
    assert_eq!(client.pin(&pin).await, receipt);
    let reply = pic
        .query_call(
            target,
            CanisterId::anonymous(),
            "greet",
            ic_wire::encode(
                &"Store",
                ic_wire::Encoding::Frame {
                    method: canic::GREET,
                    direction: ic_wire::Direction::Request,
                },
                canic::LIMITS,
            )
            .unwrap(),
        )
        .await
        .unwrap();
    let greeting: canic::Greeting = ic_wire::decode(
        &reply,
        ic_wire::Encoding::Frame {
            method: canic::GREET,
            direction: ic_wire::Direction::Response,
        },
        canic::LIMITS,
    )
    .unwrap();
    assert!(greeting.message.contains("Store"));
    println!(
        "Store pins survived maintenance; cold/warm cache and cache-loss repair supported real install and upgrade"
    );
    owner.close().await;
}
