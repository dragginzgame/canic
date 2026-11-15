//! Store owns remote commits even when Root's reply callback traps.
use super::{Client, artifacts, custody, fixture, inject, maintain, padded, store, support};
use canic_management_protocol::publication::{self, Command, Error, Reply, Request};
use ic_testkit::pocket_ic::CanisterId;

async fn request(root: &Client<'_>, supplier: &Client<'_>, command: Command) -> Request {
    Request {
        store: supplier.id.to_text(),
        expires_at_ns: root.pic.get_time().await.as_nanos_since_unix_epoch() + 240_000_000_000,
        command,
    }
}
async fn send(root: &Client<'_>, input: &Request) -> Reply {
    let bytes = root
        .pic
        .update_call(
            root.id,
            root.operator,
            "store_publication",
            ic_wire::encode(
                input,
                ic_wire::Encoding::Frame {
                    method: publication::EXECUTE,
                    direction: ic_wire::Direction::Request,
                },
                publication::REQUEST_LIMITS,
            )
            .unwrap(),
        )
        .await
        .unwrap();
    ic_wire::decode(
        &bytes,
        ic_wire::Encoding::Frame {
            method: publication::EXECUTE,
            direction: ic_wire::Direction::Response,
        },
        publication::RESPONSE_LIMITS,
    )
    .unwrap()
}
fn manifest(version: &str, bytes: &[u8]) -> store::Manifest {
    store::Manifest {
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
    }
}

#[tokio::test(flavor = "current_thread")]
async fn publication_after_handover_resumes_across_root_maintenance_without_a_root_catalog() {
    let (_, binary) = support::inputs();
    let owner = support::instance(&binary).await;
    let pic = owner.instance();
    let (root, supplier, existing, _) = fixture(pic, false).await;
    let root_before = pic.get_stable_memory(root.id).await;
    let custody_before = custody(&root).await;
    let base = std::fs::read(artifacts().join("app.wasm")).unwrap();
    let custom_start = base.len() + 10;
    let mut wasm = padded(base);
    wasm[custom_start..].fill(43); // New valid custom-section bytes across several chunks.
    let manifest = manifest("after-handover", &wasm);
    let prepare = request(&root, &supplier, Command::Prepare(manifest.clone())).await;
    // The external operator cannot directly mutate Root's Store after handover.
    assert!(
        pic.update_call(
            supplier.id,
            root.operator,
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
    let first = send(&root, &prepare).await.unwrap();
    assert!(!first.published);
    assert!(!first.missing.is_empty());
    let mut conflict = manifest.clone();
    conflict.hash = [88; 32];
    assert_eq!(
        send(
            &root,
            &request(&root, &supplier, Command::Prepare(conflict)).await
        )
        .await,
        Err(Error::Store(store::Error::Conflict))
    );
    // Send one missing chunk, maintain Root, then resume from Store's retained projection.
    let first_index = first.missing[0] as usize;
    let chunks: Vec<_> = wasm.chunks(store::CHUNK_BYTES).collect();
    let put = request(
        &root,
        &supplier,
        Command::Put(store::Put {
            release: manifest.release.clone(),
            index: first_index as u32,
            bytes: chunks[first_index].to_vec(),
        }),
    )
    .await;
    let progress = send(&root, &put).await.unwrap();
    assert!(!progress.missing.contains(&(first_index as u32)));
    assert!(!progress.missing.is_empty());
    assert_eq!(pic.get_stable_memory(root.id).await, root_before);
    maintain(&root).await;
    let observe = request(&root, &supplier, Command::Observe(manifest.release.clone())).await;
    assert_eq!(send(&root, &observe).await.unwrap(), progress);
    // Replaying an exact command observes retained work, not another reservation.
    let supplier_before = pic.get_stable_memory(supplier.id).await;
    assert_eq!(send(&root, &prepare).await.unwrap(), progress);
    assert_eq!(send(&root, &put).await.unwrap(), progress);
    assert_eq!(pic.get_stable_memory(supplier.id).await, supplier_before);
    for index in progress.missing {
        send(
            &root,
            &request(
                &root,
                &supplier,
                Command::Put(store::Put {
                    release: manifest.release.clone(),
                    index,
                    bytes: chunks[index as usize].to_vec(),
                }),
            )
            .await,
        )
        .await
        .unwrap();
    }
    let publish = request(&root, &supplier, Command::Publish(manifest.release.clone())).await;
    let published = send(&root, &publish).await.unwrap();
    assert!(published.published && published.missing.is_empty());
    assert_eq!(published.manifest, manifest);
    let supplier_before = pic.get_stable_memory(supplier.id).await;
    assert_eq!(send(&root, &publish).await.unwrap(), published);
    assert_eq!(pic.get_stable_memory(supplier.id).await, supplier_before);
    let prior = send(
        &root,
        &request(
            &root,
            &supplier,
            Command::Observe(existing.reference.manifest.release),
        )
        .await,
    )
    .await
    .unwrap();
    assert!(prior.published); // A new version never replaces older release history.
    let custody_after = custody(&root).await;
    assert_eq!(custody_after.sources, custody_before.sources);
    assert!(
        custody_after
            .children
            .iter()
            .all(|c| c.creation.approval.is_none())
    );
    assert_eq!(pic.get_stable_memory(root.id).await, root_before);
    owner.close().await;
}

#[tokio::test(flavor = "current_thread")]
async fn publication_refusals_preserve_state_and_lost_replies_recover_from_store() {
    let (_, binary) = support::inputs();
    let owner = support::instance(&binary).await;
    let pic = owner.instance();
    let (root, supplier, _, _) = fixture(pic, true).await;
    let wasm = std::fs::read(artifacts().join("app.wasm")).unwrap();
    assert!(wasm.len() < store::CHUNK_BYTES);
    let manifest = manifest("callback-loss", &wasm);
    let prepare = request(&root, &supplier, Command::Prepare(manifest.clone())).await;
    let root_before = pic.get_stable_memory(root.id).await;
    let store_before = pic.get_stable_memory(supplier.id).await;
    let frame = ic_wire::encode(
        &prepare,
        ic_wire::Encoding::Frame {
            method: publication::EXECUTE,
            direction: ic_wire::Direction::Request,
        },
        publication::REQUEST_LIMITS,
    )
    .unwrap();
    assert!(
        pic.update_call(
            root.id,
            CanisterId::anonymous(),
            "store_publication",
            frame.clone()
        )
        .await
        .is_err()
    );
    let mut trailing = frame.clone();
    trailing.push(0);
    let wrong = ic_wire::encode(
        &prepare,
        ic_wire::Encoding::Frame {
            method: publication::EXECUTE + 1,
            direction: ic_wire::Direction::Request,
        },
        publication::REQUEST_LIMITS,
    )
    .unwrap();
    let response = ic_wire::encode(
        &prepare,
        ic_wire::Encoding::Frame {
            method: publication::EXECUTE,
            direction: ic_wire::Direction::Response,
        },
        publication::REQUEST_LIMITS,
    )
    .unwrap();
    for bytes in [
        vec![],
        trailing,
        wrong,
        response,
        frame[..frame.len() - 1].to_vec(),
        vec![0; publication::REQUEST_LIMITS.bytes + 1],
    ] {
        assert!(
            pic.update_call(root.id, root.operator, "store_publication", bytes)
                .await
                .is_err()
        );
    }
    let mut invalid = prepare.clone();
    invalid.expires_at_ns = pic.get_time().await.as_nanos_since_unix_epoch();
    assert_eq!(send(&root, &invalid).await, Err(Error::Expired));
    invalid.expires_at_ns += publication::MAX_WINDOW_NS * 2;
    assert_eq!(send(&root, &invalid).await, Err(Error::Invalid));
    for target in [
        root.id.to_text(),
        CanisterId::anonymous().to_text(),
        "aaaaa-aa".into(),
        "invalid".into(),
    ] {
        let mut invalid = prepare.clone();
        invalid.store = target;
        assert_eq!(send(&root, &invalid).await, Err(Error::Invalid));
    }
    assert_eq!(pic.get_stable_memory(root.id).await, root_before);
    assert_eq!(pic.get_stable_memory(supplier.id).await, store_before);
    // Each real encoder failure happens after the remote Store's commit. Root's
    // callback rollback must not be mistaken for no effect at the Store.
    for command in [
        Command::Prepare(manifest.clone()),
        Command::Put(store::Put {
            release: manifest.release.clone(),
            index: 0,
            bytes: wasm,
        }),
        Command::Publish(manifest.release.clone()),
    ] {
        let input = request(&root, &supplier, command).await;
        inject(&root, 12).await;
        assert!(
            pic.update_call(
                root.id,
                root.operator,
                "store_publication",
                ic_wire::encode(
                    &input,
                    ic_wire::Encoding::Frame {
                        method: publication::EXECUTE,
                        direction: ic_wire::Direction::Request
                    },
                    publication::REQUEST_LIMITS
                )
                .unwrap()
            )
            .await
            .is_err()
        );
        inject(&root, 0).await;
        let retained = send(
            &root,
            &request(&root, &supplier, Command::Observe(manifest.release.clone())).await,
        )
        .await
        .unwrap();
        let store_before = pic.get_stable_memory(supplier.id).await;
        assert_eq!(send(&root, &input).await.unwrap(), retained);
        assert_eq!(pic.get_stable_memory(supplier.id).await, store_before);
        assert_eq!(pic.get_stable_memory(root.id).await, root_before);
    }
    assert!(
        send(
            &root,
            &request(&root, &supplier, Command::Observe(manifest.release)).await
        )
        .await
        .unwrap()
        .published
    );
    owner.close().await;
}
