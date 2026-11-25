//! Installed CLI custody: publication -> removal -> explicit restoration, plus lost replies.
use super::{
    CanisterId, CanisterSettings, Duration, Limits, Path, PathBuf, PocketIcBuilder, PreparedServer,
    Server, Value, cli, hex, json, run,
};
use canic_management_protocol::{children, publication as wire, stored::STORE_LIMITS};
use ic_testkit::pocket_ic::nonblocking::PocketIc;
use ic_wasm_store_protocol as store;

fn time() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos()
        .try_into()
        .unwrap()
}

fn deadlines(dir: &Path, operation: &str, steps: &[&str]) -> u64 {
    let deadline = time() + 12_000_000_000;
    for step in steps {
        let path = dir.join("journal").join(operation).join(step);
        std::fs::create_dir_all(&path).unwrap();
        // Fixture chooses a shorter dispatch window before any signature exists.
        std::fs::write(path.join("dispatch-deadline.json"), deadline.to_string()).unwrap();
    }
    deadline
}

async fn expire(deadline: u64) {
    tokio::time::sleep(Duration::from_nanos(
        deadline.saturating_sub(time()) + 250_000_000,
    ))
    .await;
}

fn publication(dir: &Path, version: &str, root_hash: &str, store_hash: &str) -> String {
    cli(
        dir,
        &[
            "fleet",
            "plan",
            "--canister",
            "app",
            "--publish-version",
            version,
            "--store",
            "store",
            "--root-module",
            root_hash,
            "--store-module",
            store_hash,
            "--controller",
            "2vxsx-fae",
        ],
    )["operation"]
        .as_str()
        .unwrap()
        .into()
}

fn remove(dir: &Path, operation: &str, action: &str) -> String {
    cli(
        dir,
        &[
            "fleet",
            "artifact",
            "plan-removal",
            operation,
            "--action",
            action,
        ],
    )["operation"]
        .as_str()
        .unwrap()
        .into()
}

fn apply(dir: &Path, operation: &str) -> Value {
    cli(
        dir,
        &["fleet", "apply", operation, "--identity", "anonymous"],
    )
}

async fn update<T: serde::Serialize, R: serde::de::DeserializeOwned>(
    pic: &PocketIc,
    canister: CanisterId,
    method: &str,
    id: u64,
    value: &T,
) -> R {
    let bytes = ic_wire::encode(
        value,
        ic_wire::Encoding::Frame {
            method: id,
            direction: ic_wire::Direction::Request,
        },
        wire::REQUEST_LIMITS,
    )
    .unwrap();
    let reply = pic
        .update_call(canister, CanisterId::anonymous(), method, bytes)
        .await
        .unwrap();
    ic_wire::decode(
        &reply,
        ic_wire::Encoding::Frame {
            method: id,
            direction: ic_wire::Direction::Response,
        },
        wire::RESPONSE_LIMITS,
    )
    .unwrap()
}

async fn observe(
    pic: &PocketIc,
    root: CanisterId,
    supplier: CanisterId,
    version: &str,
) -> wire::Reply {
    let value: wire::ObservationReply = update(
        pic,
        root,
        "observe_store",
        wire::OBSERVATION,
        &wire::ObservationRequest::ReleaseStatus(wire::CustodyRequest {
            store: supplier.to_text(),
            expires_at_ns: pic.get_time().await.as_nanos_since_unix_epoch() + 240_000_000_000,
            release: store::Release {
                name: "app".into(),
                version: version.into(),
            },
        }),
    )
    .await;
    match value? {
        wire::Observation::ReleaseStatus(status) => Ok(status),
        _ => panic!("expected release status"),
    }
}

async fn fault(pic: &PocketIc, root: CanisterId, value: u8) {
    update::<_, ()>(
        pic,
        root,
        "inject_child_fault",
        children::INJECT,
        &children::Request {
            slot: children::ChildId::FIRST,
            value,
        },
    )
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn cli_removal_restoration_retains_attempts_and_blocks_lost_results() {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .to_owned();
    let assets = PathBuf::from(std::env::var_os("CANIC_PUBLICATION_ARTIFACTS").unwrap());
    let binary = PreparedServer::check(&canic_qualification::server_directory()).unwrap();
    let mut server = Server::start(
        binary,
        Limits {
            startup: Duration::from_secs(30),
            lifetime: Duration::from_secs(300),
            stdout_bytes: 1024 * 1024,
            stderr_bytes: 1024 * 1024,
        },
    )
    .unwrap();
    let mut owner = server
        .instance(
            PocketIcBuilder::new()
                .with_nns_subnet()
                .with_application_subnet(),
            Duration::from_secs(30),
        )
        .await
        .unwrap();
    let pic = owner.client();
    let root_bytes = std::fs::read(assets.join("root-faults.wasm")).unwrap();
    let store_bytes = std::fs::read(assets.join("store.wasm")).unwrap();
    let root = pic.create_canister().await;
    let supplier = pic.create_canister().await;
    for canister in [root, supplier] {
        pic.add_cycles(canister, 10_000_000_000_000).await;
    }
    pic.install_canister(root, root_bytes.clone(), vec![], None)
        .await;
    pic.install_canister(supplier, store_bytes.clone(), vec![], None)
        .await;
    pic.update_canister_settings(
        supplier,
        None,
        CanisterSettings {
            controllers: Some(vec![root]),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    let root_stable = pic.get_stable_memory(root).await;
    let url = owner.client_mut().make_live(None).await.to_string();
    let pic = owner.client();
    let dir = repo
        .join("target/publication")
        .join(format!("removal-{}", std::process::id()));
    std::fs::create_dir(&dir).unwrap();
    let wasm = dir.join("app.wasm");
    std::fs::copy(assets.join("app.wasm"), &wasm).unwrap();
    assert!(std::fs::metadata(&wasm).unwrap().len() < store::CHUNK_BYTES as u64);
    let config = json!({"apps":{"assets":{
        "canisters":{"root":{"role":"root"},"store":{"role":"wasm-store","parent":"root"},"app":{"parent":"root","wasm":wasm}},
        "fleets":{"local":{"network":{"kind":"Local","url":url,"root_key":hex(&pic.root_key().await.unwrap()),"root_subnet":pic.topology().await.get_nns().unwrap().to_text(),"routing_canister":root.to_text()},"journal":"journal","canisters":{"root":{"id":root.to_text()},"store":{"id":supplier.to_text()}}}}
    }}});
    std::fs::write(dir.join("canic.toml"), toml::to_string(&config).unwrap()).unwrap();
    let root_hash = hex(&store::digest(&root_bytes));
    let store_hash = hex(&store::digest(&store_bytes));
    let original = publication(&dir, "v1", &root_hash, &store_hash);
    let deadline = deadlines(
        &dir,
        &original,
        &[
            "publication/prepare",
            "publication/chunk-0",
            "publication/publish",
        ],
    );
    apply(&dir, &original);
    assert!(
        !run(
            &dir,
            &[
                "fleet",
                "artifact",
                "plan-removal",
                &original,
                "--action",
                "delete"
            ]
        )
        .status
        .success()
    );
    let alias = publication(&dir, "shared", &root_hash, &store_hash);
    deadlines(
        &dir,
        &alias,
        &[
            "publication/prepare",
            "publication/chunk-0",
            "publication/publish",
        ],
    );
    apply(&dir, &alias);
    let before = observe(pic, root, supplier, "v1").await.unwrap().manifest;
    expire(deadline).await;
    let removal = remove(&dir, &original, "delete");
    assert_eq!(remove(&dir, &original, "delete"), removal);
    assert!(
        !run(
            &dir,
            &["fleet", "reconcile", &removal, "--identity", "anonymous"]
        )
        .status
        .success()
    );
    let deadline = deadlines(&dir, &removal, &["removal"]);
    let result = apply(&dir, &removal);
    let path = dir.join("journal").join(&removal).join("removal");
    let signed = std::fs::read(path.join("signed.json")).unwrap();
    assert_eq!(apply(&dir, &removal), result);
    assert_eq!(
        cli(
            &dir,
            &["fleet", "reconcile", &removal, "--identity", "anonymous"]
        ),
        result
    );
    assert_eq!(std::fs::read(path.join("signed.json")).unwrap(), signed);
    assert!(
        !run(
            &dir,
            &["fleet", "apply", &original, "--identity", "anonymous"]
        )
        .status
        .success()
    );
    assert_eq!(
        observe(pic, root, supplier, "v1").await,
        Err(wire::Error::Store(store::Error::NotFound))
    );
    assert_eq!(
        observe(pic, root, supplier, "shared")
            .await
            .unwrap()
            .manifest
            .hash,
        before.hash
    );
    assert!(
        !run(&dir, &["fleet", "artifact", "plan-restoration", &removal])
            .status
            .success()
    );
    expire(deadline).await;
    let restoration = cli(&dir, &["fleet", "artifact", "plan-restoration", &removal]);
    assert_eq!(restoration["plan"]["publication"]["after_removal"], removal);
    let restored = restoration["operation"].as_str().unwrap();
    assert_ne!(restored, original);
    apply(&dir, restored);
    assert_eq!(
        observe(pic, root, supplier, "v1").await.unwrap().manifest,
        before
    );
    // Even a fresh ingress replay of the old content-bound payload is expired.
    let request: wire::RemovalRequest = ic_wire::decode(
        &std::fs::read(path.join("arguments.bin")).unwrap(),
        ic_wire::Encoding::Frame {
            method: wire::REMOVE,
            direction: ic_wire::Direction::Request,
        },
        wire::REMOVAL_LIMITS,
    )
    .unwrap();
    let reply: wire::RemovalReply =
        update(pic, root, "store_removal", wire::REMOVE, &request).await;
    assert_eq!(reply, Err(wire::Error::Expired));
    assert_eq!(pic.get_stable_memory(root).await, root_stable);

    // Arrange a pin through Store's public API; actual refusal is driven by CLI -> Root -> Store.
    let pin = store::Pin {
        operation: [91; 32],
        manifest: observe(pic, root, supplier, "shared")
            .await
            .unwrap()
            .manifest,
    };
    let pinned = pic
        .update_call(
            supplier,
            root,
            "pin",
            ic_wire::encode(
                &pin,
                ic_wire::Encoding::Frame {
                    method: store::PIN,
                    direction: ic_wire::Direction::Request,
                },
                STORE_LIMITS,
            )
            .unwrap(),
        )
        .await
        .unwrap();
    let pinned: Result<store::PinReceipt, store::Error> = ic_wire::decode(
        &pinned,
        ic_wire::Encoding::Frame {
            method: store::PIN,
            direction: ic_wire::Direction::Response,
        },
        STORE_LIMITS,
    )
    .unwrap();
    pinned.unwrap();
    let protected = remove(&dir, &alias, "delete");
    let unchanged = pic.get_stable_memory(supplier).await;
    assert!(
        !run(
            &dir,
            &["fleet", "apply", &protected, "--identity", "anonymous"]
        )
        .status
        .success()
    );
    assert_eq!(pic.get_stable_memory(supplier).await, unchanged);
    assert!(
        !run(&dir, &["fleet", "artifact", "plan-restoration", &protected])
            .status
            .success()
    );

    for (version, point) in [("cancelled", 0), ("lost", 12)] {
        // Each incomplete upload has distinct valid bytes, so a published alias cannot satisfy it.
        let bytes = [std::fs::read(&wasm).unwrap(), vec![0, 2, 0, point]].concat();
        std::fs::write(&wasm, &bytes).unwrap();
        let publication = publication(&dir, version, &root_hash, &store_hash);
        let manifest = store::Manifest {
            release: store::Release {
                name: "app".into(),
                version: version.into(),
            },
            hash: store::digest(&bytes),
            bytes: bytes.len() as u64,
            chunks: bytes
                .chunks(store::CHUNK_BYTES)
                .map(store::digest)
                .collect(),
        };
        let prepared: wire::Reply = update(
            pic,
            root,
            "publish_store_artifact",
            wire::EXECUTE,
            &wire::Request {
                store: supplier.to_text(),
                expires_at_ns: pic.get_time().await.as_nanos_since_unix_epoch() + 240_000_000_000,
                command: wire::Command::Prepare(manifest.clone()),
            },
        )
        .await;
        assert!(!prepared.unwrap().published);
        let cancellation = remove(&dir, &publication, "cancel");
        let deadline = deadlines(&dir, &cancellation, &["removal"]);
        fault(pic, root, point).await;
        let outcome = run(
            &dir,
            &["fleet", "apply", &cancellation, "--identity", "anonymous"],
        );
        assert_eq!(
            outcome.status.success(),
            point == 0,
            "{}",
            String::from_utf8_lossy(&outcome.stderr)
        );
        fault(pic, root, 0).await;
        assert_eq!(
            observe(pic, root, supplier, version).await,
            Err(wire::Error::Store(store::Error::NotFound))
        );
        let path = dir.join("journal").join(&cancellation).join("removal");
        let signed = std::fs::read(path.join("signed.json")).unwrap();
        expire(deadline).await;
        if point == 0 {
            let restored = cli(
                &dir,
                &["fleet", "artifact", "plan-restoration", &cancellation],
            );
            apply(&dir, restored["operation"].as_str().unwrap());
            assert_eq!(
                observe(pic, root, supplier, version)
                    .await
                    .unwrap()
                    .manifest,
                manifest
            );
        } else {
            for args in [
                vec!["fleet", "artifact", "plan-restoration", &cancellation],
                vec![
                    "fleet",
                    "reconcile",
                    &cancellation,
                    "--identity",
                    "anonymous",
                ],
            ] {
                assert!(!run(&dir, &args).status.success());
            }
            assert!(!path.join("reply.bin").exists());
        }
        assert_eq!(std::fs::read(path.join("signed.json")).unwrap(), signed);
    }
    owner.client_mut().stop_live().await;
    std::fs::remove_dir_all(dir).unwrap();
    assert_eq!(
        owner.close(std::time::Duration::from_secs(10)).await,
        ic_testkit::server::Deletion::Attempted
    );
    let stopped = server.stop().unwrap();
    assert!(stopped.process.cleanup.is_empty(), "{stopped:?}");
}
