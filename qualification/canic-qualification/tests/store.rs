//! Canic application installation and Root integration with the standalone Store.
mod credential;
mod store_root_tests;
mod support;

use canic_management_protocol::stored::STORE_LIMITS;
use ic_testkit::pocket_ic::CanisterId;
use ic_wasm_store_protocol as store;

#[path = "../fixtures/local_store.rs"]
mod local_store;
#[path = "../fixtures/miner_pool.rs"]
mod miner_pool_fixture;
use local_store::Client;

impl Client<'_> {
    async fn inspect(&self, request: &store::Inspect) -> store::Inspection {
        let encoding = |direction| ic_wire::Encoding::Frame {
            method: store::INSPECT,
            direction,
        };
        let bytes =
            ic_wire::encode(request, encoding(ic_wire::Direction::Request), STORE_LIMITS).unwrap();
        let reply = self
            .pic
            .query_call(self.id, self.operator, "inspect", bytes)
            .await
            .unwrap();
        ic_wire::decode::<Result<store::Inspection, store::Error>>(
            &reply,
            encoding(ic_wire::Direction::Response),
            STORE_LIMITS,
        )
        .unwrap()
        .unwrap()
    }

    async fn info(&self) -> store::Info {
        let store::Inspection::Info(info) = self.inspect(&store::Inspect::Info).await else {
            panic!("expected Store info")
        };
        info
    }

    async fn observe(&self, request: &store::Observe) -> Result<store::Observation, store::Error> {
        self.update("observe", store::OBSERVE, request, STORE_LIMITS)
            .await
    }

    async fn status(&self, release: &store::Release) -> store::Status {
        let store::Observation::Status(status) = self
            .observe(&store::Observe::Status(release.clone()))
            .await
            .unwrap()
        else {
            panic!("expected release status")
        };
        status
    }

    async fn release_custody(
        &self,
        release: &store::Release,
    ) -> Result<store::Custody, store::Error> {
        let store::Observation::Custody(custody) = self
            .observe(&store::Observe::Custody(release.clone()))
            .await?
        else {
            panic!("expected release custody")
        };
        Ok(custody)
    }

    async fn publish_bytes(&self, version: &str, bytes: &[u8]) -> store::Manifest {
        self.publish_named("greeting", version, bytes).await
    }

    async fn pin(&self, pin: &store::Pin) -> store::PinReceipt {
        self.update::<_, Result<store::PinReceipt, store::Error>>(
            "pin",
            store::PIN,
            pin,
            STORE_LIMITS,
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
            STORE_LIMITS,
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
    let (mut server, owner) = support::instance(&binary).await;
    let pic = owner.client();
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
            STORE_LIMITS,
        )
        .await;
    assert_eq!(absent, Err(store::CacheError::Pin(store::Error::NotFound)));
    let receipt = client.pin(&pin).await;
    assert_eq!(client.pin(&pin).await, receipt);
    let mut changed = pin.clone();
    changed.manifest.hash[0] ^= 1;
    let refused: Result<store::PinReceipt, store::Error> = client
        .update("pin", store::PIN, &changed, STORE_LIMITS)
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
                STORE_LIMITS
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
            STORE_LIMITS,
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
    assert_eq!(
        client
            .observe(&store::Observe::Pin(pin.clone()))
            .await
            .unwrap(),
        store::Observation::Pin(receipt.clone())
    );
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
                    method: canic_greeting_witness::GREET,
                    direction: ic_wire::Direction::Request,
                },
                canic_greeting_witness::LIMITS,
            )
            .unwrap(),
        )
        .await
        .unwrap();
    let greeting: canic_greeting_witness::Greeting = ic_wire::decode(
        &reply,
        ic_wire::Encoding::Frame {
            method: canic_greeting_witness::GREET,
            direction: ic_wire::Direction::Response,
        },
        canic_greeting_witness::LIMITS,
    )
    .unwrap();
    assert!(greeting.message.contains("Store"));
    println!(
        "Store pins survived maintenance; cold/warm cache and cache-loss repair supported real install and upgrade"
    );
    assert_eq!(
        owner.close(std::time::Duration::from_secs(10)).await,
        ic_testkit::server::Deletion::Attempted
    );
    let stopped = server.stop().unwrap();
    assert!(stopped.process.cleanup.is_empty(), "{stopped:?}");
}
